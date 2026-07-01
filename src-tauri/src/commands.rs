use std::collections::HashMap;
use std::net::TcpListener;
use std::path::PathBuf;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::error::AppResult;
use crate::models::{ChildNode, ExtensionStat, NodeDto, ScanOptions, ScanResult, SearchResult, TreemapNode, VolumeInfo};
use crate::AppState;
use crate::McpState;
use tauri::State;
use tauri::Manager;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpStatus {
    pub running: bool,
    pub port: u16,
}

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
    let mut result = tauri::async_runtime::spawn_blocking(move || crate::scanner::scan(&app, options))
        .await
        .map_err(|error| format!("扫描线程失败: {error}"))?
        .map_err(|error| error.to_string())?;

    let nodes = std::mem::take(&mut result.nodes);
    *state.tree.lock().unwrap() = Some(nodes);
    *state.root_path.lock().unwrap() = Some(result.root.clone());

    Ok(result)
}

#[tauri::command]
pub fn get_children(
    parent_id: u32,
    state: State<'_, AppState>,
) -> AppResult<Vec<ChildNode>> {
    let guard = state.tree.lock().unwrap();
    let nodes = guard
        .as_ref()
        .ok_or_else(|| crate::error::AppError::Internal("尚未扫描".into()))?;

    let parent = nodes.get(parent_id as usize)
        .ok_or_else(|| crate::error::AppError::Internal("节点不存在".into()))?;

    let children: Vec<ChildNode> = parent
        .children
        .iter()
        .filter_map(|cid| {
            let child = nodes.get(*cid as usize)?;
            Some(ChildNode {
                id: child.id,
                parent: child.parent,
                name: child.name.clone(),
                is_dir: child.is_dir,
                size: child.size,
                allocated: child.allocated,
                total_size: child.total_size,
                total_allocated: child.total_allocated,
                child_count: child.children.len() as u32,
                file_count: child.file_count,
                dir_count: child.dir_count,
                extension: child.extension.clone(),
            })
        })
        .collect();

    Ok(children)
}

#[tauri::command]
pub fn get_node_path(
    node_id: u32,
    state: State<'_, AppState>,
) -> AppResult<String> {
    let guard = state.tree.lock().unwrap();
    let nodes = guard
        .as_ref()
        .ok_or_else(|| crate::error::AppError::Internal("尚未扫描".into()))?;
    let root = state.root_path.lock().unwrap();
    let root_str = root.as_deref().unwrap_or("");

    let mut parts: Vec<&str> = Vec::new();
    let mut current = Some(node_id);

    while let Some(id) = current {
        if let Some(node) = nodes.get(id as usize) {
            parts.push(&node.name);
            current = node.parent;
        } else {
            break;
        }
    }

    parts.reverse();
    if parts.is_empty() {
        return Ok(root_str.to_string());
    }

    let mut path = parts[0].to_string();
    for part in &parts[1..] {
        if path.ends_with('\\') || path.ends_with('/') {
            path.push_str(part);
        } else {
            path.push('\\');
            path.push_str(part);
        }
    }

    Ok(path)
}

#[tauri::command]
pub fn get_treemap_data(
    root_id: u32,
    max_items: u32,
    state: State<'_, AppState>,
) -> AppResult<Vec<TreemapNode>> {
    let guard = state.tree.lock().unwrap();
    let nodes = guard
        .as_ref()
        .ok_or_else(|| crate::error::AppError::Internal("尚未扫描".into()))?;

    let root = nodes.get(root_id as usize)
        .ok_or_else(|| crate::error::AppError::Internal("根节点不存在".into()))?;

    let total_size: u64 = root.children
        .iter()
        .filter_map(|id| nodes.get(*id as usize))
        .map(|n| n.total_allocated)
        .sum();

    let mut items: Vec<TreemapNode> = Vec::new();

    for &child_id in &root.children {
        if items.len() >= 10000 {
            break;
        }

        let node = match nodes.get(child_id as usize) {
            Some(n) if n.total_allocated > 0 => n,
            _ => continue,
        };

        let budget = if total_size > 0 {
            ((node.total_allocated as f64 / total_size as f64) * max_items as f64)
                .max(1.0) as u32
        } else {
            1
        };

        let mut child_remaining = budget;
        if let Some(child) = build_treemap_node(child_id, nodes, &mut child_remaining) {
            items.push(child);
        }
    }

    items.sort_by(|a, b| b.size.cmp(&a.size));
    Ok(items)
}

