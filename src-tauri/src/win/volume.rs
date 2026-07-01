use crate::models::VolumeInfo;

#[cfg(windows)]
use std::{ffi::OsStr, os::windows::ffi::OsStrExt};

#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{
    GetDiskFreeSpaceExW, GetDiskFreeSpaceW, GetDriveTypeW, GetVolumeInformationW,
};

#[cfg(windows)]
const DRIVE_UNKNOWN: u32 = 0;
#[cfg(windows)]
const DRIVE_NO_ROOT_DIR: u32 = 1;
#[cfg(windows)]
const DRIVE_REMOVABLE: u32 = 2;
#[cfg(windows)]
const DRIVE_FIXED: u32 = 3;
#[cfg(windows)]
const DRIVE_REMOTE: u32 = 4;
#[cfg(windows)]
const DRIVE_CDROM: u32 = 5;
#[cfg(windows)]
const DRIVE_RAMDISK: u32 = 6;

pub fn list_volumes() -> anyhow::Result<Vec<VolumeInfo>> {
    #[cfg(windows)]
    {
        list_windows_volumes()
    }

    #[cfg(not(windows))]
    {
        list_non_windows_volumes()
    }
}

pub fn cluster_size_for_root(root: &str) -> u64 {
    #[cfg(windows)]
    {
        let root = normalize_windows_root(root);
        let root_w = wide_null(&root);

        let mut sectors_per_cluster = 0u32;
        let mut bytes_per_sector = 0u32;
        let mut free_clusters = 0u32;
        let mut total_clusters = 0u32;

        let ok = unsafe {
            GetDiskFreeSpaceW(
                root_w.as_ptr(),
                &mut sectors_per_cluster,
                &mut bytes_per_sector,
                &mut free_clusters,
                &mut total_clusters,
            )
        };

        if ok != 0 {
            let cluster = sectors_per_cluster as u64 * bytes_per_sector as u64;
            if cluster > 0 {
                return cluster;
            }
        }

        4096
    }

    #[cfg(not(windows))]
    {
        let _ = root;
        4096
    }
}

#[cfg(windows)]
fn list_windows_volumes() -> anyhow::Result<Vec<VolumeInfo>> {
    let mut volumes = Vec::new();

    for letter in b'A'..=b'Z' {
        let root = format!("{}:\\", letter as char);
        let root_w = wide_null(&root);

        let drive_type_raw = unsafe { GetDriveTypeW(root_w.as_ptr()) };

        if drive_type_raw == DRIVE_NO_ROOT_DIR || drive_type_raw == DRIVE_UNKNOWN {
            continue;
        }

        let mut total_bytes = 0u64;
        let mut available_bytes = 0u64;
        let mut free_bytes = 0u64;

        unsafe {
            GetDiskFreeSpaceExW(
                root_w.as_ptr(),
                &mut available_bytes,
                &mut total_bytes,
                &mut free_bytes,
            );
        }

        let mut label_buf = [0u16; 260];
        let mut fs_buf = [0u16; 64];
        let mut serial = 0u32;
        let mut max_component_len = 0u32;
        let mut fs_flags = 0u32;

        let volume_info_ok = unsafe {
            GetVolumeInformationW(
                root_w.as_ptr(),
                label_buf.as_mut_ptr(),
                label_buf.len() as u32,
                &mut serial,
                &mut max_component_len,
                &mut fs_flags,
                fs_buf.as_mut_ptr(),
                fs_buf.len() as u32,
            )
        };

        let label = if volume_info_ok != 0 {
            utf16z_to_string(&label_buf)
        } else {
            String::new()
        };

        let fs_name = if volume_info_ok != 0 {
            utf16z_to_string(&fs_buf)
        } else {
            String::new()
        };

        let cluster_size = cluster_size_for_root(&root);
        let drive_type = drive_type_name(drive_type_raw).to_string();
        let ntfs_candidate = fs_name.eq_ignore_ascii_case("NTFS");

        let display_name = if label.is_empty() {
            root.clone()
        } else {
            format!("{} ({})", label, root.trim_end_matches('\\'))
        };

        volumes.push(VolumeInfo {
            root,
            display_name,
            fs_name,
            drive_type,
            total_bytes,
            available_bytes,
            cluster_size,
            ntfs_candidate,
        });
    }

    Ok(volumes)
}

#[cfg(not(windows))]
fn list_non_windows_volumes() -> anyhow::Result<Vec<VolumeInfo>> {
    let root = "/".to_string();
    let total_bytes = fs2::total_space(&root).unwrap_or(0);
    let available_bytes = fs2::available_space(&root).unwrap_or(0);

    Ok(vec![VolumeInfo {
        root: root.clone(),
        display_name: root,
        fs_name: String::new(),
        drive_type: "Root".to_string(),
        total_bytes,
        available_bytes,
        cluster_size: 4096,
        ntfs_candidate: false,
    }])
}

#[cfg(windows)]
fn normalize_windows_root(root: &str) -> String {
    let trimmed = root.trim();

    if trimmed.len() >= 2 && trimmed.as_bytes()[1] == b':' {
        let drive = trimmed.chars().next().unwrap().to_ascii_uppercase();
        return format!("{drive}:\\");
    }

    trimmed.to_string()
}

#[cfg(windows)]
fn drive_type_name(raw: u32) -> &'static str {
    if raw == DRIVE_FIXED {
        "Fixed"
    } else if raw == DRIVE_REMOVABLE {
        "Removable"
    } else if raw == DRIVE_REMOTE {
        "Network"
    } else if raw == DRIVE_CDROM {
        "CdRom"
    } else if raw == DRIVE_RAMDISK {
        "RamDisk"
    } else {
        "Unknown"
    }
}

#[cfg(windows)]
fn wide_null(text: &str) -> Vec<u16> {
    OsStr::new(text)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(windows)]
fn utf16z_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|ch| *ch == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}


