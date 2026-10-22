//! Bounded, world-to-device outer shadow admission shared by every consumer.
use crate::{OuterShadow, RasterError, RasterViewport, number::Scale};
pub(crate) fn words(shadow: &OuterShadow, v: &RasterViewport) -> Result<[u32; 5], RasterError> {
    let scale = Scale::new(v.scale)?;
    let mut values = [0f32; 3];
    for (out, raw) in values.iter_mut().zip([
        shadow.offset.x.raw(),
        shadow.offset.y.raw(),
        shadow.sigma.raw(),
    ]) {
        *out = scale.value(raw)?;
        if scale.error(raw, *out)? > v.coordinate_tolerance.raw() {
            return Err(RasterError::Precision);
        }
    }
    if values[0].abs() > 8192.0 || values[1].abs() > 8192.0 || !(0.0..=32.0).contains(&values[2]) {
        return Err(RasterError::Limit("outer shadow extent or sigma"));
    }
    Ok([
        1,
        u32::from_le_bytes(shadow.rgba),
        values[0].to_bits(),
        values[1].to_bits(),
        values[2].to_bits(),
    ])
}