fn build_treemap_node(
    id: u32,
    nodes: &[NodeDto],
    remaining: &mut u32,
) -> Option<TreemapNode> {
    let node = nodes.get(id as usize)?;
    if node.total_allocated == 0 {
        return None;
    }

    if node.is_dir && !node.children.is_empty() {
        let mut children: Vec<TreemapNode> = Vec::new();
        for &child_id in &node.children {
            if *remaining == 0 {
                break;
            }
            if let Some(child) = build_treemap_node(child_id, nodes, remaining) {
                children.push(child);
            }
        }
        children.sort_by(|a, b| b.size.cmp(&a.size));

        *remaining = remaining.saturating_sub(1);

        Some(TreemapNode {
            id: node.id,
            size: node.total_allocated,
            name: node.name.clone(),
            is_dir: true,
            extension: None,
            children,
        })
    } else {
        if *remaining == 0 {
            return None;
        }
        *remaining -= 1;

        Some(TreemapNode {
            id: node.id,
            size: node.total_allocated,
            name: node.name.clone(),
            is_dir: node.is_dir,
            extension: node.extension.clone(),
            children: Vec::new(),
        })
    }
}

#[tauri::command]
pub fn get_node_with_ancestors(
    node_id: u32,
    state: State<'_, AppState>,
) -> AppResult<Vec<ChildNode>> {
    let guard = state.tree.lock().unwrap();
    let nodes = guard
        .as_ref()
        .ok_or_else(|| crate::error::AppError::Internal("尚未扫描".into()))?;

    let mut result: Vec<ChildNode> = Vec::new();
    let mut current = Some(node_id);

    while let Some(id) = current {
        if let Some(node) = nodes.get(id as usize) {
            result.push(ChildNode {
                id: node.id,
                parent: node.parent,
                name: node.name.clone(),
                is_dir: node.is_dir,
                size: node.size,
                allocated: node.allocated,
                total_size: node.total_size,
                total_allocated: node.total_allocated,
                child_count: node.children.len() as u32,
                file_count: node.file_count,
                dir_count: node.dir_count,
                extension: node.extension.clone(),
            });
            current = node.parent;
        } else {
            break;
        }
    }

    result.reverse();
    Ok(result)
}

#[tauri::command]
pub fn search_files(
    query: String,
    max_results: u32,
    state: State<'_, AppState>,
) -> AppResult<Vec<SearchResult>> {
    let guard = state.tree.lock().unwrap();
    let nodes = guard
        .as_ref()
        .ok_or_else(|| crate::error::AppError::Internal("尚未扫描".into()))?;
    let root = state.root_path.lock().unwrap();
    let root_str = root.as_deref().unwrap_or("");

    if query.is_empty() || nodes.is_empty() {
        return Ok(Vec::new());
    }

    let query_lower = query.to_ascii_lowercase();
    let max = max_results as usize;

    let mut results: Vec<SearchResult> = Vec::new();
    let mut path_cache: HashMap<u32, String> = HashMap::new();
    path_cache.insert(0, root_str.to_string());

    for node in nodes.iter().skip(1) {
        if results.len() >= max {
            break;
        }

        if !node.name.to_ascii_lowercase().contains(&query_lower) {
            continue;
        }

        let path = if let Some(cached) = path_cache.get(&node.id) {
            cached.clone()
        } else {
            let mut parts: Vec<&str> = Vec::new();
            let mut current = node.parent;
            while let Some(pid) = current {
                if let Some(pnode) = nodes.get(pid as usize) {
                    parts.push(&pnode.name);
                    current = pnode.parent;
                } else {
                    break;
                }
            }
            parts.reverse();
            let mut p = root_str.to_string();
            for part in parts {
                if p.ends_with('\\') || p.ends_with('/') {
                    p.push_str(part);
                } else {
                    p.push('\\');
                    p.push_str(part);
                }
            }
            if !p.ends_with('\\') {
                p.push('\\');
            }
            p.push_str(&node.name);
            p
        };

        results.push(SearchResult {
            id: node.id,
            name: node.name.clone(),
            path,
            is_dir: node.is_dir,
            size: node.size,
            allocated: node.allocated,
            total_size: node.total_size,
            total_allocated: node.total_allocated,
            extension: node.extension.clone(),
        });
    }

    results.sort_by(|a, b| b.total_allocated.cmp(&a.total_allocated));
    if results.len() > max {
        results.truncate(max);
    }

    Ok(results)
}

