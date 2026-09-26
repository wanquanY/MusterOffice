use crate::{backend::*, *};
use mo_font::FontInspection;
use std::collections::BTreeSet;
pub(crate) struct Prepared {
    pub frame: Vec<u32>,
    pub variations: Vec<Vec<EffectiveVariation>>,
}
pub(crate) fn tag(value: &str) -> Result<u32, TextError> {
    if value.len() != 4 || !value.bytes().all(|b| (32..=126).contains(&b)) {
        return Err(TextError::Invalid("four-byte OpenType tag"));
    }
    Ok(u32::from_be_bytes(
        value.as_bytes().try_into().expect("checked four bytes"),
    ))
}
pub(crate) fn check_size(request: &ShapeRequest, limits: TextLimits) -> Result<(), TextError> {
    // Check UTF-8 bytes before allocating the scalar buffer.
    if request.text.len() > limits.max_scalars.min(65536) * 4
        || request.runs.len() > limits.max_runs.min(256)
    {
        return Err(TextError::Limit("text bytes or runs"));
    }
    Ok(())
}
pub(crate) fn encode(
    request: &ShapeRequest,
    font: &FontInspection,
    limits: TextLimits,
    check: &dyn Fn() -> bool,
) -> Result<Prepared, TextError> {
    let text: Vec<u32> = request.text.chars().map(u32::from).collect();
    if text.len() > limits.max_scalars.min(65536)
        || text.len().saturating_mul(request.runs.len()) > limits.max_context_scalars
    {
        return Err(TextError::Limit(
            "Unicode scalars or cumulative shaping context",
        ));
    }
    let mut frame = vec![MAGIC, 1, HARFBUZZ_VERSION, request.runs.len() as u32];
    let mut effective = Vec::new();
    for run in &request.runs {
        cancelled(check)?;
        let coordinates = validate_run(run, font, text.len(), limits)?;
        let length = 13 + text.len() + run.features.len() * 4 + run.variations.len() * 2;
        if frame.len() + 2 + run.language.len() + length > MAX_REQUEST_WORDS {
            return Err(TextError::Limit("batch request words"));
        }
        frame.extend([run.language.len() as u32, length as u32]);
        frame.extend(run.language.bytes().map(u32::from));
        frame.extend([
            COMPONENT_MAGIC,
            request.face_index,
            run.direction.word(),
            tag(&run.script)?,
            run.flags.word(),
            run.cluster_level.word(),
            text.len() as u32,
            run.start,
            run.end - run.start,
            run.features.len() as u32,
            run.variations.len() as u32,
            run.max_glyphs,
            1,
        ]);
        frame.extend_from_slice(&text);
        for feature in &run.features {
            let end = feature.end.unwrap_or(text.len() as u32);
            if feature.start > end || end as usize > text.len() {
                return Err(TextError::Invalid("feature scalar range"));
            }
            frame.extend([
                tag(&feature.tag)?,
                feature.value,
                feature.start,
                feature.end.unwrap_or(u32::MAX),
            ]);
        }
        for coordinate in &coordinates {
            frame.extend([tag(&coordinate.tag)?, coordinate.effective_f32_bits]);
        }
        effective.push(coordinates);
    }
    Ok(Prepared {
        frame,
        variations: effective,
    })
}

pub(crate) fn validate_run(
    run: &ShapeRun,
    font: &FontInspection,
    scalar_count: usize,
    limits: TextLimits,
) -> Result<Vec<EffectiveVariation>, TextError> {
    if run.start > run.end || run.end as usize > scalar_count {
        return Err(TextError::Invalid("run scalar range"));
    }
    if run.script.len() != 4 || !run.script.bytes().all(|b| b.is_ascii_alphabetic()) {
        return Err(TextError::Invalid("ISO 15924 script tag"));
    }
    if run.language.is_empty()
        || run.language.len() > 255
        || !run
            .language
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err(TextError::Invalid("explicit shaping language"));
    }
    if run.features.len() > 1024
        || run.variations.len() > 64
        || run.max_glyphs as usize > limits.max_glyphs.min(262144)
    {
        return Err(TextError::Limit("run features, axes or glyph budget"));
    }

    for feature in &run.features {
        let end = feature.end.unwrap_or(scalar_count as u32);
        if feature.start > end || end as usize > scalar_count {
            return Err(TextError::Invalid("feature scalar range"));
        }
        tag(&feature.tag)?;
    }
    validate_variations(&run.variations, font)
}
pub(crate) fn validate_variations(
    variations: &[ShapeVariation],
    font: &FontInspection,
) -> Result<Vec<EffectiveVariation>, TextError> {
    if variations.len() > 64 {
        return Err(TextError::Limit("variation axes"));
    }
    let mut seen = BTreeSet::new();
    let mut coordinates = Vec::new();
    for variation in variations {
        if !seen.insert(&variation.tag) {
            return Err(TextError::Invalid("duplicate variation axis"));
        }
        let axis = font
            .axes
            .iter()
            .find(|a| a.tag == variation.tag)
            .ok_or(TextError::Invalid("unknown variation axis"))?;
        if !(axis.minimum_16_16..=axis.maximum_16_16).contains(&variation.value_16_16) {
            return Err(TextError::Invalid("variation outside font axis bounds"));
        }
        let bits = (variation.value_16_16 as f32 / 65536.0).to_bits();
        tag(&variation.tag)?;
        coordinates.push(EffectiveVariation {
            tag: variation.tag.clone(),
            requested_16_16: variation.value_16_16,
            effective_f32_bits: bits,
        });
    }
    Ok(coordinates)
}
