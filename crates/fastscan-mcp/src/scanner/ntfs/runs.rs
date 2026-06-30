use anyhow::{bail, Context, Result};

#[derive(Debug, Clone)]
pub struct DataRun {
    pub lcn: i64,
    pub clusters: u64,
}

pub fn parse_mapping_pairs(buf: &[u8]) -> Result<Vec<DataRun>> {
    let mut runs = Vec::new();
    let mut offset = 0usize;
    let mut current_lcn = 0i64;

    loop {
        let header = *buf.get(offset).with_context(|| "unterminated NTFS mapping pairs")?;
        offset += 1;
        if header == 0 { break; }

        let length_size = (header & 0x0f) as usize;
        let offset_size = (header >> 4) as usize;
        if length_size == 0 || length_size > 8 || offset_size > 8 { bail!("invalid NTFS mapping pair header: {header:#x}"); }

        let length = read_unsigned(buf, offset, length_size).with_context(|| "invalid mapping pair length")?;
        offset += length_size;

        let lcn = if offset_size == 0 { -1 }
        else {
            let delta = read_signed(buf, offset, offset_size).with_context(|| "invalid mapping pair LCN delta")?;
            offset += offset_size;
            current_lcn = current_lcn.checked_add(delta).with_context(|| "LCN overflow")?;
            current_lcn
        };

        if length > 0 { runs.push(DataRun { lcn, clusters: length }); }
    }
    Ok(runs)
}

fn read_unsigned(buf: &[u8], offset: usize, size: usize) -> Option<u64> {
    let mut value = 0u64;
    for index in 0..size { value |= (*buf.get(offset + index)? as u64) << (index * 8); }
    Some(value)
}

fn read_signed(buf: &[u8], offset: usize, size: usize) -> Option<i64> {
    if size == 0 || size > 8 { return None; }
    let mut value = 0i64;
    for index in 0..size { value |= (*buf.get(offset + index)? as i64) << (index * 8); }
    let sign_bit = 1i64 << (size * 8 - 1);
    if value & sign_bit != 0 { value |= (!0i64) << (size * 8); }
    Some(value)
}
