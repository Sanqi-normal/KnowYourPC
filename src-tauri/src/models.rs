use serde::{Deserialize, Serialize};

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
