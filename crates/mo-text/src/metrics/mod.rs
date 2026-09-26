//! Font-instance metrics from the same isolated font functions/axes as shaping.
#[cfg(test)]
mod tests;
mod types;
use crate::{backend::*, *};
use mo_font::{FontLimits, VerifiedFont};
use std::collections::BTreeSet;
pub use types::*;
fn check_size(request: &FontMetricsRequest) -> Result<(), TextError> {
    if request.instances.len() > 256
        || request
            .instances
            .iter()
            .any(|i| i.metrics.len() > 28 || i.variations.len() > 64)
    {
        return Err(TextError::Limit("font metric instances, values or axes"));
    }
    Ok(())
}
pub fn measure(
    request: &FontMetricsRequest,
    bytes: &[u8],
    backend: &mut dyn TextBackend,
    check: &dyn Fn() -> bool,
) -> Result<FontMetricsResult, TextError> {
    cancelled(check)?;
    check_size(request)?;
    let font = VerifiedFont::load(
        &request.expected_sha256,
        request.face_index,
        bytes,
        FontLimits::default(),
        check,
    )?;
    measure_verified(request, &font, backend, check)
}
pub fn measure_verified(
    request: &FontMetricsRequest,
    font: &VerifiedFont<'_>,
    backend: &mut dyn TextBackend,
    check: &dyn Fn() -> bool,
) -> Result<FontMetricsResult, TextError> {
    cancelled(check)?;
    check_size(request)?;
    let metadata = font.metadata();
    if request.expected_sha256 != metadata.sha256 || request.face_index != metadata.face_index {
        return Err(mo_font::FontError::ResourceConflict.into());
    }
    let mut frame = vec![
        METRICS_BATCH_MAGIC,
        1,
        HARFBUZZ_VERSION,
        request.instances.len() as u32,
    ];
    let mut variations = Vec::new();
    for instance in &request.instances {
        cancelled(check)?;
        let coords = prepare::validate_variations(&instance.variations, metadata)?;
        let mut seen = BTreeSet::new();
        for metric in &instance.metrics {
            if !seen.insert(metric) {
                return Err(TextError::Invalid("duplicate font metric"));
            }
        }
        frame.extend([
            6 + coords.len() as u32 * 2 + instance.metrics.len() as u32,
            METRICS_MAGIC,
            1,
            request.face_index,
            coords.len() as u32,
            instance.metrics.len() as u32,
            0,
        ]);
        for coord in &coords {
            frame.extend([prepare::tag(&coord.tag)?, coord.effective_f32_bits]);
        }
        frame.extend(instance.metrics.iter().map(|m| m.word()));
        variations.push(coords);
    }
    cancelled(check)?;
    let raw = if request.instances.is_empty() {
        vec![0, 0]
    } else {
        match backend.measure_batch(font.bytes(), &frame) {
            Ok(raw) => raw,
            Err(error) => {
                backend.invalidate();
                return Err(error);
            }
        }
    };
    let decode = || -> Result<FontMetricsResult, TextError> {
        cancelled(check)?;
        if raw.len() < 2 || raw.len() > MAX_METRICS_RESULT_WORDS {
            return Err(TextError::BackendInvalid("metric batch length"));
        }
        if raw[0] != 0 {
            if raw.len() != 2 || raw[0] > 6 || raw[1] as usize >= request.instances.len() {
                return Err(TextError::BackendInvalid("metric failure frame"));
            }
            return Err(TextError::BackendFailure {
                status: raw[0],
                run: raw[1],
            });
        }
        if raw[1] as usize != request.instances.len() {
            return Err(TextError::BackendInvalid("metric instance count"));
        }
        let mut offset = 2;
        let mut instances = Vec::new();
        for (instance, effective_variations) in request.instances.iter().zip(variations) {
            cancelled(check)?;
            let count = *raw
                .get(offset)
                .ok_or(TextError::BackendInvalid("metric output length"))?
                as usize;
            offset += 1;
            if count != 6 + instance.metrics.len() * 3 {
                return Err(TextError::BackendInvalid("metric output size"));
            }
            let words = raw
                .get(offset..offset + count)
                .ok_or(TextError::BackendInvalid("metric output range"))?;
            offset += count;
            if words[..6]
                != [
                    METRICS_MAGIC,
                    1,
                    metadata.units_per_em as u32,
                    metadata.units_per_em as u32 * 64,
                    instance.metrics.len() as u32,
                    0,
                ]
            {
                return Err(TextError::BackendInvalid("metric output header"));
            }
            let mut values = Vec::new();
            for (&metric, data) in instance.metrics.iter().zip(words[6..].chunks_exact(3)) {
                cancelled(check)?;
                if data[0] != metric.word() || data[1] > 1 || (data[1] == 0 && data[2] != 0) {
                    return Err(TextError::BackendInvalid("metric identity or availability"));
                }
                values.push(MeasuredMetric {
                    metric,
                    position: if data[1] == 1 {
                        Some(data[2] as i32)
                    } else {
                        None
                    },
                });
            }
            instances.push(MeasuredInstance {
                effective_variations,
                values,
            });
        }
        if offset != raw.len() {
            return Err(TextError::BackendInvalid("trailing metric output"));
        }
        cancelled(check)?;
        Ok(FontMetricsResult {
            font_sha256: metadata.sha256.clone(),
            face_index: metadata.face_index,
            units_per_em: metadata.units_per_em,
            position_units_per_em: metadata.units_per_em as u32 * 64,
            profile: "harfbuzz-14.5.0-ot-metrics-design64-no-synthesis-v1".into(),
            instances,
        })
    };
    let result = decode();
    if !request.instances.is_empty()
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
