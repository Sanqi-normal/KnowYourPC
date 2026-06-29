mod boot;
mod record;
mod runs;

use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use tauri::AppHandle;

use crate::{
    models::ScanResult,
    scanner::{
        emit_progress,
        ntfs::{
            boot::parse_boot_sector,
            record::{MftStream, ParsedRecord},
        },
        tree::{finalize_tree, TreeNode},
    },
};

#[cfg(windows)]
use std::{
    fs::{File, OpenOptions},
    os::windows::fs::{FileExt, OpenOptionsExt},
    time::Instant,
};

#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{
    FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_SEQUENTIAL_SCAN, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE,
};

#[cfg(windows)]
const MFT_READ_CHUNK_BYTES: u64 = 16 * 1024 * 1024;

#[cfg(windows)]
pub fn scan_volume(app: &AppHandle, root: &str) -> Result<ScanResult> {
    let started = Instant::now();

    let root_display = drive_root_from_input(root)?;
    let device_path = volume_device_path(&root_display)?;

    emit_progress(
        app,
        "ntfs.open",
        0,
        None,
        format!("正在打开卷 {device_path}"),
    );

    let volume = open_volume(&device_path)?;

    let mut boot_sector = [0u8; 512];
    read_exact_at(&volume, 0, &mut boot_sector).context("读取 NTFS Boot Sector 失败")?;

    let boot = parse_boot_sector(&boot_sector)?;

    let mft_record0_offset = boot
        .mft_lcn
        .checked_mul(boot.cluster_size)
        .context("MFT offset overflow")?;

    let mut mft_record0 = vec![0u8; boot.file_record_size];
    read_exact_at(&volume, mft_record0_offset, &mut mft_record0)
        .context("读取 $MFT record 0 失败")?;

    let mft = record::parse_mft_stream(&mut mft_record0, boot.bytes_per_sector as usize)?;

    let record_count = mft.data_size / boot.file_record_size as u64;

    emit_progress(
        app,
        "ntfs.mft",
        0,
        Some(record_count),
        format!(
            "NTFS 几何: cluster={} bytes, FILE record={} bytes, MFT records≈{}",
            boot.cluster_size, boot.file_record_size, record_count
        ),
    );

    let parsed_records = enumerate_mft_records(app, &volume, &boot, &mft, record_count)?;

    emit_progress(
        app,
        "ntfs.tree",
        parsed_records.len() as u64,
        None,
        "正在重建目录树并聚合大小".to_string(),
    );

    let nodes = build_tree_from_records(&root_display, parsed_records);

    let total_size = nodes.first().map(|node| node.total_size).unwrap_or(0);
    let total_allocated = nodes
        .first()
        .map(|node| node.total_allocated)
        .unwrap_or(0);
    let file_count = nodes.first().map(|node| node.file_count).unwrap_or(0);
    let dir_count = nodes.first().map(|node| node.dir_count).unwrap_or(0);

    Ok(ScanResult {
        root: root_display,
        scanner: "ntfs-mft-raw".to_string(),
        elapsed_ms: started.elapsed().as_millis() as u64,
        node_count: nodes.len() as u64,
        file_count,
        dir_count,
        total_size,
        total_allocated,
        nodes,
        warnings: Vec::new(),
    })
}

#[cfg(not(windows))]
pub fn scan_volume(_app: &AppHandle, _root: &str) -> Result<ScanResult> {
    bail!("NTFS MFT scanner is only available on Windows");
}

