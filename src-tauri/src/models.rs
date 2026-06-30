use serde::{Deserialize, Serialize};

// ── Hardware Info Types ──

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuInfo {
    pub name: String,
    pub architecture: String,
    pub physical_cores: u32,
    pub logical_threads: u32,
    pub frequency_mhz: u32,
    pub l1_cache_kb: Option<u32>,
    pub l2_cache_kb: Option<u32>,
    pub l3_cache_kb: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RamSlot {
    pub slot: String,
    pub capacity_gb: f64,
    pub memory_type: String,
    pub speed_mhz: u32,
    pub manufacturer: String,
    pub part_number: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RamInfo {
    pub total_gb: f64,
    pub slots: Vec<RamSlot>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    pub name: String,
    pub vram_mb: u64,
    pub driver_version: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MotherboardInfo {
    pub manufacturer: String,
    pub product: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BiosInfo {
    pub manufacturer: String,
    pub version: String,
    pub release_date: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatteryInfo {
    pub present: bool,
    pub design_capacity_mwh: Option<u32>,
    pub full_charge_capacity_mwh: Option<u32>,
    pub cycle_count: Option<u32>,
    pub health_percent: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareInfo {
    pub cpu: CpuInfo,
    pub ram: RamInfo,
    pub gpus: Vec<GpuInfo>,
    pub motherboard: MotherboardInfo,
    pub bios: BiosInfo,
    pub battery: BatteryInfo,
}

// ── Performance Types ──

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_mb: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerfSnapshot {
    pub cpu_percent: f32,
    pub gpu_percent: f32,
    pub memory_used_gb: f64,
    pub memory_total_gb: f64,
    pub memory_percent: f32,
    pub disk_read_mbps: f64,
    pub disk_write_mbps: f64,
    pub net_recv_kbps: f64,
    pub net_sent_kbps: f64,
    pub top_processes: Vec<ProcessInfo>,
}

// ── Existing Types ──

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeInfo {
    pub root: String,
    pub display_name: String,
    pub fs_name: String,
    pub drive_type: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub cluster_size: u64,
    pub ntfs_candidate: bool,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScanMode {
    Auto,
    NtfsMft,
    Walk,
}

impl Default for ScanMode {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanOptions {
    pub root: String,
    #[serde(default)]
    pub mode: ScanMode,
    #[serde(default)]
    pub include_system_files: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeDto {
    pub id: u32,
    pub parent: Option<u32>,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub allocated: u64,
    pub total_size: u64,
    pub total_allocated: u64,
    pub child_count: u32,
    pub children: Vec<u32>,
    pub file_count: u64,
    pub dir_count: u64,
    pub extension: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildNode {
    pub id: u32,
    pub parent: Option<u32>,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub allocated: u64,
    pub total_size: u64,
    pub total_allocated: u64,
    pub child_count: u32,
    pub file_count: u64,
    pub dir_count: u64,
    pub extension: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub id: u32,
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub allocated: u64,
    pub total_size: u64,
    pub total_allocated: u64,
    pub extension: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreemapNode {
    pub id: u32,
    pub size: u64,
    pub name: String,
    pub is_dir: bool,
    pub extension: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<TreemapNode>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub root: String,
    pub scanner: String,
    pub elapsed_ms: u64,
    pub node_count: u64,
    pub file_count: u64,
    pub dir_count: u64,
    pub total_size: u64,
    pub total_allocated: u64,
    pub nodes: Vec<NodeDto>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub phase: String,
    pub processed: u64,
    pub total: Option<u64>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionStat {
    pub extension: String,
    pub size: u64,
    pub allocated: u64,
    pub file_count: u64,
}
