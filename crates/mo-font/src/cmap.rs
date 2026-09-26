//! Unicode-only selection. Symbol/Macintosh charmaps are never implicit substitutes.
mod validation;
mod variation;
use super::*;
use container::{u16_at as u16r, u32_at as u32r};
use read_fonts::tables::cmap::{CmapSubtable, MapVariant};

/// Borrowed tables are retained only after their full supported structures were checked.
pub(super) struct CheckedCmap<'a> {
    pub info: Option<FontCmap>,
    base: Option<CmapSubtable<'a>>,
    variation: Option<CmapSubtable<'a>>,
}
pub(super) fn inspect<'a>(
    font: &FontRef<'a>,
    glyphs: u16,
    check: &dyn Fn() -> bool,
) -> Result<CheckedCmap<'a>, FontError> {
    let Some(raw) = font.table_data(Tag::new(b"cmap")) else {
        return Ok(CheckedCmap {
            info: None,
            base: None,
            variation: None,
        });
    };
    let data = raw.as_bytes();
    if u16r(data, 0)? != 0 {
        return Err(FontError::Unsupported("cmap version"));
    }
    let count = u16r(data, 2)? as usize;
    let end = 4 + count * 8;
    if end > data.len() {
        return Err(invalid("cmap records"));
    }
    let priorities = [
        (3, 10),
        (0, 6),
        (0, 4),
        (3, 1),
        (0, 3),
        (0, 2),
        (0, 1),
        (0, 0),
    ];
    let mut selected: Option<(usize, usize)> = None;
    let mut uvs = None;
    let mut seen = BTreeSet::new();
    for i in 0..count {
        cancelled(check)?;
        let pair = (u16r(data, 4 + i * 8)?, u16r(data, 6 + i * 8)?);
        let rank = priorities.iter().position(|p| *p == pair);
        if rank.is_none() && pair != (0, 5) {
            continue;
        }
        if !seen.insert(pair) {
            return Err(invalid("duplicate Unicode cmap encoding record"));
        }
        let offset = u32r(data, 8 + i * 8)? as usize;
        if offset < end {
            return Err(invalid("cmap subtable overlaps records"));
        }
        let format = u16r(data, offset)?;
        if pair == (0, 5) {
            if format != 14 {
                return Err(invalid("Unicode variation cmap format"));
            }
            let length = u32r(data, offset + 2)? as usize;
            let table = data
                .get(offset..)
                .and_then(|tail| tail.get(..length))
                .ok_or_else(|| invalid("cmap length"))?;
            variation::validate(table, glyphs, check)?;
            uvs = Some(i as u16);
        } else if let Some(rank) = rank
            && selected.is_none_or(|(_, old)| rank < old)
        {
            selected = Some((i, rank));
        }
    }
    let cmap = font.cmap()?;
    let base = if let Some((i, _)) = selected {
        let offset = u32r(data, 8 + i * 8)? as usize;
        let format = u16r(data, offset)?;
        let length = match format {
            0 | 4 | 6 => u16r(data, offset + 2)? as usize,
            10 | 12 | 13 => u32r(data, offset + 4)? as usize,
            _ => return Err(FontError::Unsupported("selected Unicode cmap format")),
        };
        let table = data
            .get(offset..)
            .and_then(|tail| tail.get(..length))
            .ok_or_else(|| invalid("cmap length"))?;
        validation::validate(table, format, glyphs, check)?;
        Some((
            FontCmap {
                record_index: i as u16,
                platform_id: u16r(data, 4 + i * 8)?,
                encoding_id: u16r(data, 6 + i * 8)?,
                format,
                variation_record_index: uvs,
            },
            cmap.subtable(i as u16)?,
        ))
    } else {
        None
    };
    if uvs.is_some() && base.is_none() {
        return Err(invalid("variation cmap without Unicode base"));
    }
    let variation = uvs.map(|i| cmap.subtable(i)).transpose()?;
    let (info, base) = match base {
        Some((info, base)) => (Some(info), Some(base)),
        None => (None, None),
    };
    Ok(CheckedCmap {
        info,
        base,
        variation,
    })
}
impl CheckedCmap<'_> {
    pub fn query(&self, character: FontCharacter) -> CoverageOutcome {
        if let Some(table) = &self.base {
            if let Some(selector) = character.variation_selector {
                let mapped = match &self.variation {
                    Some(CmapSubtable::Format14(t)) => t.map_variant(character.codepoint, selector),
                    _ => None,
                };
                match mapped {
                    Some(MapVariant::UseDefault) => {
                        outcome(table.map_codepoint(character.codepoint).map(|v| v.to_u32()))
                    }
                    Some(MapVariant::Variant(id)) => outcome(Some(id.to_u32())),
                    None => CoverageOutcome::UnsupportedVariation,
                }
            } else {
                outcome(table.map_codepoint(character.codepoint).map(|v| v.to_u32()))
            }
        } else {
            CoverageOutcome::NoUnicodeCmap
        }
    }
}
use std::collections::BTreeSet;
fn outcome(glyph: Option<u32>) -> CoverageOutcome {
    match glyph {
        Some(id) if id != 0 => CoverageOutcome::Mapped { glyph_id: id },
        _ => CoverageOutcome::Missing,
    }
}
fn glyph(id: u32, count: u16) -> Result<(), FontError> {
    if id >= count as u32 {
        Err(invalid("cmap glyph outside glyph set"))
    } else {
        Ok(())
    }
}