#[tauri::command]
pub fn open_in_explorer(
    path: String,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let guard = state.root_path.lock().unwrap();
    let root = guard
        .as_deref()
        .ok_or_else(|| crate::error::AppError::Internal("尚未扫描".into()))?;

    let root_path = std::path::Path::new(root);
    let path_obj = std::path::Path::new(&path);
    // Resolve .. components before prefix check
    let canonical = path_obj.canonicalize().unwrap_or_else(|_| path_obj.to_path_buf());
    if !canonical.starts_with(root_path) {
        return Err(crate::error::AppError::Win(
            "路径不在当前扫描卷范围内".into(),
        ));
    }
    drop(guard);

    #[cfg(windows)]
    {
        // Drive root "C:\" → open directly; otherwise /select, to highlight
        if path.ends_with(":\\") {
            std::process::Command::new("explorer")
                .arg(&path)
                .spawn()
                .map_err(|e| crate::error::AppError::Win(e.to_string()))?;
        } else {
            std::process::Command::new("explorer")
                .arg("/select,")
                .arg(&path)
                .spawn()
                .map_err(|e| crate::error::AppError::Win(e.to_string()))?;
        }
    }

    #[cfg(not(windows))]
    {
        let open_path = if path_obj.is_dir() {
            path_obj.to_path_buf()
        } else {
            path_obj.parent().unwrap_or(path_obj).to_path_buf()
        };
        tauri_plugin_opener::open_path(&open_path, None::<&str>)
            .map_err(|e| crate::error::AppError::Win(e.to_string()))?;
    }

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
    let guard = state.tree.lock().unwrap();
    let nodes = guard
        .as_ref()
        .ok_or_else(|| crate::error::AppError::Internal("尚未扫描".into()))?;

    let mut ext_map: HashMap<String, ExtensionStat> = HashMap::new();

    for node in nodes {
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

fn find_available_port(start: u16) -> Option<u16> {
    for port in start..start + 10 {
        if TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return Some(port);
        }
    }
    None
}

fn check_http_health(port: u16) -> bool {
    use std::io::{Read, Write};
    if let Ok(mut stream) = std::net::TcpStream::connect(("127.0.0.1", port)) {
        let request = format!("GET /health HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
        if stream.write_all(request.as_bytes()).is_ok() {
            let mut response = String::new();
            if stream.read_to_string(&mut response).is_ok() {
                return response.contains("200 OK") || response.contains("ok");
            }
        }
    }
    false
}

fn get_mcp_binary() -> Result<PathBuf, String> {
    let binary_name = if cfg!(windows) { "fastscan-mcp.exe" } else { "fastscan-mcp" };

    // 1. Environment variable override (highest priority)
    if let Ok(env_path) = std::env::var("FASTSCAN_MCP_PATH") {
        let p = PathBuf::from(&env_path);
        if p.exists() {
            return Ok(p);
        }
    }

    let mut tried = Vec::new();

    if let Ok(exe) = std::env::current_exe() {
        let exe_dir = exe.parent().unwrap();

        // 2. Same directory (bundled sidecar in production)
        let p = exe_dir.join(binary_name);
        tried.push(p.display().to_string());
        if p.exists() {
            return Ok(p);
        }

        // 3. binaries/ subdirectory (Tauri bundled resource path)
        let p = exe_dir.join("binaries").join(binary_name);
        tried.push(p.display().to_string());
        if p.exists() {
            return Ok(p);
        }

        // 4. bin/ subdirectory
        let p = exe_dir.join("bin").join(binary_name);
        tried.push(p.display().to_string());
        if p.exists() {
            return Ok(p);
        }

        // 5-6. Workspace target/ (2 levels up from exe_dir)
        for profile in &["release", "debug"] {
            let p = exe_dir
                .join("..")
                .join("..")
                .join("target")
                .join(profile)
                .join(binary_name);
            tried.push(p.display().to_string());
            if p.exists() {
                return Ok(p);
            }
        }

        // 7-8. Independent crate build (3 levels up from exe_dir, for legacy non-workspace setups)
        for profile in &["release", "debug"] {
            let p = exe_dir
                .join("..")
                .join("..")
                .join("..")
                .join("crates")
                .join("fastscan-mcp")
                .join("target")
                .join(profile)
                .join(binary_name);
            tried.push(p.display().to_string());
            if p.exists() {
                return Ok(p);
            }
        }
    }

    // 9. Search in PATH
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let p = dir.join(binary_name);
            tried.push(p.display().to_string());
            if p.exists() {
                return Ok(p);
            }
        }
    }

    Err(format!(
        "找不到 fastscan-mcp 二进制文件。已尝试以下路径:\n{}",
        tried
            .iter()
            .map(|p| format!("  - {p}"))
            .collect::<Vec<_>>()
            .join("\n")
    ))
}

