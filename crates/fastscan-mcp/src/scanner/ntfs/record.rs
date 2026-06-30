use anyhow::{bail, Context, Result};

use super::runs::{parse_mapping_pairs, DataRun};

const ATTR_TYPE_FILE_NAME: u32 = 0x30;
const ATTR_TYPE_DATA: u32 = 0x80;
const ATTR_TYPE_END: u32 = 0xffff_ffff;
const FILE_RECORD_IN_USE: u16 = 0x0001;
const FILE_RECORD_IS_DIRECTORY: u16 = 0x0002;
const FRN_MASK: u64 = 0x0000_ffff_ffff_ffff;
const ATTR_FLAG_COMPRESSED: u16 = 0x0001;
const ATTR_FLAG_SPARSE: u16 = 0x8000;

#[derive(Debug, Clone)]
pub struct MftStream {
    pub runs: Vec<DataRun>,
    pub allocated_size: u64,
    pub data_size: u64,
}

#[derive(Debug, Clone)]
pub struct ParsedRecord {
    pub record_number: u64,
    pub parent_record: u64,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub allocated: u64,
    pub created: i64,
    pub modified: i64,
    pub accessed: i64,
}

#[derive(Debug, Clone)]
struct NameCandidate {
    parent_record: u64,
    namespace: u8,
    name: String,
    created: i64,
    modified: i64,
    accessed: i64,
}

pub fn parse_mft_stream(record: &mut [u8], bytes_per_sector: usize) -> Result<MftStream> {
    apply_update_sequence_array(record, bytes_per_sector)?;
    let first_attr_offset = read_u16(record, 20).context("invalid first attribute offset")? as usize;
    let mut offset = first_attr_offset;

    while offset + 16 <= record.len() {
        let attr_type = read_u32(record, offset).context("invalid attribute type")?;
        if attr_type == ATTR_TYPE_END { break; }
        let attr_len = read_u32(record, offset + 4).context("invalid attribute length")? as usize;
        if attr_len < 16 || offset + attr_len > record.len() { bail!("corrupted MFT attribute"); }
        let attr = &record[offset..offset + attr_len];

        if attr_type == ATTR_TYPE_DATA && attr.get(8).copied() == Some(1) && attr.get(9).copied() == Some(0) {
            let mapping_offset = read_u16(attr, 32).context("invalid mapping pairs offset")? as usize;
            let allocated_size = read_u64(attr, 40).context("invalid MFT allocated size")?;
            let data_size = read_u64(attr, 48).context("invalid MFT data size")?;
            let mapping_pairs = attr.get(mapping_offset..).context("mapping pairs out of bounds")?;
            let runs = parse_mapping_pairs(mapping_pairs)?;
            if runs.is_empty() { bail!("MFT DATA attribute has no data runs"); }
            return Ok(MftStream { runs, allocated_size, data_size });
        }
        offset += attr_len;
    }
    bail!("未找到 $MFT 的非驻留 unnamed $DATA 属性")
}

pub fn parse_user_file_record(record_number: u64, record: &mut [u8], bytes_per_sector: usize) -> Option<ParsedRecord> {
    if record.len() < 48 || record.get(0..4) != Some(b"FILE") { return None; }
    if apply_update_sequence_array(record, bytes_per_sector).is_err() { return None; }
    let flags = read_u16(record, 22)?;
    if flags & FILE_RECORD_IN_USE == 0 { return None; }
    let base_file_record = read_u64(record, 32).unwrap_or(0) & FRN_MASK;
    if base_file_record != 0 { return None; }
    let is_dir = flags & FILE_RECORD_IS_DIRECTORY != 0;
    let first_attr_offset = read_u16(record, 20)? as usize;
    let mut offset = first_attr_offset;
    let mut names = Vec::<NameCandidate>::new();
    let mut size = 0u64;
    let mut allocated = 0u64;

    while offset + 16 <= record.len() {
        let attr_type = read_u32(record, offset)?;
        if attr_type == ATTR_TYPE_END { break; }
        let attr_len = read_u32(record, offset + 4)? as usize;
        if attr_len < 16 || offset + attr_len > record.len() { break; }
        let attr = &record[offset..offset + attr_len];

        match attr_type {
            ATTR_TYPE_FILE_NAME => { if let Some(name) = parse_file_name_attribute(attr) { names.push(name); } }
            ATTR_TYPE_DATA => { add_data_attribute_size(attr, &mut size, &mut allocated); }
            _ => {}
        }
        offset += attr_len;
    }

    let selected_name = choose_name(&names)?;
    if selected_name.name == "." { return None; }
    Some(ParsedRecord { record_number, parent_record: selected_name.parent_record, name: selected_name.name.clone(), is_dir, size, allocated, created: selected_name.created, modified: selected_name.modified, accessed: selected_name.accessed })
}

