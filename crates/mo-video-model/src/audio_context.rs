use crate::{AudioRenderContext, SampleSide, TimeMap, TimeSample, ValidationError};
use mo_time::{ExactRatio as R, TimeError};

fn boundary(map: &TimeMap, local: &R, bits: u64) -> Result<Option<R>, TimeError> {
    if let Some(last) = map.anchors.last()
        && R::from_time(last.local).compare(local).is_eq()
    {
        return Ok(Some(R::from_time(last.source)));
    }
    Ok(map
        .sample(
            &TimeSample {
                time: local.clone(),
                side: SampleSide::After,
            },
            bits,
        )?
        .map(|v| v.sample.time))
}
impl AudioRenderContext {
    /// Both maps must first pass ordinary anchor validation. Checking their
    /// union of breakpoints proves equality over every visible linear segment.
    pub fn matches_window(
        &self,
        visible: &TimeMap,
        duration: &R,
        bits: u64,
        check: &dyn Fn() -> bool,
    ) -> Result<bool, ValidationError> {
        let offset = R::from_time(self.offset);
        let end = offset.add(duration, bits)?;
        let Some(last) = self.time_map.anchors.last() else {
            return Ok(false);
        };
        if offset.compare(&R::integer(0)).is_lt() || end.compare(&R::from_time(last.local)).is_gt()
        {
            return Ok(false);
        }
        for a in &visible.anchors {
            if check() {
                return Err(ValidationError::Cancelled);
            }
            let local = R::from_time(a.local).add(&offset, bits)?;
            if boundary(&self.time_map, &local, bits)?
                .is_none_or(|value| value.compare(&R::from_time(a.source)).is_ne())
            {
                return Ok(false);
            }
        }
        for a in &self.time_map.anchors {
            if check() {
                return Err(ValidationError::Cancelled);
            }
            let local = R::from_time(a.local);
            if local.compare(&offset).is_ge() && local.compare(&end).is_le() {
                let local = local.sub(&offset, bits)?;
                if boundary(visible, &local, bits)?
                    .is_none_or(|value| value.compare(&R::from_time(a.source)).is_ne())
                {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
}
