//! Admission before derived scene storage, shared with final raster work checks.
use crate::{Brush, RasterError};
pub const MAX_DRAWS: usize = 65536;
pub const MAX_GRADIENT_INPUT_STOPS: usize = 262144;
pub(crate) fn stops(current: usize, additional: usize) -> Result<usize, RasterError> {
    current
        .checked_add(additional)
        .filter(|n| *n <= MAX_GRADIENT_INPUT_STOPS)
        .ok_or(RasterError::Limit("gradient input stops"))
}
#[derive(Debug, Default, Clone, Copy)]
pub struct PaintBudget {
    draws: usize,
    gradient_stops: usize,
}
impl PaintBudget {
    /// Counts logical work even when ramps share storage. Commit only on success
    /// so callers can discard a rejected candidate without corrupting admission.
    pub fn include_draw(&mut self, gradient_stop_count: usize) -> Result<(), RasterError> {
        let draws = self
            .draws
            .checked_add(1)
            .filter(|n| *n <= MAX_DRAWS)
            .ok_or(RasterError::Limit("draws"))?;
        let gradient_stops = stops(self.gradient_stops, gradient_stop_count)?;
        self.draws = draws;
        self.gradient_stops = gradient_stops;
        Ok(())
    }
    pub fn include_brush(&mut self, brush: &Brush) -> Result<(), RasterError> {
        self.include_draw(match brush {
            Brush::Gradient { gradient } => gradient.stops.len(),
            _ => 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn budgets_are_exact_and_failed_admission_does_not_consume_remaining_work() {
        let mut b = PaintBudget::default();
        for _ in 0..64 {
            b.include_draw(4096).unwrap();
        }
        assert!(matches!(
            b.include_draw(1),
            Err(RasterError::Limit("gradient input stops"))
        ));
        assert!(b.include_draw(usize::MAX).is_err());
        for _ in 64..MAX_DRAWS {
            b.include_draw(0).unwrap();
        }
        assert!(matches!(
            b.include_draw(0),
            Err(RasterError::Limit("draws"))
        ));
        assert_eq!(
            (b.draws, b.gradient_stops),
            (MAX_DRAWS, MAX_GRADIENT_INPUT_STOPS)
        );
    }
}
