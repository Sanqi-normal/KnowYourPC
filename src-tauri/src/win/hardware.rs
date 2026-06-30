use serde::Deserialize;
use wmi::{COMLibrary, WMIConnection};

use crate::models::*;

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct Win32Processor {
    Name: Option<String>,
    NumberOfCores: Option<u32>,
    NumberOfLogicalProcessors: Option<u32>,
    MaxClockSpeed: Option<u32>,
    Architecture: Option<u16>,
    L2CacheSize: Option<u32>,
    L3CacheSize: Option<u32>,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct Win32PhysicalMemory {
    DeviceLocator: Option<String>,
    Capacity: Option<u64>,
    MemoryType: Option<u16>,
    Speed: Option<u32>,
    Manufacturer: Option<String>,
    PartNumber: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct Win32VideoController {
    Name: Option<String>,
    AdapterRAM: Option<u64>,
    DriverVersion: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct Win32BaseBoard {
    Manufacturer: Option<String>,
    Product: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct Win32Bios {
    Manufacturer: Option<String>,
    SMBIOSBIOSVersion: Option<String>,
    ReleaseDate: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct Win32Battery {
    DesignCapacity: Option<u32>,
    FullChargeCapacity: Option<u32>,
    CycleCount: Option<u32>,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct Win32CacheMemory {
    Level: Option<u32>,
    MaxCacheSize: Option<u32>,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct Win32ComputerSystem {
    TotalPhysicalMemory: Option<u64>,
}

fn new_wmi() -> Result<WMIConnection, String> {
    match COMLibrary::new() {
        Ok(com) => WMIConnection::new(com).map_err(|e| format!("WMI 连接失败: {e}")),
        Err(_) => {
            // COMLibrary::new() uses COINIT_MULTITHREADED. If the thread already
            // has COM initialized with a different mode (e.g. COINIT_APARTMENTTHREADED),
            // it fails with RPC_E_CHANGED_MODE (0x80010106). Try STA as fallback.
            unsafe {
                let _ = windows_sys::Win32::System::Com::CoInitializeEx(
                    std::ptr::null(),
                    windows_sys::Win32::System::Com::COINIT_APARTMENTTHREADED as u32,
                );
            }
            let com = unsafe { COMLibrary::assume_initialized() };
            WMIConnection::new(com).map_err(|e| format!("WMI 连接失败: {e}"))
        }
    }
}

fn query<T>(wmi: &WMIConnection, wql: &str) -> Vec<T>
where
    T: serde::de::DeserializeOwned,
{
    wmi.raw_query(wql).unwrap_or_default()
}

fn arch_to_string(arch: u16) -> &'static str {
    match arch {
        0 => "x86",
        1 => "MIPS",
        2 => "Alpha",
        3 => "PowerPC",
        5 => "ARM",
        6 => "Itanium",
        9 => "x64",
        12 => "ARM64",
        _ => "未知",
    }
}

fn mem_type_to_string(mt: u16) -> &'static str {
    match mt {
        20 => "DDR",
        21 => "DDR2",
        24 => "DDR3",
        26 => "DDR4",
        34 => "DDR5",
        _ => "Unknown",
    }
}

fn extract_date(date_str: &str) -> String {
    if date_str.is_empty() {
        return "未知".into();
    }
    let parts: Vec<&str> = date_str.split(|c| c == '/' || c == '-' || c == ' ').collect();
    if parts.len() >= 3 {
        format!("{}-{}-{}", parts[0], parts[1], parts[2])
    } else {
        date_str.to_string()
    }
}

pub fn get_hardware_info() -> Result<HardwareInfo, String> {
    let wmi = new_wmi()?;

    // CPU
    let cpus: Vec<Win32Processor> = query(
        &wmi,
        "SELECT Name,NumberOfCores,NumberOfLogicalProcessors,MaxClockSpeed,Architecture,L2CacheSize,L3CacheSize FROM Win32_Processor",
    );
    let cpu_raw = cpus.into_iter().next().unwrap_or(Win32Processor {
        Name: Some("未知".into()),
        NumberOfCores: Some(0),
        NumberOfLogicalProcessors: Some(0),
        MaxClockSpeed: Some(0),
        Architecture: Some(9),
        L2CacheSize: None,
        L3CacheSize: None,
    });

    // Cache
    let caches: Vec<Win32CacheMemory> = query(
        &wmi,
        "SELECT Level,MaxCacheSize FROM Win32_CacheMemory",
    );
    let l1_cache: u32 = caches
        .iter()
        .filter(|x| x.Level == Some(1))
        .filter_map(|x| x.MaxCacheSize)
        .sum();

    let cpu = CpuInfo {
        name: cpu_raw.Name.unwrap_or_else(|| "未知".into()),
        architecture: arch_to_string(cpu_raw.Architecture.unwrap_or(9)).into(),
        physical_cores: cpu_raw.NumberOfCores.unwrap_or(0),
        logical_threads: cpu_raw.NumberOfLogicalProcessors.unwrap_or(0),
        frequency_mhz: cpu_raw.MaxClockSpeed.unwrap_or(0),
        l1_cache_kb: if l1_cache > 0 { Some(l1_cache) } else { None },
        l2_cache_kb: cpu_raw.L2CacheSize,
        l3_cache_kb: cpu_raw.L3CacheSize,
    };

    // Total RAM
    let sys_info: Vec<Win32ComputerSystem> = query(
        &wmi,
        "SELECT TotalPhysicalMemory FROM Win32_ComputerSystem",
    );
    let total_ram = sys_info
        .into_iter()
        .next()
        .and_then(|s| s.TotalPhysicalMemory)
        .unwrap_or(0);
    let total_ram_gb = total_ram as f64 / 1_073_741_824.0;

    // RAM slots
    let mem_slots: Vec<Win32PhysicalMemory> = query(
        &wmi,
        "SELECT DeviceLocator,Capacity,MemoryType,Speed,Manufacturer,PartNumber FROM Win32_PhysicalMemory",
    );
    let slots: Vec<RamSlot> = mem_slots
        .into_iter()
        .map(|s| RamSlot {
            slot: s.DeviceLocator.unwrap_or_else(|| "未知".into()),
            capacity_gb: s.Capacity.unwrap_or(0) as f64 / 1_073_741_824.0,
            memory_type: mem_type_to_string(s.MemoryType.unwrap_or(0)).into(),
            speed_mhz: s.Speed.unwrap_or(0),
            manufacturer: s.Manufacturer.unwrap_or_else(|| "未知".into()),
            part_number: s.PartNumber.unwrap_or_else(|| "".into()),
        })
        .collect();

    let ram = RamInfo {
        total_gb: if total_ram_gb > 0.0 {
            total_ram_gb
        } else {
            slots.iter().map(|s| s.capacity_gb).sum()
        },
        slots,
    };

    // GPU
    let gpus_raw: Vec<Win32VideoController> = query(
        &wmi,
        "SELECT Name,AdapterRAM,DriverVersion FROM Win32_VideoController",
    );
    let gpus: Vec<GpuInfo> = gpus_raw
        .into_iter()
        .map(|g| GpuInfo {
            name: g.Name.unwrap_or_else(|| "未知".into()),
            vram_mb: g.AdapterRAM.unwrap_or(0) / 1_048_576,
            driver_version: g.DriverVersion.unwrap_or_else(|| "未知".into()),
        })
        .collect();

    // Motherboard
    let mb_raw: Vec<Win32BaseBoard> = query(
        &wmi,
        "SELECT Manufacturer,Product FROM Win32_BaseBoard",
    );
    let mb = mb_raw.into_iter().next().unwrap_or(Win32BaseBoard {
        Manufacturer: Some("未知".into()),
        Product: Some("未知".into()),
    });
    let motherboard = MotherboardInfo {
        manufacturer: mb.Manufacturer.unwrap_or_else(|| "未知".into()),
        product: mb.Product.unwrap_or_else(|| "未知".into()),
    };

    // BIOS
    let bios_raw: Vec<Win32Bios> = query(
        &wmi,
        "SELECT Manufacturer,SMBIOSBIOSVersion,ReleaseDate FROM Win32_BIOS",
    );
    let bios = bios_raw.into_iter().next().unwrap_or(Win32Bios {
        Manufacturer: Some("未知".into()),
        SMBIOSBIOSVersion: Some("未知".into()),
        ReleaseDate: Some("".into()),
    });
    let bios = BiosInfo {
        manufacturer: bios.Manufacturer.unwrap_or_else(|| "未知".into()),
        version: bios.SMBIOSBIOSVersion.unwrap_or_else(|| "未知".into()),
        release_date: extract_date(&bios.ReleaseDate.unwrap_or_default()),
    };

    // Battery
    let bat_raw: Vec<Win32Battery> = query(
        &wmi,
        "SELECT DesignCapacity,FullChargeCapacity,CycleCount FROM Win32_Battery",
    );
    let battery = if let Some(b) = bat_raw.into_iter().next() {
        let design = b.DesignCapacity;
        let full = b.FullChargeCapacity;
        let health = match (design, full) {
            (Some(d), Some(f)) if d > 0 => (f as f64 / d as f64) * 100.0,
            _ => 0.0,
        };
        BatteryInfo {
            present: true,
            design_capacity_mwh: design,
            full_charge_capacity_mwh: full,
            cycle_count: b.CycleCount,
            health_percent: health,
        }
    } else {
        BatteryInfo {
            present: false,
            design_capacity_mwh: None,
            full_charge_capacity_mwh: None,
            cycle_count: None,
            health_percent: 0.0,
        }
    };

    Ok(HardwareInfo {
        cpu,
        ram,
        gpus,
        motherboard,
        bios,
        battery,
    })
}
