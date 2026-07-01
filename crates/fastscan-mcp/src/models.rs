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
    fn default() -> Self { Self::Auto }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanOptions {
    pub root: String,
    #[serde(default)]
    pub mode: ScanMode,
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
    pub created: Option<i64>,
    pub modified: Option<i64>,
    pub accessed: Option<i64>,
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
    pub created: Option<i64>,
    pub modified: Option<i64>,
    pub accessed: Option<i64>,
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
    pub created: Option<i64>,
    pub modified: Option<i64>,
    pub accessed: Option<i64>,
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
pub struct ExtensionStat {
    pub extension: String,
    pub size: u64,
    pub allocated: u64,
    pub file_count: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub id: u32,
    pub name: String,
    pub path: String,
    pub size: u64,
    pub allocated: u64,
    pub extension: Option<String>,
    pub created: Option<i64>,
    pub modified: Option<i64>,
    pub accessed: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirInfo {
    pub id: u32,
    pub name: String,
    pub path: String,
    pub total_size: u64,
    pub total_allocated: u64,
    pub file_count: u64,
    pub dir_count: u64,
    pub child_count: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    pub file_name: String,
    pub size: u64,
    pub file_count: usize,
    pub total_size: u64,
    pub files: Vec<FileInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStatus {
    pub elapsed_ms: u64,
    pub node_count: u64,
    pub file_count: u64,
    pub dir_count: u64,
    pub total_size: u64,
    pub total_allocated: u64,
    pub scanner: String,
    pub warnings: Vec<String>,
}
