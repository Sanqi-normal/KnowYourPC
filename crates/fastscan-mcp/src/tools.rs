use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;

use crate::models::*;
use crate::progress::{NoopCallback, ScanContext};
use crate::state::AppState;
use crate::win;

pub struct ToolRegistry {
    state: Arc<AppState>,
}

impl ToolRegistry {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    pub fn list_tools(&self) -> Vec<serde_json::Value> {
        vec![
            tool_schema("list_volumes", "List all disk volumes with capacity and filesystem info", serde_json::json!({"type": "object"}), serde_json::json!({
                "type": "array", "items": { "type": "object" }
            })),
            tool_schema("scan_disk", "Deep scan a disk volume. NTFS MFT mode is ~100x faster but needs admin. Results cached for subsequent queries.", serde_json::json!({
                "type": "object",
                "properties": {
                    "root": { "type": "string", "description": "Drive root, e.g. C:\\" },
                    "mode": { "type": "string", "enum": ["auto", "ntfs", "walk"], "description": "Scan mode (auto=try NTFS first, ntfs=MFT fast scan, walk=compatible)" },
                    "includeSystemFiles": { "type": "boolean", "description": "Include system files" }
                },
                "required": ["root"]
            }), serde_json::json!({"type": "object"})),
            tool_schema("scan_status", "Get current scan result summary.", serde_json::json!({"type": "object"}), serde_json::json!({"type": "object"})),
            tool_schema("browse_directory", "Get children of a directory node. Call after scan_disk.", serde_json::json!({
                "type": "object",
                "properties": {
                    "parentId": { "type": "number", "description": "Node ID of the directory" }
                },
                "required": ["parentId"]
            }), serde_json::json!({"type": "array", "items": { "type": "object" }})),
            tool_schema("get_node_path", "Get the full path of a file/directory node.", serde_json::json!({
                "type": "object",
                "properties": {
                    "nodeId": { "type": "number", "description": "Node ID" }
                },
                "required": ["nodeId"]
            }), serde_json::json!({"type": "string"})),
            tool_schema("get_node_details", "Get a node plus all its ancestors (breadcrumb).", serde_json::json!({
                "type": "object",
                "properties": {
                    "nodeId": { "type": "number", "description": "Node ID" }
                },
                "required": ["nodeId"]
            }), serde_json::json!({"type": "array", "items": { "type": "object" }})),
            tool_schema("search_files", "Search files/folders by name with optional filters.", serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query (case-insensitive)" },
                    "maxResults": { "type": "number", "description": "Max results (default 50)" },
                    "minSize": { "type": "number", "description": "Minimum file size in bytes" },
                    "maxSize": { "type": "number", "description": "Maximum file size in bytes" },
                    "extension": { "type": "string", "description": "Filter by extension (e.g. '.exe', 'exe')" },
                    "isDir": { "type": "boolean", "description": "Filter by directory: true=only dirs, false=only files" },
                    "olderThanDays": { "type": "number", "description": "Files not modified in N days" }
                },
                "required": ["query"]
            }), serde_json::json!({"type": "array", "items": { "type": "object" }})),
            tool_schema("get_extension_stats", "Get file extension statistics from the last scan.", serde_json::json!({"type": "object"}), serde_json::json!({
                "type": "array", "items": { "type": "object" }
            })),
            tool_schema("get_treemap", "Get treemap visualization data for a directory.", serde_json::json!({
                "type": "object",
                "properties": {
                    "rootId": { "type": "number", "description": "Node ID to treemap" },
                    "maxItems": { "type": "number", "description": "Max items per level (default 50)" }
                },
                "required": ["rootId"]
            }), serde_json::json!({"type": "array", "items": { "type": "object" }})),
            tool_schema("get_largest_files", "Get the largest files from the last scan.", serde_json::json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "number", "description": "Number of files to return (default 50)" },
                    "minSize": { "type": "number", "description": "Minimum file size in bytes" },
                    "extension": { "type": "string", "description": "Filter by extension (e.g. '.exe')" }
                }
            }), serde_json::json!({"type": "array", "items": { "type": "object" }})),
            tool_schema("get_largest_directories", "Get the largest directories from the last scan.", serde_json::json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "number", "description": "Number of directories to return (default 50)" },
                    "minSize": { "type": "number", "description": "Minimum total size in bytes" }
                }
            }), serde_json::json!({"type": "array", "items": { "type": "object" }})),
            tool_schema("find_empty_directories", "Find empty directories (no files, no subdirectories).", serde_json::json!({
                "type": "object",
                "properties": {
                    "maxResults": { "type": "number", "description": "Max results (default 100)" }
                }
            }), serde_json::json!({"type": "array", "items": { "type": "object" }})),
            tool_schema("find_duplicate_files", "Find duplicate files by name and size.", serde_json::json!({
                "type": "object",
                "properties": {
                    "minSize": { "type": "number", "description": "Minimum file size in bytes to consider (default 0)" },
                    "maxResults": { "type": "number", "description": "Max duplicate groups to return (default 50)" }
                }
            }), serde_json::json!({"type": "array", "items": { "type": "object" }})),
            tool_schema("find_files_by_age", "Find files by age (modification time).", serde_json::json!({
                "type": "object",
                "properties": {
                    "olderThanDays": { "type": "number", "description": "Files not modified in N days (required)" },
                    "maxResults": { "type": "number", "description": "Max results (default 50)" },
                    "extension": { "type": "string", "description": "Filter by extension" },
                    "minSize": { "type": "number", "description": "Minimum file size in bytes" }
                },
                "required": ["olderThanDays"]
            }), serde_json::json!({"type": "array", "items": { "type": "object" }})),
        ]
    }

    pub async fn call_tool(&self, name: &str, args: Value) -> Result<Value, String> {
        match name {
            "list_volumes" => self.list_volumes(),
            "scan_disk" => self.scan_disk(args).await,
            "scan_status" => self.scan_status(),
            "browse_directory" => self.browse_directory(args),
            "get_node_path" => self.get_node_path(args),
            "get_node_details" => self.get_node_details(args),
            "search_files" => self.search_files(args),
            "get_extension_stats" => self.get_extension_stats(),
            "get_treemap" => self.get_treemap(args),
            "get_largest_files" => self.get_largest_files(args),
            "get_largest_directories" => self.get_largest_directories(args),
            "find_empty_directories" => self.find_empty_directories(args),
            "find_duplicate_files" => self.find_duplicate_files(args),
            "find_files_by_age" => self.find_files_by_age(args),
            _ => Err(format!("Unknown tool: {name}")),
        }
    }

    fn list_volumes(&self) -> Result<Value, String> {
        let volumes = win::volume::list_volumes().map_err(|e| e.to_string())?;
        serde_json::to_value(volumes).map_err(|e| e.to_string())
    }

    async fn scan_disk(&self, args: Value) -> Result<Value, String> {
        let root = args.get("root").and_then(|v| v.as_str()).ok_or("Missing 'root' argument")?.to_string();
        let mode_str = args.get("mode").and_then(|v| v.as_str()).unwrap_or("auto");
        let _ = args.get("includeSystemFiles").and_then(|v| v.as_bool()).unwrap_or(false);

        let mode = match mode_str {
            "ntfs" | "ntfsMft" => ScanMode::NtfsMft,
            "walk" => ScanMode::Walk,
            _ => ScanMode::Auto,
        };

        let options = ScanOptions { root, mode };

        let ctx = ScanContext::new(Arc::new(NoopCallback), 4096);
        let result = tokio::task::spawn_blocking(move || {
            crate::scanner::scan(&ctx, options)
        }).await.map_err(|e| format!("扫描线程失败: {e}"))?
        .map_err(|e| e.to_string())?;

        let mut tree = self.state.tree.lock().unwrap();
        *tree = Some(result.nodes);
        drop(tree);
        let mut rp = self.state.root_path.lock().unwrap();
        *rp = Some(result.root);

        serde_json::to_value(&ScanStatus {
            elapsed_ms: result.elapsed_ms,
            node_count: result.node_count,
            file_count: result.file_count,
            dir_count: result.dir_count,
            total_size: result.total_size,
            total_allocated: result.total_allocated,
            scanner: result.scanner,
            warnings: result.warnings,
        }).map_err(|e| e.to_string())
    }

    fn scan_status(&self) -> Result<Value, String> {
        let guard = self.state.tree.lock().unwrap();
        let nodes = guard.as_ref().ok_or("尚未扫描，请先调用 scan_disk")?;
        let root = &nodes[0];
        let scanned = nodes.len() > 1;
        serde_json::to_value(serde_json::json!({
            "scanned": scanned,
            "nodeCount": nodes.len(),
            "fileCount": root.file_count,
            "dirCount": root.dir_count,
            "totalSize": root.total_size,
            "totalAllocated": root.total_allocated,
        })).map_err(|e| e.to_string())
    }

    fn to_child_node(node: &NodeDto) -> ChildNode {
        ChildNode {
            id: node.id, parent: node.parent, name: node.name.clone(),
            is_dir: node.is_dir, size: node.size, allocated: node.allocated,
            total_size: node.total_size, total_allocated: node.total_allocated,
            child_count: node.children.len() as u32,
            file_count: node.file_count, dir_count: node.dir_count,
            extension: node.extension.clone(),
            created: node.created, modified: node.modified, accessed: node.accessed,
        }
    }

    fn build_path(&self, node_id: u32, nodes: &[NodeDto], root_str: &str) -> String {
        let mut parts: Vec<&str> = Vec::new();
        let mut current = Some(node_id);
        while let Some(id) = current {
            if let Some(node) = nodes.get(id as usize) {
                parts.push(&node.name);
                current = node.parent;
            } else { break; }
        }
        parts.reverse();
        if parts.is_empty() { return root_str.to_string(); }
        let mut path = parts[0].to_string();
        for part in &parts[1..] {
            if path.ends_with('\\') || path.ends_with('/') { path.push_str(part); }
            else { path.push('\\'); path.push_str(part); }
        }
        path
    }

    fn browse_directory(&self, args: Value) -> Result<Value, String> {
        let parent_id = args.get("parentId").and_then(|v| v.as_u64()).ok_or("Missing 'parentId'")? as u32;
        let guard = self.state.tree.lock().unwrap();
        let nodes = guard.as_ref().ok_or("尚未扫描，请先调用 scan_disk")?;
        let parent = nodes.get(parent_id as usize).ok_or("节点不存在")?;

        let children: Vec<ChildNode> = parent.children.iter()
            .filter_map(|cid| {
                let child = nodes.get(*cid as usize)?;
                Some(Self::to_child_node(child))
            }).collect();

        serde_json::to_value(children).map_err(|e| e.to_string())
    }

    fn get_node_path(&self, args: Value) -> Result<Value, String> {
        let node_id = args.get("nodeId").and_then(|v| v.as_u64()).ok_or("Missing 'nodeId'")? as u32;
        let guard = self.state.tree.lock().unwrap();
        let nodes = guard.as_ref().ok_or("尚未扫描")?;
        let root = self.state.root_path.lock().unwrap();
        let root_str = root.as_deref().unwrap_or("");

        Ok(Value::String(self.build_path(node_id, nodes, root_str)))
    }

    fn get_node_details(&self, args: Value) -> Result<Value, String> {
        let node_id = args.get("nodeId").and_then(|v| v.as_u64()).ok_or("Missing 'nodeId'")? as u32;
        let guard = self.state.tree.lock().unwrap();
        let nodes = guard.as_ref().ok_or("尚未扫描")?;

        let mut result: Vec<ChildNode> = Vec::new();
        let mut current = Some(node_id);
        while let Some(id) = current {
            if let Some(node) = nodes.get(id as usize) {
                result.push(Self::to_child_node(node));
                current = node.parent;
            } else { break; }
        }
        result.reverse();
        serde_json::to_value(result).map_err(|e| e.to_string())
    }

    fn search_files(&self, args: Value) -> Result<Value, String> {
        let query = args.get("query").and_then(|v| v.as_str()).ok_or("Missing 'query'")?.to_string();
        let max_results = args.get("maxResults").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
        let min_size = args.get("minSize").and_then(|v| v.as_u64());
        let max_size = args.get("maxSize").and_then(|v| v.as_u64());
        let ext_filter = args.get("extension").and_then(|v| v.as_str()).map(|s| s.trim_start_matches('.').to_ascii_lowercase());
        let is_dir_filter = args.get("isDir").and_then(|v| v.as_bool());
        let older_than_days = args.get("olderThanDays").and_then(|v| v.as_u64());
        let older_than_secs = older_than_days.map(|d| (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64) - (d as i64 * 86400));

        let guard = self.state.tree.lock().unwrap();
        let nodes = guard.as_ref().ok_or("尚未扫描")?;
        let root = self.state.root_path.lock().unwrap();
        let root_str = root.as_deref().unwrap_or("");

        if query.is_empty() || nodes.is_empty() { return Ok(Value::Array(Vec::new())); }

        let query_lower = query.to_ascii_lowercase();
        let mut results: Vec<SearchResult> = Vec::new();
        let mut path_cache: HashMap<u32, String> = HashMap::new();
        path_cache.insert(0, root_str.to_string());

        for node in nodes.iter().skip(1) {
            if results.len() >= max_results { break; }
            if !node.name.to_ascii_lowercase().contains(&query_lower) { continue; }
            if let Some(f) = is_dir_filter { if node.is_dir != f { continue; } }
            if let Some(ref ext) = ext_filter {
                match node.extension { Some(ref e) => if e != ext { continue; } None => continue }
            }
            if let Some(mn) = min_size { if node.size < mn { continue; } }
            if let Some(mx) = max_size { if node.size > mx { continue; } }
            if let Some(ots) = older_than_secs {
                match node.modified { Some(m) if m < ots => {} _ => continue }
            }

            let path = if let Some(cached) = path_cache.get(&node.id) { cached.clone() }
            else {
                let p = self.build_path(node.id, nodes, root_str);
                path_cache.insert(node.id, p.clone());
                p
            };

            results.push(SearchResult {
                id: node.id, name: node.name.clone(), path,
                is_dir: node.is_dir, size: node.size, allocated: node.allocated,
                total_size: node.total_size, total_allocated: node.total_allocated,
                extension: node.extension.clone(),
                created: node.created, modified: node.modified, accessed: node.accessed,
            });
        }

        results.sort_by(|a, b| b.total_allocated.cmp(&a.total_allocated));
        results.truncate(max_results);
        serde_json::to_value(results).map_err(|e| e.to_string())
    }

    fn get_extension_stats(&self) -> Result<Value, String> {
        let guard = self.state.tree.lock().unwrap();
        let nodes = guard.as_ref().ok_or("尚未扫描")?;

        let mut ext_map: HashMap<String, ExtensionStat> = HashMap::new();
        for node in nodes {
            if node.is_dir { continue; }
            let ext = node.extension.as_deref()
                .filter(|e| !e.is_empty())
                .map(|e| e.to_ascii_lowercase())
                .unwrap_or_else(|| "(无扩展名)".to_string());

            let entry = ext_map.entry(ext.clone()).or_insert(ExtensionStat { extension: ext, size: 0, allocated: 0, file_count: 0 });
            entry.size = entry.size.saturating_add(node.total_size);
            entry.allocated = entry.allocated.saturating_add(node.total_allocated);
            entry.file_count += 1;
        }

        let mut stats: Vec<ExtensionStat> = ext_map.into_values().collect();
        stats.sort_by(|a, b| b.allocated.cmp(&a.allocated));
        if stats.len() > 100 { stats.truncate(100); }
        serde_json::to_value(stats).map_err(|e| e.to_string())
    }

    fn get_treemap(&self, args: Value) -> Result<Value, String> {
        let root_id = args.get("rootId").and_then(|v| v.as_u64()).ok_or("Missing 'rootId'")? as u32;
        let max_items = args.get("maxItems").and_then(|v| v.as_u64()).unwrap_or(50) as u32;

        let guard = self.state.tree.lock().unwrap();
        let nodes = guard.as_ref().ok_or("尚未扫描")?;
        let root = nodes.get(root_id as usize).ok_or("根节点不存在")?;

        let total_size: u64 = root.children.iter()
            .filter_map(|id| nodes.get(*id as usize))
            .map(|n| n.total_allocated).sum();

        let mut items: Vec<TreemapNode> = Vec::new();
        for &child_id in &root.children {
            if items.len() >= 10000 { break; }
            let node = match nodes.get(child_id as usize) {
                Some(n) if n.total_allocated > 0 => n,
                _ => continue,
            };
            let budget = if total_size > 0 {
                ((node.total_allocated as f64 / total_size as f64) * max_items as f64).max(1.0) as u32
            } else { 1 };
            let mut child_remaining = budget;
            if let Some(child) = build_treemap_node(child_id, nodes, &mut child_remaining) {
                items.push(child);
            }
        }
        items.sort_by(|a, b| b.size.cmp(&a.size));
        serde_json::to_value(items).map_err(|e| e.to_string())
    }

    fn get_largest_files(&self, args: Value) -> Result<Value, String> {
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
        let min_size = args.get("minSize").and_then(|v| v.as_u64()).unwrap_or(0);
        let ext_filter = args.get("extension").and_then(|v| v.as_str()).map(|s| s.trim_start_matches('.').to_ascii_lowercase());

        let guard = self.state.tree.lock().unwrap();
        let nodes = guard.as_ref().ok_or("尚未扫描")?;
        let root = self.state.root_path.lock().unwrap();
        let root_str = root.as_deref().unwrap_or("");

        let mut files: Vec<FileInfo> = nodes.iter().filter(|n| {
            if n.is_dir { return false; }
            if n.size < min_size { return false; }
            if let Some(ref ext) = ext_filter {
                match n.extension { Some(ref e) if e == ext => {} _ => return false }
            }
            true
        }).map(|n| {
            let path = self.build_path(n.id, nodes, root_str);
            FileInfo {
                id: n.id, name: n.name.clone(), path,
                size: n.size, allocated: n.allocated,
                extension: n.extension.clone(),
                created: n.created, modified: n.modified, accessed: n.accessed,
            }
        }).collect();

        files.sort_by(|a, b| b.size.cmp(&a.size));
        files.truncate(limit);
        serde_json::to_value(files).map_err(|e| e.to_string())
    }

    fn get_largest_directories(&self, args: Value) -> Result<Value, String> {
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
        let min_size = args.get("minSize").and_then(|v| v.as_u64()).unwrap_or(0);

        let guard = self.state.tree.lock().unwrap();
        let nodes = guard.as_ref().ok_or("尚未扫描")?;
        let root = self.state.root_path.lock().unwrap();
        let root_str = root.as_deref().unwrap_or("");

        let mut dirs: Vec<DirInfo> = nodes.iter().filter(|n| {
            n.is_dir && n.id != 0 && n.total_allocated >= min_size
        }).map(|n| {
            let path = self.build_path(n.id, nodes, root_str);
            DirInfo {
                id: n.id, name: n.name.clone(), path,
                total_size: n.total_size, total_allocated: n.total_allocated,
                file_count: n.file_count, dir_count: n.dir_count,
                child_count: n.children.len() as u32,
            }
        }).collect();

        dirs.sort_by(|a, b| b.total_allocated.cmp(&a.total_allocated));
        dirs.truncate(limit);
        serde_json::to_value(dirs).map_err(|e| e.to_string())
    }

    fn find_empty_directories(&self, args: Value) -> Result<Value, String> {
        let max_results = args.get("maxResults").and_then(|v| v.as_u64()).unwrap_or(100) as usize;

        let guard = self.state.tree.lock().unwrap();
        let nodes = guard.as_ref().ok_or("尚未扫描")?;
        let root = self.state.root_path.lock().unwrap();
        let root_str = root.as_deref().unwrap_or("");

        let mut dirs: Vec<DirInfo> = nodes.iter().filter(|n| {
            n.is_dir && n.id != 0 && n.children.is_empty()
        }).map(|n| {
            let path = self.build_path(n.id, nodes, root_str);
            DirInfo {
                id: n.id, name: n.name.clone(), path,
                total_size: n.total_size, total_allocated: n.total_allocated,
                file_count: n.file_count, dir_count: n.dir_count,
                child_count: 0,
            }
        }).collect();

        dirs.sort_by(|a, b| b.total_allocated.cmp(&a.total_allocated));
        dirs.truncate(max_results);
        serde_json::to_value(dirs).map_err(|e| e.to_string())
    }

    fn find_duplicate_files(&self, args: Value) -> Result<Value, String> {
        let min_size = args.get("minSize").and_then(|v| v.as_u64()).unwrap_or(0);
        let max_results = args.get("maxResults").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

        let guard = self.state.tree.lock().unwrap();
        let nodes = guard.as_ref().ok_or("尚未扫描")?;
        let root = self.state.root_path.lock().unwrap();
        let root_str = root.as_deref().unwrap_or("");

        let mut groups: HashMap<(String, u64), Vec<&NodeDto>> = HashMap::new();
        for node in nodes.iter().skip(1) {
            if node.is_dir { continue; }
            if node.size < min_size { continue; }
            let key = (node.name.to_ascii_lowercase(), node.size);
            groups.entry(key).or_default().push(node);
        }

        let mut result: Vec<DuplicateGroup> = groups.into_iter()
            .filter(|(_, v)| v.len() > 1)
            .map(|((name, size), files)| {
                DuplicateGroup {
                    file_name: name,
                    size,
                    files: files.iter().map(|n| FileInfo {
                        id: n.id, name: n.name.clone(),
                        path: self.build_path(n.id, nodes, root_str),
                        size: n.size, allocated: n.allocated,
                        extension: n.extension.clone(),
                        created: n.created, modified: n.modified, accessed: n.accessed,
                    }).collect(),
                }
            }).collect();

        result.sort_by(|a, b| b.files.len().cmp(&a.files.len()));
        result.truncate(max_results);
        serde_json::to_value(result).map_err(|e| e.to_string())
    }

    fn find_files_by_age(&self, args: Value) -> Result<Value, String> {
        let older_than_days = args.get("olderThanDays").and_then(|v| v.as_u64()).ok_or("Missing 'olderThanDays'")?;
        let max_results = args.get("maxResults").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
        let ext_filter = args.get("extension").and_then(|v| v.as_str()).map(|s| s.trim_start_matches('.').to_ascii_lowercase());
        let min_size = args.get("minSize").and_then(|v| v.as_u64()).unwrap_or(0);

        let cutoff = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64) - (older_than_days as i64 * 86400);

        let guard = self.state.tree.lock().unwrap();
        let nodes = guard.as_ref().ok_or("尚未扫描")?;
        let root = self.state.root_path.lock().unwrap();
        let root_str = root.as_deref().unwrap_or("");

        let mut files: Vec<FileInfo> = nodes.iter().filter(|n| {
            if n.is_dir { return false; }
            if n.size < min_size { return false; }
            match n.modified { Some(m) if m < cutoff => {} _ => return false }
            if let Some(ref ext) = ext_filter {
                match n.extension { Some(ref e) if e == ext => {} _ => return false }
            }
            true
        }).map(|n| {
            let path = self.build_path(n.id, nodes, root_str);
            FileInfo {
                id: n.id, name: n.name.clone(), path,
                size: n.size, allocated: n.allocated,
                extension: n.extension.clone(),
                created: n.created, modified: n.modified, accessed: n.accessed,
            }
        }).collect();

        files.sort_by(|a, b| a.modified.cmp(&b.modified));
        files.truncate(max_results);
        serde_json::to_value(files).map_err(|e| e.to_string())
    }
}

fn build_treemap_node(id: u32, nodes: &[NodeDto], remaining: &mut u32) -> Option<TreemapNode> {
    let node = nodes.get(id as usize)?;
    if node.total_allocated == 0 { return None; }
    if node.is_dir && !node.children.is_empty() {
        let mut children: Vec<TreemapNode> = Vec::new();
        for &child_id in &node.children {
            if *remaining == 0 { break; }
            if let Some(child) = build_treemap_node(child_id, nodes, remaining) { children.push(child); }
        }
        children.sort_by(|a, b| b.size.cmp(&a.size));
        *remaining = remaining.saturating_sub(1);
        Some(TreemapNode { id: node.id, size: node.total_allocated, name: node.name.clone(), is_dir: true, extension: None, children })
    } else {
        if *remaining == 0 { return None; }
        *remaining -= 1;
        Some(TreemapNode { id: node.id, size: node.total_allocated, name: node.name.clone(), is_dir: node.is_dir, extension: node.extension.clone(), children: Vec::new() })
    }
}

fn tool_schema(name: &str, description: &str, input_schema: Value, _output_schema: Value) -> Value {
    serde_json::json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema,
    })
}
