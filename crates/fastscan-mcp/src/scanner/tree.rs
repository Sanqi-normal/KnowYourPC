use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};

use rayon::prelude::*;

use crate::models::NodeDto;
use crate::progress::ScanContext;

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub id: u32,
    pub parent: Option<u32>,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub allocated: u64,
    pub total_size: u64,
    pub total_allocated: u64,
    pub children: Vec<u32>,
    pub file_count: u64,
    pub dir_count: u64,
    pub extension: Option<String>,
    pub created: Option<i64>,
    pub modified: Option<i64>,
    pub accessed: Option<i64>,
}

impl TreeNode {
    pub fn root(name: impl Into<String>) -> Self {
        Self {
            id: 0,
            parent: None,
            name: name.into(),
            is_dir: true,
            size: 0,
            allocated: 0,
            total_size: 0,
            total_allocated: 0,
            children: Vec::new(),
            file_count: 0,
            dir_count: 0,
            extension: None,
            created: None,
            modified: None,
            accessed: None,
        }
    }

    pub fn new(
        id: u32,
        name: impl Into<String>,
        parent: Option<u32>,
        is_dir: bool,
        size: u64,
        allocated: u64,
        created: Option<i64>,
        modified: Option<i64>,
        accessed: Option<i64>,
    ) -> Self {
        let name = name.into();
        Self {
            id,
            parent,
            extension: if is_dir { None } else { extension_of(&name) },
            name,
            is_dir,
            size,
            allocated,
            total_size: size,
            total_allocated: allocated,
            children: Vec::new(),
            file_count: if is_dir { 0 } else { 1 },
            dir_count: if is_dir { 1 } else { 0 },
            created,
            modified,
            accessed,
        }
    }
}

pub fn finalize_tree(ctx: &ScanContext, mut nodes: Vec<TreeNode>) -> Vec<NodeDto> {
    finalize_tree_in_place(ctx, &mut nodes);
    tree_to_node_dtos(nodes)
}

pub fn finalize_tree_in_place(ctx: &ScanContext, nodes: &mut Vec<TreeNode>) {
    if nodes.is_empty() {
        nodes.push(TreeNode::root("root"));
    }

    let len = nodes.len();

    nodes.par_iter_mut().enumerate().for_each(|(i, node)| {
        node.children.retain(|child| (*child as usize) < len && *child != i as u32);
    });

    parallel_aggregate(nodes);

    let totals: Vec<u64> = nodes.iter().map(|node| node.total_allocated).collect();
    let names: Vec<String> = nodes
        .iter()
        .map(|node| node.name.to_ascii_lowercase())
        .collect();

    let count = nodes.len();
    nodes.par_iter_mut().enumerate().for_each(|(_i, node)| {
        node.children.sort_by(|a, b| {
            let ai = *a as usize;
            let bi = *b as usize;
            if ai >= count || bi >= count {
                return std::cmp::Ordering::Equal;
            }
            totals[bi]
                .cmp(&totals[ai])
                .then_with(|| names[ai].cmp(&names[bi]))
        });
    });

    ctx.progress.emit(
        "ntfs.aggregate",
        100,
        Some(100),
        &format!("聚合完成 ({} 个节点)", count),
    );
}

fn parallel_aggregate(nodes: &mut [TreeNode]) {
    let len = nodes.len();
    if len <= 1 { return; }

    let mut depth = vec![usize::MAX; len];
    depth[0] = 0;
    let mut queue = VecDeque::new();
    queue.push_back(0usize);

    while let Some(idx) = queue.pop_front() {
        for &child in &nodes[idx].children {
            let ci = child as usize;
            if ci < len && depth[ci] == usize::MAX {
                depth[ci] = depth[idx] + 1;
                queue.push_back(ci);
            }
        }
    }

    let max_depth = depth
        .iter()
        .filter(|&&d| d != usize::MAX)
        .max()
        .copied()
        .unwrap_or(0);

    if max_depth == 0 {
        nodes[0].total_size = nodes.iter().skip(1).map(|n| n.size).sum();
        nodes[0].total_allocated = nodes.iter().skip(1).map(|n| n.allocated).sum();
        nodes[0].file_count = nodes.iter().skip(1).filter(|n| !n.is_dir).count() as u64;
        nodes[0].dir_count = nodes.iter().skip(1).filter(|n| n.is_dir).count() as u64;
        return;
    }

    let mut by_depth: Vec<Vec<usize>> = vec![Vec::new(); max_depth + 1];
    for (i, &d) in depth.iter().enumerate() {
        if d != usize::MAX {
            by_depth[d].push(i);
        }
    }

    let total_sizes: Vec<AtomicU64> = nodes.iter().map(|n| AtomicU64::new(n.size)).collect();
    let total_allocated: Vec<AtomicU64> = nodes.iter().map(|n| AtomicU64::new(n.allocated)).collect();
    let file_counts: Vec<AtomicU64> = nodes
        .iter()
        .map(|n| AtomicU64::new(if n.is_dir { 0 } else { 1 }))
        .collect();
    let dir_counts: Vec<AtomicU64> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            if i == 0 { AtomicU64::new(0) }
            else if n.is_dir { AtomicU64::new(1) }
            else { AtomicU64::new(0) }
        })
        .collect();

    let parents: Vec<Option<u32>> = nodes.iter().map(|n| n.parent).collect();

    for d in (1..=max_depth).rev() {
        by_depth[d].par_iter().for_each(|&idx| {
            let parent_idx = match parents[idx] {
                Some(p) if (p as usize) < len && p as usize != idx => p as usize,
                _ => return,
            };
            total_sizes[parent_idx].fetch_add(total_sizes[idx].load(Ordering::Relaxed), Ordering::Relaxed);
            total_allocated[parent_idx].fetch_add(total_allocated[idx].load(Ordering::Relaxed), Ordering::Relaxed);
            file_counts[parent_idx].fetch_add(file_counts[idx].load(Ordering::Relaxed), Ordering::Relaxed);
            dir_counts[parent_idx].fetch_add(dir_counts[idx].load(Ordering::Relaxed), Ordering::Relaxed);
        });
    }

    nodes.par_iter_mut().enumerate().for_each(|(i, node)| {
        node.total_size = total_sizes[i].load(Ordering::Relaxed);
        node.total_allocated = total_allocated[i].load(Ordering::Relaxed);
        node.file_count = file_counts[i].load(Ordering::Relaxed);
        node.dir_count = dir_counts[i].load(Ordering::Relaxed);
    });
}

fn tree_to_node_dtos(nodes: Vec<TreeNode>) -> Vec<NodeDto> {
    nodes
        .into_par_iter()
        .map(|node| NodeDto {
            id: node.id,
            parent: node.parent,
            name: node.name,
            is_dir: node.is_dir,
            size: node.size,
            allocated: node.allocated,
            total_size: node.total_size,
            total_allocated: node.total_allocated,
            child_count: node.children.len() as u32,
            children: node.children,
            file_count: node.file_count,
            dir_count: node.dir_count,
            extension: node.extension,
            created: node.created,
            modified: node.modified,
            accessed: node.accessed,
        })
        .collect()
}

fn extension_of(name: &str) -> Option<String> {
    std::path::Path::new(name)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .filter(|value| !value.is_empty())
}
