use std::collections::HashMap;

use crate::error::AppResult;
use crate::models::{ExtensionStat, ScanOptions, ScanResult, VolumeInfo};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn list_volumes() -> Result<Vec<VolumeInfo>, String> {
    crate::win::volume::list_volumes().map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn scan(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    options: ScanOptions,
) -> Result<ScanResult, String> {
    let result = tauri::async_runtime::spawn_blocking(move || crate::scanner::scan(&app, options))
        .await
        .map_err(|error| format!("扫描线程失败: {error}"))?
        .map_err(|error| error.to_string())?;

    *state.scan.lock().unwrap() = Some(result.clone());
    Ok(result)
}

#[tauri::command]
pub fn open_in_explorer(path: String) -> AppResult<()> {
    let parent_path = std::path::Path::new(&path)
        .parent()
        .unwrap_or(std::path::Path::new(&path));
    tauri_plugin_opener::open_path(parent_path, None::<&str>)
        .map_err(|e| crate::error::AppError::Win(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub fn is_admin() -> bool {
    crate::win::elevation::is_elevated()
}

#[tauri::command]
pub fn restart_as_admin() -> Result<(), String> {
    crate::win::elevation::restart_elevated()?;
    std::process::exit(0);
}

#[tauri::command]
pub fn get_extension_stats(state: State<'_, AppState>) -> AppResult<Vec<ExtensionStat>> {
    let guard = state.scan.lock().unwrap();
    let scan = guard
        .as_ref()
        .ok_or_else(|| crate::error::AppError::Internal("尚未扫描".into()))?;

    let mut ext_map: HashMap<String, ExtensionStat> = HashMap::new();

    for node in &scan.nodes {
        if node.is_dir {
            continue;
        }
        let ext = node
            .extension
            .as_deref()
            .filter(|e| !e.is_empty())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_else(|| "(无扩展名)".to_string());

        let entry = ext_map.entry(ext.clone()).or_insert(ExtensionStat {
            extension: ext,
            size: 0,
            allocated: 0,
            file_count: 0,
        });
        entry.size = entry.size.saturating_add(node.total_size);
        entry.allocated = entry.allocated.saturating_add(node.total_allocated);
        entry.file_count += 1;
    }

    let mut stats: Vec<ExtensionStat> = ext_map.into_values().collect();
    stats.sort_by(|a, b| b.allocated.cmp(&a.allocated));

    if stats.len() > 100 {
        stats.truncate(100);
    }

    Ok(stats)
}