#[cfg(windows)]
fn enumerate_mft_records(
    app: &AppHandle,
    volume: &File,
    boot: &boot::NtfsBoot,
    mft: &MftStream,
    record_count: u64,
) -> Result<Vec<ParsedRecord>> {
    let mut parsed = Vec::with_capacity(record_count.min(1_000_000) as usize);

    let mut records_seen = 0u64;
    let mut stream_bytes_read = 0u64;
    let mut pending = Vec::<u8>::new();
    let mut last_emit = Instant::now();

    for run in &mft.runs {
        let run_bytes = run.clusters.saturating_mul(boot.cluster_size);
        let remaining_mft_bytes = mft.data_size.saturating_sub(stream_bytes_read);

        if remaining_mft_bytes == 0 {
            break;
        }

        let bytes_to_process = run_bytes.min(remaining_mft_bytes);

        if run.lcn < 0 {
            let skipped_records = bytes_to_process / boot.file_record_size as u64;
            records_seen = records_seen.saturating_add(skipped_records);
            stream_bytes_read = stream_bytes_read.saturating_add(bytes_to_process);
            pending.clear();
            continue;
        }

        let disk_base = (run.lcn as u64)
            .checked_mul(boot.cluster_size)
            .context("MFT data run disk offset overflow")?;

        let mut run_offset = 0u64;

        while run_offset < bytes_to_process && records_seen < record_count {
            let read_len = (bytes_to_process - run_offset).min(MFT_READ_CHUNK_BYTES) as usize;

            let prefix_len = pending.len();
            let mut buf = vec![0u8; prefix_len + read_len];

            if prefix_len > 0 {
                buf[..prefix_len].copy_from_slice(&pending);
            }

            read_exact_at(
                volume,
                disk_base + run_offset,
                &mut buf[prefix_len..prefix_len + read_len],
            )?;

            run_offset += read_len as u64;
            stream_bytes_read = stream_bytes_read.saturating_add(read_len as u64);

            let complete_len = (buf.len() / boot.file_record_size) * boot.file_record_size;

            for record_buf in buf[..complete_len].chunks_mut(boot.file_record_size) {
                if records_seen >= record_count {
                    break;
                }

                if let Some(record) = record::parse_user_file_record(
                    records_seen,
                    record_buf,
                    boot.bytes_per_sector as usize,
                ) {
                    parsed.push(record);
                }

                records_seen += 1;
            }

            pending.clear();
            pending.extend_from_slice(&buf[complete_len..]);

            if last_emit.elapsed().as_millis() > 220 {
                emit_progress(
                    app,
                    "ntfs.mft",
                    records_seen,
                    Some(record_count),
                    format!(
                        "正在流式读取 MFT: {} / {} records",
                        records_seen, record_count
                    ),
                );
                last_emit = Instant::now();
            }
        }
    }

    Ok(parsed)
}

fn build_tree_from_records(root: &str, records: Vec<ParsedRecord>) -> Vec<crate::models::NodeDto> {
    let filtered: Vec<ParsedRecord> = records
        .into_iter()
        .filter(|record| record.record_number != 5 && record.name != ".")
        .collect();

    let mut nodes = Vec::<TreeNode>::with_capacity(filtered.len() + 1);
    nodes.push(TreeNode::root(root.to_string()));

    let mut frn_to_node_id = HashMap::<u64, u32>::with_capacity(filtered.len());

    for record in &filtered {
        let id = nodes.len() as u32;
        frn_to_node_id.insert(record.record_number, id);

        nodes.push(TreeNode::new(
            id,
            record.name.clone(),
            None,
            record.is_dir,
            record.size,
            record.allocated,
        ));
    }

    for (index, record) in filtered.iter().enumerate() {
        let id = (index + 1) as u32;

        let mut parent_id = if record.parent_record == 5 {
            0
        } else {
            frn_to_node_id
                .get(&record.parent_record)
                .copied()
                .unwrap_or(0)
        };

        if parent_id == id {
            parent_id = 0;
        }

        if parent_id != 0 && !nodes[parent_id as usize].is_dir {
            parent_id = 0;
        }

        nodes[id as usize].parent = Some(parent_id);
        nodes[parent_id as usize].children.push(id);
    }

    finalize_tree(nodes)
}

#[cfg(windows)]
fn open_volume(device_path: &str) -> Result<File> {
    OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_SEQUENTIAL_SCAN)
        .open(device_path)
        .with_context(|| {
            format!(
                "无法打开卷 {device_path}。NTFS MFT 原始读取通常需要管理员权限，请以管理员身份运行。"
            )
        })
}

#[cfg(windows)]
fn read_exact_at(file: &File, offset: u64, buf: &mut [u8]) -> std::io::Result<()> {
    let mut done = 0usize;

    while done < buf.len() {
        let read = file.seek_read(&mut buf[done..], offset + done as u64)?;

        if read == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "unexpected EOF while reading volume",
            ));
        }

        done += read;
    }

    Ok(())
}

fn drive_root_from_input(root: &str) -> Result<String> {
    let trimmed = root.trim();

    if let Some(rest) = trimmed.strip_prefix(r"\\.\") {
        if rest.len() >= 2 && rest.as_bytes()[1] == b':' {
            let drive = rest.chars().next().unwrap().to_ascii_uppercase();
            return Ok(format!("{drive}:\\"));
        }
    }

    if trimmed.len() >= 2 && trimmed.as_bytes()[1] == b':' {
        let drive = trimmed.chars().next().unwrap().to_ascii_uppercase();
        return Ok(format!("{drive}:\\"));
    }

    bail!("NTFS MFT 模式只支持 Windows 盘符根路径，例如 C:\\");
}

fn volume_device_path(root: &str) -> Result<String> {
    if root.len() >= 2 && root.as_bytes()[1] == b':' {
        let drive = root.chars().next().unwrap().to_ascii_uppercase();
        Ok(format!(r"\\.\{}:", drive))
    } else {
        bail!("invalid Windows drive root: {root}");
    }
}
