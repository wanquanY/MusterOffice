use crate::{Budget, ByteRange, IndexError as E, Result, reserve};
use mo_media::ReadAt;

#[derive(Clone, Copy)]
pub(crate) struct BoxData<'a> {
    pub kind: [u8; 4],
    pub data: &'a [u8],
}

pub(crate) fn u16be(data: &[u8], at: usize) -> Result<u16> {
    Ok(u16::from_be_bytes(
        data.get(at..at + 2)
            .ok_or(E::Invalid("short field"))?
            .try_into()
            .map_err(|_| E::Invalid("field"))?,
    ))
}
pub(crate) fn u32be(data: &[u8], at: usize) -> Result<u32> {
    Ok(u32::from_be_bytes(
        data.get(at..at + 4)
            .ok_or(E::Invalid("short field"))?
            .try_into()
            .map_err(|_| E::Invalid("field"))?,
    ))
}
pub(crate) fn u64be(data: &[u8], at: usize) -> Result<u64> {
    Ok(u64::from_be_bytes(
        data.get(at..at + 8)
            .ok_or(E::Invalid("short field"))?
            .try_into()
            .map_err(|_| E::Invalid("field"))?,
    ))
}
pub(crate) fn fourcc(data: &[u8], at: usize) -> Result<[u8; 4]> {
    data.get(at..at + 4)
        .ok_or(E::Invalid("short type"))?
        .try_into()
        .map_err(|_| E::Invalid("type"))
}
pub(crate) fn full(data: &[u8], versions: &[u8]) -> Result<u8> {
    let version = *data.first().ok_or(E::Invalid("short full box"))?;
    if !versions.contains(&version) || data.get(1..4) != Some(&[0, 0, 0]) {
        return Err(E::Unsupported("box version or flags"));
    }
    Ok(version)
}
pub(crate) fn children<'a>(data: &'a [u8], budget: &mut Budget<'_>) -> Result<Vec<BoxData<'a>>> {
    let mut at = 0usize;
    let mut out = Vec::new();
    while at < data.len() {
        budget.box_read()?;
        let n = u32be(data, at)?;
        let kind = fourcc(data, at + 4)?;
        let (n, header) = match n {
            0 => return Err(E::Unsupported("nested zero-sized box")),
            1 => (
                usize::try_from(u64be(data, at + 8)?).map_err(|_| E::Budget)?,
                16,
            ),
            _ => (n as usize, 8),
        };
        let end = at
            .checked_add(n)
            .filter(|end| *end <= data.len())
            .ok_or(E::Invalid("box exceeds parent"))?;
        if n < header {
            return Err(E::Invalid("box length"));
        }
        reserve(&mut out, 1)?;
        out.push(BoxData {
            kind,
            data: &data[at + header..end],
        });
        at = end;
    }
    Ok(out)
}
pub(crate) fn optional<'a>(boxes: &[BoxData<'a>], kind: &[u8; 4]) -> Result<Option<&'a [u8]>> {
    let mut found = boxes.iter().filter(|b| &b.kind == kind);
    let first = found.next();
    if found.next().is_some() {
        return Err(E::Invalid("duplicate singleton box"));
    }
    Ok(first.map(|b| b.data))
}
pub(crate) fn required<'a>(boxes: &[BoxData<'a>], kind: &[u8; 4]) -> Result<&'a [u8]> {
    optional(boxes, kind)?.ok_or(E::Invalid("missing required box"))
}
fn exact(
    source: &mut dyn ReadAt,
    mut at: u64,
    mut out: &mut [u8],
    budget: &Budget<'_>,
    read: &mut u64,
) -> Result<()> {
    while !out.is_empty() {
        budget.active()?;
        let length = out.len().min(65536);
        let n = source.read_at(at, &mut out[..length])?;
        if n == 0 || n > length {
            return Err(E::Invalid("short or oversized source read"));
        }
        *read = read.checked_add(n as u64).ok_or(E::Budget)?;
        at = at.checked_add(n as u64).ok_or(E::Budget)?;
        out = &mut out[n..];
    }
    budget.active()
}
pub(crate) fn read_movie(
    source: &mut dyn ReadAt,
    length: u64,
    budget: &mut Budget<'_>,
) -> Result<(Vec<u8>, Vec<ByteRange>, u64)> {
    let mut at = 0;
    let mut movie = None;
    let mut media = Vec::new();
    let mut read = 0;
    while at < length {
        budget.box_read()?;
        if length - at < 8 {
            return Err(E::Invalid("trailing partial header"));
        }
        let mut header = [0u8; 16];
        exact(source, at, &mut header[..8], budget, &mut read)?;
        let size = u32be(&header, 0)?;
        let kind = fourcc(&header, 4)?;
        let (size, header_size) = match size {
            0 => (length - at, 8),
            1 => {
                if length - at < 16 {
                    return Err(E::Invalid("large header"));
                }
                exact(source, at + 8, &mut header[8..], budget, &mut read)?;
                (u64be(&header, 8)?, 16)
            }
            _ => (u64::from(size), 8),
        };
        let end = at
            .checked_add(size)
            .filter(|end| *end <= length)
            .ok_or(E::Invalid("box exceeds file"))?;
        if size < header_size {
            return Err(E::Invalid("top-level box length"));
        }
        let range = ByteRange {
            offset: at + header_size,
            length: size - header_size,
        };
        match &kind {
            b"moof" => return Err(E::Unsupported("fragmented media")),
            b"moov" => {
                if movie.is_some() {
                    return Err(E::Invalid("duplicate movie"));
                }
                let n = usize::try_from(range.length).map_err(|_| E::Budget)?;
                if n > budget.limits.max_metadata_bytes {
                    return Err(E::Budget);
                }
                let mut bytes = Vec::new();
                reserve(&mut bytes, n)?;
                bytes.resize(n, 0);
                exact(source, range.offset, &mut bytes, budget, &mut read)?;
                movie = Some(bytes);
            }
            b"mdat" => {
                reserve(&mut media, 1)?;
                media.push(range);
            }
            _ => {}
        }
        at = end;
    }
    Ok((movie.ok_or(E::Invalid("missing movie"))?, media, read))
}
