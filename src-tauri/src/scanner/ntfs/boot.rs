use anyhow::{bail, Context, Result};

#[derive(Debug, Clone)]
pub struct NtfsBoot {
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub cluster_size: u64,
    pub total_sectors: u64,
    pub mft_lcn: u64,
    pub file_record_size: usize,
}

pub fn parse_boot_sector(buf: &[u8]) -> Result<NtfsBoot> {
    if buf.len() < 512 {
        bail!("boot sector buffer too small");
    }

    if buf.get(3..11) != Some(b"NTFS    ") {
        bail!("目标卷不是 NTFS 文件系统");
    }

    let bytes_per_sector = read_u16(buf, 11).context("invalid bytes per sector")?;
    let sectors_per_cluster = *buf.get(13).context("invalid sectors per cluster")?;
    let cluster_size = bytes_per_sector as u64 * sectors_per_cluster as u64;

    if bytes_per_sector == 0 || sectors_per_cluster == 0 || cluster_size == 0 {
        bail!("invalid NTFS cluster geometry");
    }

    let total_sectors = read_u64(buf, 40).context("invalid total sectors")?;
    let mft_lcn = read_u64(buf, 48).context("invalid MFT LCN")?;
    let clusters_per_file_record = *buf.get(64).context("invalid file record size")? as i8;

    if clusters_per_file_record == 0 {
        bail!("invalid clusters per file record");
    }

    let file_record_size = if clusters_per_file_record > 0 {
        cluster_size
            .checked_mul(clusters_per_file_record as u64)
            .context("file record size overflow")?
    } else {
        let exponent = (-(clusters_per_file_record as i16)) as u32;
        1u64.checked_shl(exponent)
            .context("file record size shift overflow")?
    };

    if !(512..=1024 * 1024).contains(&file_record_size) {
        bail!("unsupported NTFS FILE record size: {file_record_size}");
    }

    Ok(NtfsBoot {
        bytes_per_sector,
        sectors_per_cluster,
        cluster_size,
        total_sectors,
        mft_lcn,
        file_record_size: file_record_size as usize,
    })
}

fn read_u16(buf: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(buf.get(offset..offset + 2)?.try_into().ok()?))
}

fn read_u64(buf: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_le_bytes(buf.get(offset..offset + 8)?.try_into().ok()?))
}
