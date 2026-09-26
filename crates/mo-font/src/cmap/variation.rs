use super::*;
fn u24(data: &[u8], at: usize) -> Result<u32, FontError> {
    let v = data
        .get(at..at + 3)
        .ok_or_else(|| invalid("cmap variation scalar"))?;
    Ok(u32::from_be_bytes([0, v[0], v[1], v[2]]))
}
pub(super) fn validate(
    data: &[u8],
    glyphs: u16,
    check: &dyn Fn() -> bool,
) -> Result<(), FontError> {
    let count = u32r(data, 6)? as usize;
    if count > data.len().saturating_sub(10) / 11 {
        return Err(invalid("cmap variation record length"));
    }
    let header_end = 10 + count * 11;
    let mut previous = None;
    // Selector domain is small; cap validation work when offsets share ranges.
    let mut budget = data.len().saturating_mul(2);
    for i in 0..count {
        cancelled(check)?;
        let at = 10 + i * 11;
        let selector = u24(data, at)?;
        if !valid_selector(selector) || previous.is_some_and(|p| selector <= p) {
            return Err(invalid("cmap variation selector order"));
        }
        previous = Some(selector);
        let default = u32r(data, at + 3)? as usize;
        let nondefault = u32r(data, at + 7)? as usize;
        let mut ranges = Vec::new();
        for (offset, stride) in [(default, 4), (nondefault, 5)] {
            if offset == 0 {
                continue;
            }
            if offset < header_end {
                return Err(invalid("cmap variation data overlaps records"));
            }
            let n = u32r(data, offset)? as usize;
            if n > data.len().saturating_sub(offset + 4) / stride {
                return Err(invalid("cmap variation mapping length"));
            }
            budget = budget
                .checked_sub(n * stride)
                .ok_or(FontError::Limit("variation cmap validation work"))?;
            let mut last = None;
            for j in 0..n {
                cancelled(check)?;
                let pos = offset + 4 + j * stride;
                let start = u24(data, pos)?;
                let end = if stride == 4 {
                    start + data[pos + 3] as u32
                } else {
                    start
                };
                if end > 0x10ffff || last.is_some_and(|p| start <= p) {
                    return Err(invalid("cmap variation scalar order"));
                }
                last = Some(end);
                if stride == 4 {
                    ranges.push((start, end));
                } else {
                    glyph(u16r(data, pos + 3)? as u32, glyphs)?;
                    let k = ranges.partition_point(|(_, end)| *end < start);
                    if ranges
                        .get(k)
                        .is_some_and(|(a, b)| *a <= start && start <= *b)
                    {
                        return Err(invalid("default and nondefault variation overlap"));
                    }
                }
            }
        }
    }
    Ok(())
}
