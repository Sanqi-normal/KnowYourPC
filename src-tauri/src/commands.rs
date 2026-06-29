use crate::models::{ScanOptions, ScanResult, VolumeInfo};

#[tauri::command]
pub async fn list_volumes() -> Result<Vec<VolumeInfo>, String> {
    crate::win::volume::list_volumes().map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn scan(
    app: tauri::AppHandle,
    options: ScanOptions,
) -> Result<ScanResult, String> {
    tauri::async_runtime::spawn_blocking(move || crate::scanner::scan(&app, options))
        .await
        .map_err(|error| format!("扫描线程失败: {error}"))?
        .map_err(|error| error.to_string())
}
