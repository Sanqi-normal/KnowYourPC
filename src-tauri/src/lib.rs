pub mod commands;
pub mod models;
pub mod scanner;
pub mod win;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::list_volumes,
            commands::scan
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
