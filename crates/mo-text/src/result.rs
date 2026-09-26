use crate::{backend::*, *};
use mo_font::FontInspection;
use std::collections::BTreeSet;
pub(crate) fn decode(
    request: &ShapeRequest,
    font: &FontInspection,
    variations: Vec<Vec<EffectiveVariation>>,
    raw: &[u32],
    limits: TextLimits,
    check: &dyn Fn() -> bool,
) -> Result<ShapedText, TextError> {
    if raw.len() < 2 || raw.len() > MAX_RESULT_WORDS {
        return Err(TextError::BackendInvalid("batch output size"));
    }
    if raw[0] != 0 {
        if raw.len() != 2 || !(1..=6).contains(&raw[0]) || raw[1] as usize >= request.runs.len() {
            return Err(TextError::BackendInvalid("failure frame"));
        }
        return Err(TextError::BackendFailure {
            status: raw[0],
            run: raw[1],
        });
    }
    if raw[1] as usize != request.runs.len() {
        return Err(TextError::BackendInvalid("run count"));
    }
    let mut offset = 2;
    let mut total = 0usize;
    let mut runs = Vec::new();
    for (run, effective_variations) in request.runs.iter().zip(variations) {
        cancelled(check)?;
        let length = *raw
            .get(offset)
            .ok_or(TextError::BackendInvalid("run length"))? as usize;
        offset += 1;
        if length > MAX_RESULT_WORDS {
            return Err(TextError::BackendInvalid("run word limit"));
        }
        let words = raw
            .get(offset..offset + length)
            .ok_or(TextError::BackendInvalid("run output range"))?;
        offset += length;
        if words.len() < 8
            || words[0] != COMPONENT_MAGIC
            || words[1] != 1
            || words[2] != u32::from(font.units_per_em)
            || words[3] != u32::from(font.units_per_em) * 64
            || words[5] != run.flags.word()
            || words[6] != run.cluster_level.word()
            || words[7] != 0
        {
            return Err(TextError::BackendInvalid("run output header"));
        }
        let count = words[4] as usize;
        if count > 262144 || count > run.max_glyphs as usize || words.len() != 8 + count * 7 {
            return Err(TextError::BackendInvalid("glyph count"));
        }
        total += count;
        if total > limits.max_glyphs {
            return Err(TextError::Limit("total output glyphs"));
        }
        let mut glyphs = Vec::with_capacity(count);
        let mut missing = BTreeSet::new();
        let mut previous = None;
        for g in words[8..].chunks_exact(7) {
            cancelled(check)?;
            if g[0] >= u32::from(font.glyph_count)
                || g[1] < run.start
                || g[1] >= run.end
                || g[2] & !7 != 0
            {
                return Err(TextError::BackendInvalid("glyph, scalar cluster or flags"));
            }
            if run.cluster_level.word() < 2
                && previous.is_some_and(|p| {
                    if run.direction.backward() {
                        p < g[1]
                    } else {
                        p > g[1]
                    }
                })
            {
                return Err(TextError::BackendInvalid("nonmonotone cluster"));
            }
            previous = Some(g[1]);
            if g[0] == 0 {
                missing.insert(g[1]);
            }
            glyphs.push(ShapedGlyph {
                glyph_id: g[0],
                cluster: g[1],
                unsafe_to_break: g[2] & 1 != 0,
                unsafe_to_concat: g[2] & 2 != 0,
                safe_to_insert_tatweel: g[2] & 4 != 0,
                x_advance: g[3] as i32,
                y_advance: g[4] as i32,
                x_offset: g[5] as i32,
                y_offset: g[6] as i32,
            });
        }
        runs.push(ShapedRun {
            start: run.start,
            end: run.end,
            direction: run.direction,
            effective_variations,
            missing_glyph_clusters: missing.into_iter().collect(),
            glyphs,
        });
    }
    if offset != raw.len() {
        return Err(TextError::BackendInvalid("trailing output words"));
    }
    Ok(ShapedText {
        font_sha256: font.sha256.clone(),
        face_index: font.face_index,
        units_per_em: font.units_per_em,
        position_units_per_em: u32::from(font.units_per_em) * 64,
        profile: "harfbuzz-14.5.0-ot-ucd18-design64-v1".into(),
        runs,
    })
}
