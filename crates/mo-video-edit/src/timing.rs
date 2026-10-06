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

fn window_map(map: &TimeMap, start: RationalTime, end: RationalTime, bits: u64) -> Result<TimeMap> {
    let mut anchors = vec![TimeAnchor {
        local: zero(),
        source: sample_boundary(map, start, bits)?,
    }];
    for anchor in &map.anchors {
        if anchor.local.compare_time(start).is_gt() && anchor.local.compare_time(end).is_lt() {
            anchors.push(TimeAnchor {
                local: sub(anchor.local, start, bits)?,
                source: anchor.source,
            });
        }
    }
    anchors.push(TimeAnchor {
        local: sub(end, start, bits)?,
        source: sample_boundary(map, end, bits)?,
    });
    Ok(TimeMap { anchors })
}

fn needs_context(map: &TimeMap, policy: AudioTimePolicy, bits: u64) -> Result<bool> {
    if policy.rate != AudioRatePolicy::PreservePitch {
        return Ok(false);
    }
    for pair in map.anchors.windows(2) {
        let duration = ExactRatio::from_time(pair[1].local)
            .sub(&ExactRatio::from_time(pair[0].local), bits)?;
        let span = ExactRatio::from_time(pair[1].source)
            .sub(&ExactRatio::from_time(pair[0].source), bits)?;
        if span.compare(&ExactRatio::integer(0)).is_ne()
            && span.compare(&duration).is_ne()
            && span
                .compare(&ExactRatio::integer(0).sub(&duration, bits)?)
                .is_ne()
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn preserved_context(
    clip: &Clip,
    map: &TimeMap,
    start: RationalTime,
    end: RationalTime,
    bits: u64,
) -> Result<Option<AudioRenderContext>> {
    if clip.audio_context.is_none() && !needs_context(map, clip.audio, bits)? {
        return Ok(None);
    }
    let mut context = clip
        .audio_context
        .clone()
        .unwrap_or_else(|| AudioRenderContext {
            time_map: map.clone(),
            offset: zero(),
        });
    let offset = add(context.offset, start, bits)?;
    let visible_end = add(context.offset, end, bits)?;
    let context_end = context
        .time_map
        .anchors
        .last()
        .ok_or(EditError::InvalidOperation("empty audio context"))?
        .local;
    let lo = if offset.compare_time(zero()).is_lt() {
        offset
    } else {
        zero()
    };
    let hi = if visible_end.compare_time(context_end).is_gt() {
        visible_end
    } else {
        context_end
    };
    if lo.ticks.get() != 0 || hi.compare_time(context_end).is_ne() {
        context.time_map = window_map(&context.time_map, lo, hi, bits)?;
    }
    context.offset = sub(offset, lo, bits)?;
    Ok(Some(context))
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
        if local_start.ticks.get() != 0
            || old_duration
                .compare(&ExactRatio::from_time(local_end))
                .is_ne()
        {
            out.audio_context = preserved_context(clip, time_map, local_start, local_end, bits)?;
        }
        *time_map = match &out.audio_context {
            Some(context) => window_map(
                &context.time_map,
                context.offset,
                add(context.offset, sub(end, start, bits)?, bits)?,
                bits,
            )?,
            None => window_map(time_map, local_start, local_end, bits)?,
        };
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
