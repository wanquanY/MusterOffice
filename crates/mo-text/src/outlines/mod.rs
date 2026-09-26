//! Batched unhinted monochrome paths from the exact font instance used to shape.
mod decode;
#[cfg(test)]
mod tests;
mod types;
use crate::{backend::*, *};
use mo_font::{FontLimits, VerifiedFont};
pub use types::*;
fn check_size(q: &FontOutlinesRequest) -> Result<(), TextError> {
    if q.instances.len() > 64
        || q.instances.iter().any(|i| {
            i.variations.len() > 64
                || i.glyph_ids.len() > 256
                || i.max_commands > 262144
                || i.max_operations == 0
                || i.max_operations > 1048576
        })
        || q.instances.iter().map(|i| i.glyph_ids.len()).sum::<usize>() > 4096
        || q.instances
            .iter()
            .map(|i| u64::from(i.max_commands))
            .sum::<u64>()
            > 262144
    {
        return Err(TextError::Limit("outline instances, glyphs, axes or work"));
    }
    Ok(())
}
pub fn extract(
    q: &FontOutlinesRequest,
    bytes: &[u8],
    backend: &mut dyn TextBackend,
    check: &dyn Fn() -> bool,
) -> Result<FontOutlinesResult, TextError> {
    cancelled(check)?;
    check_size(q)?;
    let font = VerifiedFont::load(
        &q.expected_sha256,
        q.face_index,
        bytes,
        FontLimits::default(),
        check,
    )?;
    extract_verified(q, &font, backend, check)
}
pub fn extract_verified(
    q: &FontOutlinesRequest,
    font: &VerifiedFont<'_>,
    backend: &mut dyn TextBackend,
    check: &dyn Fn() -> bool,
) -> Result<FontOutlinesResult, TextError> {
    cancelled(check)?;
    check_size(q)?;
    let metadata = font.metadata();
    if q.expected_sha256 != metadata.sha256 || q.face_index != metadata.face_index {
        return Err(mo_font::FontError::ResourceConflict.into());
    }
    let mut frame = vec![
        OUTLINES_BATCH_MAGIC,
        1,
        HARFBUZZ_VERSION,
        q.instances.len() as u32,
    ];
    let mut axes = Vec::new();
    // Validate every instance before any component invocation.
    for instance in &q.instances {
        cancelled(check)?;
        let coords = prepare::validate_variations(&instance.variations, metadata)?;
        if instance
            .glyph_ids
            .iter()
            .any(|&g| g >= u32::from(metadata.glyph_count))
        {
            return Err(TextError::Invalid("outline glyph id"));
        }
        frame.extend([
            8 + coords.len() as u32 * 2 + instance.glyph_ids.len() as u32,
            OUTLINES_MAGIC,
            1,
            q.face_index,
            coords.len() as u32,
            instance.glyph_ids.len() as u32,
            instance.max_commands,
            instance.max_operations,
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
        match backend.outline_batch(font.bytes(), &frame) {
            Ok(raw) => raw,
            Err(e) => {
                backend.invalidate();
                return Err(e);
            }
        }
    };
    let result = decode::decode(q, metadata, axes, &raw, check);
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
