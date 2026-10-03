//! Font-provided ligature carets from the same verified font instance as shaping.
#[cfg(test)]
mod tests;
mod types;
use crate::{backend::*, *};
use mo_font::{FontLimits, VerifiedFont};
use std::collections::BTreeSet;
pub use types::*;

fn check_size(q: &FontCaretsRequest) -> Result<(), TextError> {
    if q.instances.len() > 64
        || q.instances
            .iter()
            .any(|i| i.variations.len() > 64 || i.glyph_ids.len() > 256)
        || q.instances.iter().map(|i| i.glyph_ids.len()).sum::<usize>() > 4096
    {
        return Err(TextError::Limit("caret instances, glyphs or axes"));
    }
    Ok(())
}
pub fn query(
    q: &FontCaretsRequest,
    bytes: &[u8],
    backend: &mut dyn TextBackend,
    check: &dyn Fn() -> bool,
) -> Result<FontCaretsResult, TextError> {
    cancelled(check)?;
    check_size(q)?;
    let font = VerifiedFont::load(
        &q.expected_sha256,
        q.face_index,
        bytes,
        FontLimits::default(),
        check,
    )?;
    query_verified(q, &font, backend, check)
}
pub fn query_verified(
    q: &FontCaretsRequest,
    font: &VerifiedFont<'_>,
    backend: &mut dyn TextBackend,
    check: &dyn Fn() -> bool,
) -> Result<FontCaretsResult, TextError> {
    cancelled(check)?;
    check_size(q)?;
    let metadata = font.metadata();
    if q.expected_sha256 != metadata.sha256 || q.face_index != metadata.face_index {
        return Err(mo_font::FontError::ResourceConflict.into());
    }
    let mut frame = vec![
        CARETS_BATCH_MAGIC,
        1,
        HARFBUZZ_VERSION,
        q.instances.len() as u32,
    ];
    let mut axes = Vec::new();
    // Every instance is checked before the first external component call.
    for instance in &q.instances {
        cancelled(check)?;
        let coords = prepare::validate_variations(&instance.variations, metadata)?;
        let mut seen = BTreeSet::new();
        for &id in &instance.glyph_ids {
            cancelled(check)?;
            if id >= u32::from(metadata.glyph_count) || !seen.insert(id) {
                return Err(TextError::Invalid("caret glyph id or duplicate"));
            }
        }
        frame.extend([
            8 + coords.len() as u32 * 2 + instance.glyph_ids.len() as u32,
            CARETS_MAGIC,
            1,
            q.face_index,
            coords.len() as u32,
            instance.direction.word(),
            instance.glyph_ids.len() as u32,
            MAX_CARETS_PER_GLYPH as u32,
            0,
        ]);
        for coord in &coords {
            frame.extend([prepare::tag(&coord.tag)?, coord.effective_f32_bits]);
        }
        frame.extend(&instance.glyph_ids);
        axes.push(coords);
    }
    cancelled(check)?;
    let raw = if q.instances.is_empty() {
        vec![0, 0]
    } else {
        match backend.caret_batch(font.bytes(), &frame) {
            Ok(raw) => raw,
            Err(error) => {
                backend.invalidate();
                return Err(error);
            }
        }
    };
    let decode = || -> Result<FontCaretsResult, TextError> {
        cancelled(check)?;
        if raw.len() < 2 || raw.len() > MAX_CARETS_RESULT_WORDS {
            return Err(TextError::BackendInvalid("caret batch length"));
        }
        if raw[0] != 0 {
            if raw.len() != 2 || raw[0] > 6 || raw[1] as usize >= q.instances.len() {
                return Err(TextError::BackendInvalid("caret failure frame"));
            }
            return Err(TextError::BackendFailure {
                status: raw[0],
                run: raw[1],
            });
        }
        if raw[1] as usize != q.instances.len() {
            return Err(TextError::BackendInvalid("caret instance count"));
        }
        let mut offset = 2;
        let mut instances = Vec::new();
        for (instance, effective_variations) in q.instances.iter().zip(axes) {
            cancelled(check)?;
            let count = *raw
                .get(offset)
                .ok_or(TextError::BackendInvalid("caret output length"))?
                as usize;
            offset += 1;
            if count < 6 || count > 6 + instance.glyph_ids.len() * (2 + MAX_CARETS_PER_GLYPH) {
                return Err(TextError::BackendInvalid("caret output size"));
            }
            let words = raw
                .get(offset..offset + count)
                .ok_or(TextError::BackendInvalid("caret output range"))?;
            offset += count;
            if words[..6]
                != [
                    CARETS_MAGIC,
                    1,
                    metadata.units_per_em as u32,
                    metadata.units_per_em as u32 * 64,
                    instance.direction.word(),
                    instance.glyph_ids.len() as u32,
                ]
            {
                return Err(TextError::BackendInvalid("caret output header"));
            }
            let mut cursor = 6;
            let mut glyphs = Vec::new();
            for &glyph_id in &instance.glyph_ids {
                cancelled(check)?;
                let head = words
                    .get(cursor..cursor + 2)
                    .ok_or(TextError::BackendInvalid("caret glyph header"))?;
                cursor += 2;
                if head[0] != glyph_id || head[1] as usize > MAX_CARETS_PER_GLYPH {
                    return Err(TextError::BackendInvalid("caret glyph identity or count"));
                }
                let positions = words
                    .get(cursor..cursor + head[1] as usize)
                    .ok_or(TextError::BackendInvalid("caret glyph positions"))?;
                cursor += positions.len();
                glyphs.push(GlyphCarets {
                    glyph_id,
                    positions: positions.iter().map(|&p| p as i32).collect(),
                });
            }
            if cursor != words.len() {
                return Err(TextError::BackendInvalid("trailing caret instance output"));
            }
            instances.push(MeasuredCarets {
                effective_variations,
                direction: instance.direction,
                glyphs,
            });
        }
        if offset != raw.len() {
            return Err(TextError::BackendInvalid("trailing caret output"));
        }
        cancelled(check)?;
        Ok(FontCaretsResult {
            font_sha256: metadata.sha256.clone(),
            face_index: metadata.face_index,
            units_per_em: metadata.units_per_em,
            position_units_per_em: metadata.units_per_em as u32 * 64,
            profile: "harfbuzz-14.5.0-gdef-carets-design64-no-synthesis-v1".into(),
            instances,
        })
    };
    let result = decode();
    if !q.instances.is_empty()
        && matches!(
            result,
            Err(TextError::BackendInvalid(_)
                | TextError::BackendFailure { status: 2 | 6, .. }
                | TextError::Cancelled)
        )
    {
        backend.invalidate();
    }
    result
}