#[tauri::command]
pub async fn start_mcp_server(
    app: tauri::AppHandle,
    state: State<'_, McpState>,
) -> Result<u16, String> {
    // 启动前先杀掉所有残留 MCP 进程
    kill_orphan_mcp();

    let mut child_guard = state.child.lock().unwrap();
    if child_guard.is_some() {
        return Err("MCP server 已经在运行中".into());
    }

    let port = find_available_port(3721).ok_or("无法找到可用端口 (3721-3730)")?;
    let binary = get_mcp_binary()?;

    let mut cmd = std::process::Command::new(&binary);
    cmd.arg("--http")
        .arg("--port")
        .arg(port.to_string())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    let process = cmd.spawn()
        .map_err(|e| format!("启动 MCP server 失败: {e}"))?;

    // Wait briefly for health check
    let port_clone = port;
    let started = std::time::Instant::now();
    let mut healthy = false;
    while started.elapsed().as_millis() < 3000 {
        if check_http_health(port_clone) {
            healthy = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    if !healthy {
        let _ = std::process::Command::new("taskkill")
            .args(&["/PID", &process.id().to_string(), "/F"])
            .output();
        return Err("MCP server 启动超时 (3s)".into());
    }

    *child_guard = Some(process);
    *state.port.lock().unwrap() = port_clone;

    // 更新托盘菜单文字
    crate::tray::set_mcp_menu_text(&app, true);

    Ok(port_clone)
}

#[tauri::command]
pub async fn stop_mcp_server(
    app: tauri::AppHandle,
    state: State<'_, McpState>,
) -> Result<(), String> {
    let mut child_guard = state.child.lock().unwrap();
    if let Some(mut child) = child_guard.take() {
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("taskkill")
                .args(&["/PID", &child.id().to_string(), "/F"])
                .output();
        }
        #[cfg(not(windows))]
        {
            let _ = child.kill();
        }
        let _ = child.wait();
        *state.port.lock().unwrap() = 0;

        // 更新托盘菜单文字
        crate::tray::set_mcp_menu_text(&app, false);

        Ok(())
    } else {
        Err("MCP server 未在运行".into())
    }
}

#[tauri::command]
pub fn get_mcp_status(state: State<'_, McpState>) -> McpStatus {
    let running = state.child.lock().unwrap().is_some();
    let port = *state.port.lock().unwrap();
    McpStatus { running, port }
}

#[tauri::command]
pub fn hide_main_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|e| e.to_string())
    } else {
        Ok(())
    }
}

#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) -> Result<(), String> {
    kill_mcp_processes(&app);
    app.exit(0);
    Ok(())
}

/// 杀掉托管的 MCP 子进程及所有残留 MCP 进程
pub fn kill_mcp_processes(app: &tauri::AppHandle) {
    let state = app.state::<McpState>();
    if let Some(mut child) = state.child.lock().unwrap().take() {
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("taskkill")
                .args(&["/PID", &child.id().to_string(), "/F"])
                .output();
        }
        #[cfg(not(windows))]
        {
            let _ = child.kill();
        }
        let _ = child.wait();
    }
    *state.port.lock().unwrap() = 0;
    // 确保杀掉所有残留进程
    kill_orphan_mcp();
}

/// 杀掉所有 fastscan-mcp 进程（仅清理孤儿，不影响托管进程）
fn kill_orphan_mcp() {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(&["/IM", "fastscan-mcp.exe", "/F"])
            .output();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("pkill")
            .args(&["-9", "fastscan-mcp"])
            .output();
    }
}


