use anyhow::Result;

use crate::models::{ScanMode, ScanOptions, ScanResult};
use crate::progress::ScanContext;

pub mod ntfs;
pub mod path_walk;
pub mod tree;

pub fn scan(ctx: &ScanContext, options: ScanOptions) -> Result<ScanResult> {
    match options.mode {
        ScanMode::Walk => path_walk::scan_path(ctx, &options.root),
        ScanMode::NtfsMft => ntfs::scan_volume(ctx, &options.root),
        ScanMode::Auto => {
            #[cfg(windows)]
            {
                if looks_like_windows_drive_root(&options.root) {
                    match ntfs::scan_volume(ctx, &options.root) {
                        Ok(result) => return Ok(result),
                        Err(error) => {
                            let mut fallback = path_walk::scan_path(ctx, &options.root)?;
                            fallback.warnings.insert(
                                0,
                                format!("NTFS MFT 快速模式不可用，已回退到兼容递归扫描。原因: {error}"),
                            );
                            return Ok(fallback);
                        }
                    }
                }
            }
            path_walk::scan_path(ctx, &options.root)
        }
    }
}

#[cfg(windows)]
fn looks_like_windows_drive_root(root: &str) -> bool {
    let trimmed = root.trim();
    trimmed.len() >= 2 && trimmed.as_bytes()[1] == b':'
}
