use std::sync::Mutex;

pub mod commands;
pub mod error;
pub mod models;
pub mod scanner;
pub mod tray;
pub mod win;

use models::NodeDto;

use tauri::menu::MenuItem;
use tauri::Emitter;

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
    pub mcp_item: Mutex<Option<MenuItem<tauri::Wry>>>,
}

impl McpState {
    pub fn new() -> Self {
        Self {
            child: Mutex::new(None),
            port: Mutex::new(0),
            mcp_item: Mutex::new(None),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .manage(McpState::new())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.emit("show-close-dialog", ());
            }
        })
        .setup(|app| {
            crate::tray::build_tray(app.handle())?;
            Ok(())
        })
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
            commands::hide_main_window,
            commands::quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
