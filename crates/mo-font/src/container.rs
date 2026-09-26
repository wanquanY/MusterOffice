//! Selected SFNT directory bounds and checksums, before semantic table parsing.
use super::{FontError, FontLimits, FontTable, cancelled, invalid};
use mo_common::ByteLength;

pub(super) struct Container {
    pub face_count: u32,
    pub version: u32,
    pub tables: Vec<FontTable>,
}
pub(super) fn u16_at(data: &[u8], offset: usize) -> Result<u16, FontError> {
    Ok(u16::from_be_bytes(
        data.get(offset..)
            .and_then(|tail| tail.get(..2))
            .ok_or_else(|| invalid("truncated font header"))?
            .try_into()
            .unwrap(),
    ))
}
pub(super) fn u32_at(data: &[u8], offset: usize) -> Result<u32, FontError> {
    Ok(u32::from_be_bytes(
        data.get(offset..)
            .and_then(|tail| tail.get(..4))
            .ok_or_else(|| invalid("truncated font header"))?
            .try_into()
            .unwrap(),
    ))
}
fn sum(data: &[u8], head: bool, check: &dyn Fn() -> bool) -> Result<u32, FontError> {
    let mut result = 0_u32;
    for (i, block) in data.chunks(4).enumerate() {
        if i % 16384 == 0 {
            cancelled(check)?;
        }
        if head && i == 2 {
            continue;
        }
        let mut word = [0; 4];
        word[..block.len()].copy_from_slice(block);
        result = result.wrapping_add(u32::from_be_bytes(word));
    }
    Ok(result)
}
pub(super) fn valid_tag(tag: [u8; 4]) -> bool {
    tag[0] != b' '
        && tag.iter().all(|v| (0x20..=0x7e).contains(v))
        && tag
            .iter()
            .position(|v| *v == b' ')
            .is_none_or(|i| tag[i..].iter().all(|v| *v == b' '))
}
pub(super) fn inspect(
    data: &[u8],
    face: u32,
    limits: FontLimits,
    check: &dyn Fn() -> bool,
) -> Result<Container, FontError> {
    let signature = u32_at(data, 0)?;
    let collection = signature == u32::from_be_bytes(*b"ttcf");
    let mut protected = Vec::new();
    let (count, directory, header_end) = if collection {
        let version = u32_at(data, 4)?;
        if ![0x00010000, 0x00020000].contains(&version) {
            return Err(FontError::Unsupported("TTC version"));
        }
        let count = u32_at(data, 8)?;
        if count == 0 {
            return Err(invalid("empty font collection"));
        }
        if count as usize > limits.max_faces {
            return Err(FontError::Limit("font faces"));
        }
        if count as usize > data.len().saturating_sub(12) / 4 {
            return Err(invalid("truncated collection directory"));
        }
        let end = 12 + count as usize * 4 + if version == 0x00020000 { 12 } else { 0 };
        if end > data.len() {
            return Err(invalid("truncated collection directory"));
        }
        for i in 0..count as usize {
            cancelled(check)?;
            let offset = u32_at(data, 12 + i * 4)? as usize;
            if offset < end
                || !offset.is_multiple_of(4)
                || offset.checked_add(12).is_none_or(|v| v > data.len())
            {
                return Err(invalid("font face directory range"));
            }
        }
        for i in 0..count as usize {
            let start = u32_at(data, 12 + i * 4)? as usize;
            let num = u16_at(data, start + 4)? as usize;
            if num == 0 || num > limits.max_tables {
                return Err(FontError::Limit("collection face tables"));
            }
            let stop = start + 12 + num * 16;
            if stop > data.len() {
                return Err(invalid("collection face directory range"));
            }
            protected.push((start, stop));
        }
        if version == 0x00020000 {
            let at = 12 + count as usize * 4;
            let tag = u32_at(data, at)?;
            let length = u32_at(data, at + 4)? as usize;
            let start = u32_at(data, at + 8)? as usize;
            if tag != 0 || length != 0 || start != 0 {
                let segment = data.get(start..).and_then(|tail| tail.get(..length));
                if segment.is_none() {
                    return Err(invalid("TTC DSIG descriptor"));
                }
                let stop = start + length;
                if tag != u32::from_be_bytes(*b"DSIG")
                    || start < end
                    || length == 0
                    || stop > data.len()
                {
                    return Err(invalid("TTC DSIG descriptor"));
                }
                protected.push((start, stop));
            }
        }
        protected.sort_unstable();
        protected.dedup();
        if protected.windows(2).any(|v| v[0].1 > v[1].0) {
            return Err(invalid("overlapping collection directories"));
        }
        if face >= count {
            return Err(invalid("face index outside collection"));
        }
        (count, u32_at(data, 12 + face as usize * 4)? as usize, end)
    } else {
        if face != 0 {
            return Err(invalid("face index outside single font"));
        }
        (1, 0, 0)
    };
    if count as usize > limits.max_faces {
        return Err(FontError::Limit("font faces"));
    }
    let version = u32_at(data, directory)?;
    if ![0x00010000, u32::from_be_bytes(*b"OTTO")].contains(&version) {
        return Err(FontError::Unsupported("font container or SFNT version"));
    }
    let num = u16_at(data, directory + 4)? as usize;
    if num == 0 {
        return Err(invalid("empty SFNT directory"));
    }
    if num > limits.max_tables {
        return Err(FontError::Limit("font tables"));
    }
    let end = directory + 12 + num * 16;
    if end > data.len() {
        return Err(invalid("truncated SFNT directory"));
    }
    let mut tables = Vec::with_capacity(num);
    let mut ranges = Vec::with_capacity(num);
    let mut previous = [0_u8; 4];
    for i in 0..num {
        cancelled(check)?;
        let record = directory + 12 + i * 16;
        let tag: [u8; 4] = data[record..record + 4].try_into().unwrap();
        if tag <= previous || !valid_tag(tag) {
            return Err(invalid("invalid, duplicate or unsorted table tag"));
        }
        previous = tag;
        let offset = u32_at(data, record + 8)? as usize;
        let length = u32_at(data, record + 12)? as usize;
        if data
            .get(offset..)
            .and_then(|tail| tail.get(..length))
            .is_none()
        {
            return Err(invalid("table outside data or overlaps directory"));
        }
        let table_end = offset + length;
        if offset < header_end
            || !offset.is_multiple_of(4)
            || table_end > data.len()
            || (offset < end && table_end > directory)
            || protected.iter().any(|(a, b)| offset < *b && table_end > *a)
        {
            return Err(invalid("table outside data or overlaps directory"));
        }
        if length != 0 {
            ranges.push((offset, table_end));
        }
        let checksum = u32_at(data, record + 4)?;
        tables.push(FontTable {
            tag: String::from_utf8(tag.to_vec()).unwrap(),
            offset: ByteLength::new(offset as u64),
            byte_length: ByteLength::new(length as u64),
            checksum,
        });
    }
    ranges.sort_unstable();
    if ranges.windows(2).any(|v| v[0].1 > v[1].0) {
        return Err(invalid("overlapping font tables"));
    }
    for table in &tables {
        let start = table.offset.get() as usize;
        let end = start + table.byte_length.get() as usize;
        if sum(&data[start..end], table.tag == "head", check)? != table.checksum {
            return Err(invalid("font table checksum mismatch"));
        }
    }
    if !collection && sum(data, false, check)? != 0xB1B0AFBA {
        return Err(invalid("font checksum adjustment mismatch"));
    }
    Ok(Container {
        face_count: count,
        version,
        tables,
    })
}
