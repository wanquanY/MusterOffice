use crate::boxes::{self, BoxData, full, optional, required, u32be, u64be};
use crate::{Budget, ByteRange, IndexError as E, Result, Sample, reserve};

pub(crate) fn table(
    data: &[u8],
    width: usize,
    maximum: usize,
    versions: &[u8],
) -> Result<(u8, usize)> {
    let version = full(data, versions)?;
    let count = u32be(data, 4)? as usize;
    if count > maximum {
        return Err(E::Budget);
    }
    if count.checked_mul(width).and_then(|n| n.checked_add(8)) != Some(data.len()) {
        return Err(E::Invalid("table entry count/length"));
    }
    Ok((version, count))
}
enum Sizes<'a> {
    Standard {
        count: usize,
        fixed: u32,
        data: &'a [u8],
    },
    Compact {
        count: usize,
        bits: u8,
        data: &'a [u8],
    },
}
impl Sizes<'_> {
    fn count(&self) -> usize {
        match self {
            Self::Standard { count, .. } | Self::Compact { count, .. } => *count,
        }
    }
    fn at(&self, index: usize) -> Result<u32> {
        match self {
            Self::Standard { fixed, data, .. } => {
                if *fixed != 0 {
                    Ok(*fixed)
                } else {
                    u32be(data, 12 + index * 4)
                }
            }
            Self::Compact { bits: 4, data, .. } => Ok(u32::from(if index.is_multiple_of(2) {
                data[12 + index / 2] >> 4
            } else {
                data[12 + index / 2] & 15
            })),
            Self::Compact { bits: 8, data, .. } => Ok(u32::from(data[12 + index])),
            Self::Compact { bits: 16, data, .. } => {
                Ok(u32::from(boxes::u16be(data, 12 + index * 2)?))
            }
            _ => Err(E::Invalid("sample size encoding")),
        }
    }
}
fn sizes<'a>(children: &[BoxData<'a>], budget: &mut Budget<'_>) -> Result<Sizes<'a>> {
    let standard = optional(children, b"stsz")?;
    let compact = optional(children, b"stz2")?;
    let result = match (standard, compact) {
        (Some(data), None) => {
            full(data, &[0])?;
            let fixed = u32be(data, 4)?;
            let count = u32be(data, 8)? as usize;
            if (if fixed == 0 {
                count.checked_mul(4).and_then(|n| n.checked_add(12))
            } else {
                Some(12)
            }) != Some(data.len())
            {
                return Err(E::Invalid("sample sizes length"));
            }
            Sizes::Standard { count, fixed, data }
        }
        (None, Some(data)) => {
            full(data, &[0])?;
            if data.get(4..7) != Some(&[0, 0, 0]) {
                return Err(E::Invalid("compact sizes reserved bits"));
            }
            let bits = *data.get(7).ok_or(E::Invalid("short compact sizes"))?;
            if !matches!(bits, 4 | 8 | 16) {
                return Err(E::Invalid("compact size width"));
            }
            let count = u32be(data, 8)? as usize;
            let bytes = count
                .checked_mul(bits as usize)
                .map(|n| n.div_ceil(8))
                .and_then(|n| n.checked_add(12));
            if bytes != Some(data.len()) {
                return Err(E::Invalid("compact sizes length"));
            }
            Sizes::Compact { count, bits, data }
        }
        _ => return Err(E::Invalid("exactly one sample size table required")),
    };
    budget.samples = budget
        .samples
        .checked_add(result.count())
        .ok_or(E::Budget)?;
    if budget.samples > budget.limits.max_samples {
        return Err(E::Budget);
    }
    Ok(result)
}