fn ntfs_time_to_unix(ntfs_time: u64) -> i64 {
    if ntfs_time == 0 { return 0; }
    (ntfs_time / 10000 / 1000) as i64 - 11644473600i64
}

fn apply_update_sequence_array(record: &mut [u8], bytes_per_sector: usize) -> Result<()> {
    if bytes_per_sector < 2 || record.len() < bytes_per_sector { bail!("invalid sector size for FILE record"); }
    if record.get(0..4) != Some(b"FILE") { bail!("not an NTFS FILE record"); }
    let usa_offset = read_u16(record, 4).context("invalid USA offset")? as usize;
    let usa_count = read_u16(record, 6).context("invalid USA count")? as usize;
    if usa_count == 0 || usa_offset + usa_count * 2 > record.len() { bail!("invalid USA range"); }
    let sector_count = record.len() / bytes_per_sector;
    if usa_count < sector_count + 1 { bail!("USA count smaller than sector count"); }
    let update_sequence_number = read_u16(record, usa_offset).context("invalid USN")?;
    for sector_index in 0..sector_count {
        let trailer_offset = (sector_index + 1) * bytes_per_sector - 2;
        let actual = read_u16(record, trailer_offset).context("invalid sector trailer")?;
        if actual != update_sequence_number { bail!("FILE record update sequence mismatch"); }
        let fixup_offset = usa_offset + 2 * (sector_index + 1);
        let fixup = read_u16(record, fixup_offset).context("invalid fixup")?.to_le_bytes();
        record[trailer_offset..trailer_offset + 2].copy_from_slice(&fixup);
    }
    Ok(())
}

fn parse_file_name_attribute(attr: &[u8]) -> Option<NameCandidate> {
    if attr.get(8).copied()? != 0 { return None; }
    let value_len = read_u32(attr, 16)? as usize;
    let value_offset = read_u16(attr, 20)? as usize;
    let value = attr.get(value_offset..value_offset + value_len)?;
    if value.len() < 66 { return None; }
    let parent_record = read_u64(value, 0)? & FRN_MASK;
    let created = ntfs_time_to_unix(read_u64(value, 8).unwrap_or(0));
    let modified = ntfs_time_to_unix(read_u64(value, 16).unwrap_or(0));
    let accessed = ntfs_time_to_unix(read_u64(value, 32).unwrap_or(0));
    let name_len = *value.get(64)? as usize;
    let namespace = *value.get(65)?;
    let name_bytes = value.get(66..66 + name_len * 2)?;
    let mut utf16 = Vec::with_capacity(name_len);
    for chunk in name_bytes.chunks_exact(2) { utf16.push(u16::from_le_bytes([chunk[0], chunk[1]])); }
    let name = String::from_utf16_lossy(&utf16);
    if name.is_empty() { return None; }
    Some(NameCandidate { parent_record, namespace, name, created, modified, accessed })
}

fn add_data_attribute_size(attr: &[u8], size: &mut u64, allocated: &mut u64) {
    let non_resident = attr.get(8).copied().unwrap_or(0) != 0;
    if !non_resident {
        if let Some(value_len) = read_u32(attr, 16) { *size = size.saturating_add(value_len as u64); }
        return;
    }
    let lowest_vcn = read_u64(attr, 16).unwrap_or(0);
    if lowest_vcn != 0 { return; }
    let attr_flags = read_u16(attr, 12).unwrap_or(0);
    let allocated_size = read_u64(attr, 40).unwrap_or(0);
    let data_size = read_u64(attr, 48).unwrap_or(0);
    let compressed_size = read_u64(attr, 64).unwrap_or(0);
    let physical_size = if attr_flags & (ATTR_FLAG_COMPRESSED | ATTR_FLAG_SPARSE) != 0 && compressed_size > 0 { compressed_size } else { allocated_size };
    *size = size.saturating_add(data_size);
    *allocated = allocated.saturating_add(physical_size);
}

fn choose_name(names: &[NameCandidate]) -> Option<&NameCandidate> {
    names.iter().min_by_key(|candidate| (namespace_rank(candidate.namespace), candidate.name.len(), candidate.name.to_ascii_lowercase()))
}

fn namespace_rank(namespace: u8) -> u8 {
    match namespace { 1 | 3 => 0, 0 => 1, 2 => 2, _ => 3 }
}

fn read_u16(buf: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(buf.get(offset..offset + 2)?.try_into().ok()?))
}
fn read_u32(buf: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(buf.get(offset..offset + 4)?.try_into().ok()?))
}
fn read_u64(buf: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_le_bytes(buf.get(offset..offset + 8)?.try_into().ok()?))
}
