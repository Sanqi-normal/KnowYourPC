use std::sync::Mutex;

pub mod commands;
pub mod error;
pub mod models;
pub mod scanner;
pub mod win;

use models::ScanResult;

pub struct AppState {
    pub scan: Mutex<Option<ScanResult>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            scan: Mutex::new(None),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::list_volumes,
            commands::scan,
            commands::open_in_explorer,
            commands::get_extension_stats,
            commands::is_admin,
            commands::restart_as_admin,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
