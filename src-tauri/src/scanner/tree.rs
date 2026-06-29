use std::time::Instant;

use tauri::AppHandle;

use crate::models::NodeDto;
use crate::scanner::emit_progress;

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
        }
    }

    pub fn new(
        id: u32,
        name: impl Into<String>,
        parent: Option<u32>,
        is_dir: bool,
        size: u64,
        allocated: u64,
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
        }
    }
}

pub fn finalize_tree(app: &AppHandle, mut nodes: Vec<TreeNode>) -> Vec<NodeDto> {
    finalize_tree_in_place(app, &mut nodes);
    tree_to_node_dtos(nodes)
}

pub fn finalize_tree_in_place(app: &AppHandle, nodes: &mut Vec<TreeNode>) {
    if nodes.is_empty() {
        nodes.push(TreeNode::root("root"));
    }

    let len = nodes.len();

    for index in 0..len {
        nodes[index]
            .children
            .retain(|child| (*child as usize) < len && *child != index as u32);
    }

    aggregate_from_root(app, nodes);

    let totals: Vec<u64> = nodes.iter().map(|node| node.total_allocated).collect();
    let names: Vec<String> = nodes
        .iter()
        .map(|node| node.name.to_ascii_lowercase())
        .collect();

    let count = nodes.len();
    for (i, node) in nodes.iter_mut().enumerate() {
        node.children.sort_by(|a, b| {
            let ai = *a as usize;
            let bi = *b as usize;
            totals[bi]
                .cmp(&totals[ai])
                .then_with(|| names[ai].cmp(&names[bi]))
        });

        if i % 10000 == 0 && i > 0 {
            emit_progress(
                app,
                "ntfs.aggregate",
                1 + (i * 100 / count.max(1)) as u64,
                Some(100),
                format!("正在排序子节点 {}/{}", i, count),
            );
        }
    }
}

fn aggregate_from_root(app: &AppHandle, nodes: &mut [TreeNode]) {
    let len = nodes.len();
    let mut state = vec![0u8; len];
    let mut stack = vec![(0u32, false)];
    let mut processed = 0u64;
    let mut last_emit = Instant::now();

    while let Some((id, exiting)) = stack.pop() {
        let index = id as usize;
        if index >= len {
            continue;
        }

        if exiting {
            let mut total_size = nodes[index].size;
            let mut total_allocated = nodes[index].allocated;
            let mut file_count: u64 = if nodes[index].is_dir { 0 } else { 1 };
            let mut dir_count: u64 = if index == 0 {
                0
            } else if nodes[index].is_dir {
                1
            } else {
                0
            };

            for child in nodes[index].children.clone() {
                let child_index = child as usize;
                if child_index >= len || state[child_index] != 2 {
                    continue;
                }

                total_size = total_size.saturating_add(nodes[child_index].total_size);
                total_allocated =
                    total_allocated.saturating_add(nodes[child_index].total_allocated);
                file_count = file_count.saturating_add(nodes[child_index].file_count);
                dir_count = dir_count.saturating_add(nodes[child_index].dir_count);
            }

            nodes[index].total_size = total_size;
            nodes[index].total_allocated = total_allocated;
            nodes[index].file_count = file_count;
            nodes[index].dir_count = dir_count;
            state[index] = 2;
            continue;
        }

        if state[index] != 0 {
            continue;
        }

        state[index] = 1;
        stack.push((id, true));

        for child in nodes[index].children.clone().into_iter().rev() {
            let child_index = child as usize;
            if child_index < len && state[child_index] == 0 {
                stack.push((child, false));
            }
        }

        processed += 1;
        if processed % 10000 == 0 && last_emit.elapsed().as_millis() > 100 {
            emit_progress(
                app,
                "ntfs.aggregate",
                processed,
                Some(2).filter(|_| processed > 0),
                format!("正在聚合目录大小... ({} / {} 节点)", processed, len),
            );
            last_emit = Instant::now();
        }
    }
}

fn tree_to_node_dtos(nodes: Vec<TreeNode>) -> Vec<NodeDto> {
    nodes
        .into_iter()
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
