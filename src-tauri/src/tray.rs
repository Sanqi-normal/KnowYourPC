use tauri::{
    AppHandle, Emitter, Manager,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
};

pub fn build_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let icon = app
        .default_window_icon()
        .ok_or("no default window icon")?
        .clone();

    let open_item = MenuItemBuilder::with_id("open", "打开窗口").build(app)?;
    let mcp_item = MenuItemBuilder::with_id("mcp_toggle", "启动 MCP 服务").build(app)?;
    let quit_item = MenuItemBuilder::with_id("quit", "退出").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&open_item)
        .separator()
        .item(&mcp_item)
        .separator()
        .item(&quit_item)
        .build()?;

    TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .tooltip("KnowYourDisk")
        .on_menu_event(|app, event| {
            match event.id().as_ref() {
                "open" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                        let _ = window.unminimize();
                    }
                }
                "mcp_toggle" => {
                    let _ = app.emit("tray-mcp-toggle", ());
                }
                "quit" => {
                    crate::commands::kill_mcp_processes(app);
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, .. } = event {
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                    let _ = window.unminimize();
                }
            }
        })
        .build(app)?;

    let state = app.state::<crate::McpState>();
    *state.mcp_item.lock().unwrap() = Some(mcp_item);

    Ok(())
}

pub fn set_mcp_menu_text(app: &AppHandle, running: bool) {
    let text = if running {
        "停止 MCP 服务"
    } else {
        "启动 MCP 服务"
    };
    let item = {
        let state = app.state::<crate::McpState>();
        let guard = state.mcp_item.lock().unwrap();
        guard.clone()
    };
    if let Some(item) = item {
        let _ = item.set_text(text);
    }
}
