use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{bail, Result};
use tauri::AppHandle;
use walkdir::WalkDir;
use crate::{
    models::ScanResult,
    scanner::{
        emit_progress,
        tree::{finalize_tree, TreeNode},
    },
};

const PROGRESS_INTERVAL_MS: u64 = 200;
const WARN_LIMIT: usize = 64;

pub fn scan_path(app: &AppHandle, root: &str) -> Result<ScanResult> {
    let started = Instant::now();
    let root_path = PathBuf::from(root);

    if !root_path.exists() {
        bail!("路径不存在: {root}");
    }

    emit_progress(
        app,
        "walk.start",
        0,
        None,
        format!("正在使用兼容递归模式扫描 {root}"),
    );

    let cluster_size = crate::win::volume::cluster_size_for_root(root).max(1);
    let mut warnings = Vec::new();

    let mut nodes = Vec::with_capacity(65536);
    nodes.push(TreeNode::root(display_root(&root_path)));

    let mut parent_at_depth: Vec<u32> = vec![0];

    let mut processed = 0u64;
    let mut last_emit = Instant::now();

    for entry in WalkDir::new(&root_path).follow_links(false) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                push_warning(&mut warnings, error.to_string());
                continue;
            }
        };

        if entry.depth() == 0 {
            continue;
        }

        processed += 1;

        let depth = entry.depth();
        if parent_at_depth.len() <= depth {
            parent_at_depth.resize(depth + 1, 0);
        }
        let parent_id = parent_at_depth[depth - 1];

        let ft_is_dir = entry.file_type().is_dir();

        let metadata = if !ft_is_dir {
            match entry.metadata() {
                Ok(m) => Some(m),
                Err(error) => {
                    push_warning(
                        &mut warnings,
                        format!("无法读取元数据 {}: {error}", entry.path().display()),
                    );
                    continue;
                }
            }
        } else {
            None
        };

        let is_dir = ft_is_dir || metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let is_file = metadata.as_ref().map(|m| m.is_file()).unwrap_or(false);
        let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
        let allocated = if is_file {
            approximate_allocated(size, cluster_size)
        } else {
            0
        };

        let id = nodes.len() as u32;
        let name = entry.file_name().to_string_lossy().to_string();

        nodes.push(TreeNode::new(
            id,
            name,
            Some(parent_id),
            is_dir,
            size,
            allocated,
        ));

        nodes[parent_id as usize].children.push(id);

        if is_dir {
            parent_at_depth[depth] = id;
        }

        if (processed & 4095) == 0 && last_emit.elapsed().as_millis() > PROGRESS_INTERVAL_MS as u128 {
            emit_progress(
                app,
                "walk.scan",
                processed,
                None,
                format!("兼容递归扫描中，已处理 {} 个项目", processed),
            );
            last_emit = Instant::now();
        }
    }

    emit_progress(
        app,
        "walk.aggregate",
        1,
        Some(2),
        format!("正在聚合目录大小 ({} 个项目)", processed),
    );

    let nodes = finalize_tree(app, nodes);

    emit_progress(
        app,
        "walk.done",
        2,
        Some(2),
        format!("聚合完成，共 {} 个节点", nodes.len()),
    );

    let total_size = nodes.first().map(|node| node.total_size).unwrap_or(0);
    let total_allocated = nodes.first().map(|node| node.total_allocated).unwrap_or(0);
    let file_count = nodes.first().map(|node| node.file_count).unwrap_or(0);
    let dir_count = nodes.first().map(|node| node.dir_count).unwrap_or(0);

    Ok(ScanResult {
        root: root.to_string(),
        scanner: "walkdir-seq".to_string(),
        elapsed_ms: started.elapsed().as_millis() as u64,
        node_count: nodes.len() as u64,
        file_count,
        dir_count,
        total_size,
        total_allocated,
        nodes,
        warnings,
    })
}

fn approximate_allocated(size: u64, cluster_size: u64) -> u64 {
    if size == 0 { 0 } else { size.div_ceil(cluster_size) * cluster_size }
}

fn display_root(path: &Path) -> String {
    let text = path.display().to_string();
    if text.is_empty() { "/".to_string() } else { text }
}

fn push_warning(warnings: &mut Vec<String>, warning: String) {
    if warnings.len() < WARN_LIMIT {
        warnings.push(warning);
    }
}
