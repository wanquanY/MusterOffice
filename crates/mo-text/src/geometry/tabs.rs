//! Explicit, format-independent left tab stops. No implicit font or space glyphs.
use super::*;
use mo_geometry::Fixed;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LeftTabStops {
    /// Positive repeating grid used after the final explicit stop (or alone).
    pub interval: Fixed,
    /// Strictly increasing positions relative to the paragraph's left margin.
    pub stops: Vec<Fixed>,
    /// Line origins in the same coordinate system; accommodates first-line indent.
    pub first_line_offset: Fixed,
    pub continuation_offset: Fixed,
}
impl LeftTabStops {
    pub(crate) fn validate(&self) -> Result<(), TextError> {
        if self.stops.len() > 32 {
            return Err(TextError::Limit("paragraph tab stops"));
        }
        if self.interval <= Fixed::ZERO || self.stops.windows(2).any(|w| w[0] >= w[1]) {
            return Err(TextError::Invalid(
                "positive tab interval and increasing stops",
            ));
        }
        Ok(())
    }
    pub(crate) fn next(&self, pen: Fixed, line_start: u32) -> Result<Fixed, TextError> {
        let offset = if line_start == 0 {
            self.first_line_offset
        } else {
            self.continuation_offset
        };
        let position = pen.checked_add(offset)?;
        let at = self.stops.partition_point(|&stop| stop <= position);
        let stop = if let Some(&stop) = self.stops.get(at) {
            stop
        } else {
            // Euclidean division also advances correctly from negative indents.
            let step = position.raw().div_euclid(self.interval.raw());
            let raw = step
                .checked_add(1)
                .and_then(|v| v.checked_mul(self.interval.raw()))
                .ok_or(TextError::Invalid("tab position range"))?;
            Fixed::from_raw(raw)
        };
        Ok(stop.checked_sub(offset)?)
    }
}
