use crate::{EditError, Result};
use mo_common::RationalTime;
use mo_time::ExactRatio;
use mo_video_model::*;

pub(crate) fn add(a: RationalTime, b: RationalTime, bits: u64) -> Result<RationalTime> {
    Ok(ExactRatio::from_time(a)
        .add(&ExactRatio::from_time(b), bits)?
        .to_time_exact()?)
}
pub(crate) fn sub(a: RationalTime, b: RationalTime, bits: u64) -> Result<RationalTime> {
    Ok(ExactRatio::from_time(a)
        .sub(&ExactRatio::from_time(b), bits)?
        .to_time_exact()?)
}
pub(crate) fn zero() -> RationalTime {
    RationalTime::new(0, 1).expect("valid zero")
}

fn sample_boundary(map: &TimeMap, local: RationalTime, bits: u64) -> Result<RationalTime> {
    if let Some(value) = map.map_boundary(local, bits)? {
        return Ok(value.to_time_exact()?);
    }
    // Extending a trim uses the edge segment slope, then normal validation checks actual handles.
    let pair = if local.compare_time(zero()).is_lt() {
        map.anchors.get(..2)
    } else {
        map.anchors.get(map.anchors.len().saturating_sub(2)..)
    }
    .filter(|p| p.len() == 2)
    .ok_or(EditError::InvalidOperation("invalid time map"))?;
    let elapsed = ExactRatio::from_time(local).sub(&ExactRatio::from_time(pair[0].local), bits)?;
    let duration =
        ExactRatio::from_time(pair[1].local).sub(&ExactRatio::from_time(pair[0].local), bits)?;
    let span =
        ExactRatio::from_time(pair[1].source).sub(&ExactRatio::from_time(pair[0].source), bits)?;
    Ok(ExactRatio::from_time(pair[0].source)
        .add(&span.mul(&elapsed.div(&duration, bits)?, bits)?, bits)?
        .to_time_exact()?)
}

/// Keeps piecewise source mapping and effect phase without sampling or rounding them.
pub(crate) fn window(
    clip: &Clip,
    start: RationalTime,
    end: RationalTime,
    bits: u64,
) -> Result<Clip> {
    if !clip.timeline.valid() {
        return Err(EditError::InvalidOperation("invalid source clip range"));
    }
    if start.compare_time(end).is_ge() {
        return Err(EditError::InvalidOperation("empty clip window"));
    }
    let local_start = sub(start, clip.timeline.start, bits)?;
    let local_end = sub(end, clip.timeline.start, bits)?;
    let duration = sub(end, start, bits)?;
    let mut out = clip.clone();
    out.timeline = TimeRange { start, end };
    if let ClipSource::Media { time_map, .. } | ClipSource::Sequence { time_map, .. } =
        &mut out.source
    {
        let old_duration = clip.timeline.duration(bits)?;
        if time_map.anchors.len() < 2
            || time_map.anchors[0].local.ticks.get() != 0
            || !time_map
                .anchors
                .windows(2)
                .all(|p| p[0].local.compare_time(p[1].local).is_lt())
            || time_map.anchors.last().is_none_or(|a| {
                old_duration
                    .compare(&ExactRatio::from_time(a.local))
                    .is_ne()
            })
        {
            return Err(EditError::InvalidOperation("invalid source clip time map"));
        }
        let source_start = sample_boundary(time_map, local_start, bits)?;
        let source_end = sample_boundary(time_map, local_end, bits)?;
        let mut anchors = vec![TimeAnchor {
            local: zero(),
            source: source_start,
        }];
        for anchor in &time_map.anchors {
            if anchor.local.compare_time(local_start).is_gt()
                && anchor.local.compare_time(local_end).is_lt()
            {
                anchors.push(TimeAnchor {
                    local: sub(anchor.local, local_start, bits)?,
                    source: anchor.source,
                });
            }
        }
        anchors.push(TimeAnchor {
            local: duration,
            source: source_end,
        });
        *time_map = TimeMap { anchors };
    }
    for effect in &mut out.effects {
        match effect {
            Effect::Fade { envelope, .. } => {
                envelope.start = sub(envelope.start, local_start, bits)?;
                envelope.end = sub(envelope.end, local_start, bits)?;
            }
            Effect::Transform { .. } | Effect::Gain { .. } => {}
        }
    }
    Ok(out)
}

pub(crate) fn shifted(clip: &Clip, delta: RationalTime, bits: u64) -> Result<Clip> {
    let mut out = clip.clone();
    out.timeline.start = add(out.timeline.start, delta, bits)?;
    out.timeline.end = add(out.timeline.end, delta, bits)?;
    Ok(out)
}
