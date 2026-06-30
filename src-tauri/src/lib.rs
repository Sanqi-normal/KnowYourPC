use std::sync::Mutex;

pub mod commands;
pub mod error;
pub mod models;
pub mod scanner;
pub mod win;

use models::NodeDto;

pub struct AppState {
    pub tree: Mutex<Option<Vec<NodeDto>>>,
    pub root_path: Mutex<Option<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            tree: Mutex::new(None),
            root_path: Mutex::new(None),
        }
    }
}

pub struct McpState {
    pub child: Mutex<Option<std::process::Child>>,
    pub port: Mutex<u16>,
}

impl McpState {
    pub fn new() -> Self {
        Self { child: Mutex::new(None), port: Mutex::new(0) }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .manage(McpState::new())
        .invoke_handler(tauri::generate_handler![
            commands::list_volumes,
            commands::scan,
            commands::open_in_explorer,
            commands::get_extension_stats,
            commands::is_admin,
            commands::restart_as_admin,
            commands::get_children,
            commands::get_node_path,
            commands::get_treemap_data,
            commands::get_node_with_ancestors,
            commands::search_files,
            commands::start_mcp_server,
            commands::stop_mcp_server,
            commands::get_mcp_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
