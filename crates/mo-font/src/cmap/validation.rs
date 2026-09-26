//! Validate the selected map once, so a parser's failed lookup cannot masquerade as a missing glyph.
use super::*;
pub(super) fn validate(
    data: &[u8],
    format: u16,
    glyphs: u16,
    check: &dyn Fn() -> bool,
) -> Result<(), FontError> {
    let language = if format < 10 {
        u16r(data, 4)? as u32
    } else {
        u32r(data, 8)?
    };
    if language != 0 {
        return Err(invalid("Unicode cmap language must be zero"));
    }
    if format >= 10 && u16r(data, 2)? != 0 {
        return Err(invalid("cmap reserved field"));
    }
    match format {
        0 => {
            if data.len() != 262 {
                return Err(invalid("cmap format 0 length"));
            }
            for id in &data[6..] {
                glyph(*id as u32, glyphs)?;
            }
        }
        4 => format4(data, glyphs, check)?,
        6 | 10 => {
            let (start, count, header) = if format == 6 {
                (u16r(data, 6)? as u32, u16r(data, 8)? as usize, 10)
            } else {
                (u32r(data, 12)?, u32r(data, 16)? as usize, 20)
            };
            if count > data.len().saturating_sub(header) / 2
                || data.len() != header + count * 2
                || start
                    .checked_add(count as u32)
                    .is_none_or(|end| end > if format == 6 { 0x10000 } else { 0x110000 })
            {
                return Err(invalid("cmap trimmed array range"));
            }
            for i in 0..count {
                cancelled(check)?;
                glyph(u16r(data, header + i * 2)? as u32, glyphs)?;
            }
        }
        12 | 13 => {
            let count = u32r(data, 12)? as usize;
            if count > data.len().saturating_sub(16) / 12 || data.len() != 16 + count * 12 {
                return Err(invalid("cmap group length"));
            }
            let mut previous = None;
            for i in 0..count {
                cancelled(check)?;
                let at = 16 + i * 12;
                let start = u32r(data, at)?;
                let end = u32r(data, at + 4)?;
                let id = u32r(data, at + 8)?;
                if start > end || end > 0x10ffff || previous.is_some_and(|p| start <= p) {
                    return Err(invalid("cmap group order or scalar range"));
                }
                let last = if format == 12 {
                    id.checked_add(end - start)
                        .ok_or_else(|| invalid("cmap glyph overflow"))?
                } else {
                    id
                };
                glyph(last, glyphs)?;
                previous = Some(end);
            }
        }
        _ => return Err(FontError::Unsupported("Unicode cmap format")),
    }
    Ok(())
}
fn format4(data: &[u8], glyphs: u16, check: &dyn Fn() -> bool) -> Result<(), FontError> {
    let twice = u16r(data, 6)? as usize;
    let count = twice / 2;
    if count == 0
        || !twice.is_multiple_of(2)
        || data.len() < 16 + count * 8
        || !data.len().is_multiple_of(2)
    {
        return Err(invalid("cmap format 4 segment lengths"));
    }
    if u16r(data, 14 + count * 2)? != 0 {
        return Err(invalid("cmap format 4 reserved pad"));
    }
    let mut previous = None;
    for i in 0..count {
        cancelled(check)?;
        let end = u16r(data, 14 + i * 2)?;
        let start = u16r(data, 16 + count * 2 + i * 2)?;
        let delta = u16r(data, 16 + count * 4 + i * 2)?;
        let pointer = 16 + count * 6 + i * 2;
        let offset = u16r(data, pointer)? as usize;
        if start > end || previous.is_some_and(|p| start <= p) {
            return Err(invalid("cmap format 4 segment order"));
        }
        if offset != 0
            && (!offset.is_multiple_of(2)
                || pointer + offset < 16 + count * 8
                || pointer + offset + (end - start) as usize * 2 + 2 > data.len())
        {
            return Err(invalid("cmap format 4 glyph array range"));
        }
        for cp in start..=end {
            if cp % 256 == 0 {
                cancelled(check)?;
            }
            let id = if offset == 0 {
                cp.wrapping_add(delta)
            } else {
                let raw = u16r(data, pointer + offset + (cp - start) as usize * 2)?;
                if raw == 0 { 0 } else { raw.wrapping_add(delta) }
            };
            glyph(id as u32, glyphs)?;
        }
        previous = Some(end);
    }
    if previous != Some(0xffff) {
        return Err(invalid("cmap format 4 terminal segment"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use read_fonts::{FontData, FontRead, tables::cmap::Cmap10};
    #[test]
    fn format10_valid_array_and_invalid_glyph_are_distinct() {
        let mut b = Vec::new();
        for word in [0x000a0000_u32, 24, 0, 0x10000, 2] {
            b.extend_from_slice(&word.to_be_bytes());
        }
        b.extend_from_slice(&[0, 2, 0, 5]);
        validate(&b, 10, 7, &|| false).unwrap();
        let t = Cmap10::read(FontData::new(&b)).unwrap();
        assert_eq!(t.map_codepoint(0x10000_u32).unwrap().to_u32(), 2);
        assert!(t.map_codepoint(0xffff_u32).is_none());
        b[23] = 7;
        assert!(matches!(
            validate(&b, 10, 7, &|| false),
            Err(FontError::Invalid("cmap glyph outside glyph set"))
        ));
    }
}
