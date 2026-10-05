use super::*;
use mo_time::ExactRatio;

impl State<'_> {
    pub(super) fn retime(
        &mut self,
        targets: &[RetimeTarget],
        effects: EffectTimingPolicy,
        ripple_tracks: &[TrackId],
        linked_policy: LinkedPolicy,
    ) -> Result<()> {
        let ids: Vec<_> = targets.iter().map(|t| t.clip.clone()).collect();
        let selection = self.selection(&ids, linked_policy)?;
        let mut ripple_boundary: Option<(RationalTime, RationalTime)> = None;
        for target in targets {
            let mut clip = self.clip(&target.clip)?;
            let old_duration = clip.timeline.duration(self.bits())?;
            let new_duration = target
                .time_map
                .anchors
                .last()
                .ok_or(EditError::InvalidOperation("retime requires anchors"))?
                .local;
            if new_duration.ticks.get() <= 0 {
                return Err(EditError::InvalidOperation(
                    "retime duration must be positive",
                ));
            }
            let delta = ExactRatio::from_time(new_duration)
                .sub(&old_duration, self.bits())?
                .to_time_exact()?;
            if !ripple_tracks.is_empty() {
                if let Some((at, shift)) = ripple_boundary
                    && (at.compare_time(clip.timeline.end).is_ne()
                        || shift.compare_time(delta).is_ne())
                {
                    return Err(EditError::InvalidOperation(
                        "retimed ripple group must share end boundary and duration change",
                    ));
                }
                ripple_boundary = Some((clip.timeline.end, delta));
            }
            let (ClipSource::Media { time_map, .. } | ClipSource::Sequence { time_map, .. }) =
                &mut clip.source
            else {
                return Err(EditError::InvalidOperation(
                    "retime requires a mapped media or sequence source",
                ));
            };
            *time_map = target.time_map.clone();
            clip.audio = target.audio;
            clip.timeline.end = add(clip.timeline.start, new_duration, self.bits())?;
            if effects == EffectTimingPolicy::ScaleToDuration {
                let scale = ExactRatio::from_time(new_duration).div(&old_duration, self.bits())?;
                for effect in &mut clip.effects {
                    self.poll()?;
                    match effect {
                        Effect::Fade {
                            fade_in,
                            fade_out,
                            envelope,
                        } => {
                            for value in [fade_in, fade_out, &mut envelope.start, &mut envelope.end]
                            {
                                *value = ExactRatio::from_time(*value)
                                    .mul(&scale, self.bits())?
                                    .to_time_exact()?;
                            }
                        }
                        Effect::Transform { .. } | Effect::Gain { .. } => {}
                    }
                }
            }
            self.put_clip(clip);
        }
        if let Some((at, delta)) = ripple_boundary {
            let sequence = self.selection_sequence(&selection)?;
            self.ripple(ripple_tracks, at, delta, &selection, &sequence)?;
        }
        Ok(())
    }
}