pub(crate) fn samples(
    children: &[BoxData<'_>],
    description_count: usize,
    media: &[ByteRange],
    budget: &mut Budget<'_>,
) -> Result<(Vec<Sample>, u64)> {
    let sizes = sizes(children, budget)?;
    let count = sizes.count();
    let stts = required(children, b"stts")?;
    let (_, runs) = table(stts, 8, count, &[0])?;
    let mut out = Vec::new();
    reserve(&mut out, count)?;
    let mut dts = 0i64;
    for run in 0..runs {
        budget.active()?;
        let n = u32be(stts, 8 + run * 8)? as usize;
        let duration = u32be(stts, 12 + run * 8)?;
        if n == 0 || duration == 0 || n > count - out.len() {
            return Err(E::Invalid("decode time run"));
        }
        for _ in 0..n {
            budget.active()?;
            out.push(Sample {
                bytes: ByteRange {
                    offset: 0,
                    length: 0,
                },
                dts,
                pts: dts,
                decode_duration: duration,
                description: 0,
                sync_sample: true,
            });
            dts = dts
                .checked_add(i64::from(duration))
                .ok_or(E::Invalid("decode timestamp overflow"))?;
        }
    }
    if out.len() != count {
        return Err(E::Invalid("decode sample count differs"));
    }
    if let Some(ctts) = optional(children, b"ctts")? {
        let (version, runs) = table(ctts, 8, count, &[0, 1])?;
        let mut at = 0;
        for run in 0..runs {
            budget.active()?;
            let n = u32be(ctts, 8 + run * 8)? as usize;
            let offset = u32be(ctts, 12 + run * 8)?;
            let offset = if version == 0 {
                i64::from(offset)
            } else {
                i64::from(offset as i32)
            };
            if n == 0 || n > count - at {
                return Err(E::Invalid("composition time run"));
            }
            for sample in &mut out[at..at + n] {
                budget.active()?;
                sample.pts = sample
                    .dts
                    .checked_add(offset)
                    .ok_or(E::Invalid("composition timestamp overflow"))?;
            }
            at += n;
        }
        if at != count {
            return Err(E::Invalid("composition sample count differs"));
        }
    }
    if let Some(stss) = optional(children, b"stss")? {
        let (_, n) = table(stss, 4, count, &[0])?;
        for sample in &mut out {
            budget.active()?;
            sample.sync_sample = false;
        }
        let mut previous = 0;
        for i in 0..n {
            budget.active()?;
            let index = u32be(stss, 8 + i * 4)? as usize;
            if index <= previous || index > count {
                return Err(E::Invalid("sync sample order/range"));
            }
            out[index - 1].sync_sample = true;
            previous = index;
        }
    }
    let (chunks, width) = match (optional(children, b"stco")?, optional(children, b"co64")?) {
        (Some(v), None) => (v, 4),
        (None, Some(v)) => (v, 8),
        _ => return Err(E::Invalid("exactly one chunk offset table required")),
    };
    let (_, chunk_count) = table(chunks, width, count, &[0])?;
    let stsc = required(children, b"stsc")?;
    let (_, maps) = table(stsc, 12, chunk_count, &[0])?;
    if (maps == 0) != (count == 0) || (chunk_count == 0) != (count == 0) {
        return Err(E::Invalid("empty chunk mapping"));
    }
    let mut previous = 0;
    for i in 0..maps {
        budget.active()?;
        let first = u32be(stsc, 8 + i * 12)? as usize;
        let n = u32be(stsc, 12 + i * 12)? as usize;
        let description = u32be(stsc, 16 + i * 12)? as usize;
        if first <= previous
            || (i == 0 && first != 1)
            || first > chunk_count
            || n == 0
            || n > count
            || description == 0
            || description > description_count
        {
            return Err(E::Invalid("sample-to-chunk mapping"));
        }
        previous = first;
    }
    let mut sample_index = 0;
    let mut map = 0;
    for chunk in 0..chunk_count {
        budget.active()?;
        if map + 1 < maps && u32be(stsc, 8 + (map + 1) * 12)? as usize == chunk + 1 {
            map += 1;
        }
        let n = u32be(stsc, 12 + map * 12)? as usize;
        let description = u32be(stsc, 16 + map * 12)? - 1;
        if n > count - sample_index {
            return Err(E::Invalid("chunk has too many samples"));
        }
        let mut offset = if width == 4 {
            u64::from(u32be(chunks, 8 + chunk * 4)?)
        } else {
            u64be(chunks, 8 + chunk * 8)?
        };
        // Top-level mdat ranges are already ordered and cannot overlap. Binary
        // search once per chunk; never scan every mdat for every sample.
        let data_index = media
            .partition_point(|range| range.offset <= offset)
            .checked_sub(1)
            .ok_or(E::Invalid("chunk outside media data"))?;
        let range = media[data_index];
        let range_end = range.offset + range.length;
        for (index, sample) in out.iter_mut().enumerate().skip(sample_index).take(n) {
            budget.active()?;
            let length = u64::from(sizes.at(index)?);
            if length == 0 {
                return Err(E::Unsupported("empty encoded sample"));
            }
            let end = offset
                .checked_add(length)
                .filter(|end| *end <= range_end)
                .ok_or(E::Invalid("sample outside media data"))?;
            sample.bytes = ByteRange { offset, length };
            sample.description = description;
            offset = end;
        }
        sample_index += n;
    }
    if sample_index != count {
        return Err(E::Invalid("unmapped samples"));
    }
    Ok((out, dts as u64))
}
