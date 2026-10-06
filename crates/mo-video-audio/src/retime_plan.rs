use crate::{AudioError, ChannelLayout, Result, StretchAnchor, StretchSchedule, cancel};
use mo_common::{Digest, digest};
use mo_time::{ExactRatio as R, Rounding};
use mo_video_compile::{AudioSpanSource, CompiledDocument};
use mo_video_model::*;
use serde::Serialize;
use std::sync::Arc;

/// A continuous, same-direction processing run. Its visible clips may be only
/// slices of this run; neither a UI seek nor a split defines a new DSP origin.
#[derive(Debug)]
pub struct RetimePlan {
    pub(crate) clip: ClipId,
    pub(crate) source: AudioSpanSource,
    pub(crate) source_identity: Digest,
    pub(crate) input_zero: R,
    pub(crate) direction: i64,
    pub(crate) first_sample: i64,
    pub(crate) sample_rate: u32,
    pub(crate) schedule: StretchSchedule,
}
impl RetimePlan {
    pub fn source(&self) -> &AudioSpanSource {
        &self.source
    }
    pub fn input_zero(&self) -> &R {
        &self.input_zero
    }
    pub fn direction(&self) -> i64 {
        self.direction
    }
    pub fn first_sample(&self) -> i64 {
        self.first_sample
    }
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    pub fn schedule(&self) -> &StretchSchedule {
        &self.schedule
    }
    pub fn output_frames(&self) -> i64 {
        self.schedule.output_frames() as i64
    }
}
#[derive(Debug, Clone)]
pub struct RetimeRequest {
    identity: Digest,
    profile: Digest,
    layout: ChannelLayout,
    plan: Arc<RetimePlan>,
}
impl RetimeRequest {
    pub(crate) fn new(
        plan: Arc<RetimePlan>,
        layout: ChannelLayout,
        profile: Digest,
    ) -> Result<Self> {
        #[derive(Serialize)]
        struct Key<'a> {
            source: &'a Digest,
            input_zero: &'a R,
            direction: i64,
            sample_rate: u32,
            schedule: &'a StretchSchedule,
            layout: &'a [crate::Channel],
            profile: &'a Digest,
            resampler: &'a str,
            stretcher: &'a str,
        }
        let identity = digest(
            "musteroffice.audio.retime/1-draft",
            &Key {
                source: &plan.source_identity,
                input_zero: &plan.input_zero,
                direction: plan.direction,
                sample_rate: plan.sample_rate,
                schedule: &plan.schedule,
                layout: layout.channels(),
                profile: &profile,
                resampler: crate::PROFILE,
                stretcher: crate::STRETCH_PROFILE,
            },
        )?;
        Ok(Self {
            identity,
            profile,
            layout,
            plan,
        })
    }
    pub fn identity(&self) -> &Digest {
        &self.identity
    }
    pub fn profile(&self) -> &Digest {
        &self.profile
    }
    pub fn layout(&self) -> &ChannelLayout {
        &self.layout
    }
    pub fn plan(&self) -> &RetimePlan {
        &self.plan
    }
    pub(crate) fn validate_document(&self, document: &CompiledDocument) -> Result<()> {
        if document.document().audio_sample_rate != self.plan.sample_rate
            || source_identity(document, &self.plan.source)? != self.plan.source_identity
        {
            return Err(AudioError::Invalid("stale retime input dependency"));
        }
        Ok(())
    }
}
fn source_identity(document: &CompiledDocument, source: &AudioSpanSource) -> Result<Digest> {
    match source {
        AudioSpanSource::Media { binding } => {
            let Some(resource) = document.document().resources.get(&binding.resource) else {
                return Err(AudioError::Invalid("retime resource removed"));
            };
            let ResourceDescriptor::Media { streams } = &resource.descriptor else {
                return Err(AudioError::Invalid("retime source type"));
            };
            if resource.digest != binding.digest
                || streams.get(&binding.stream) != Some(&binding.descriptor)
            {
                return Err(AudioError::Invalid("retime source changed"));
            }
            Ok(digest(
                "musteroffice.audio.media-input/1",
                &(&binding.digest, &binding.stream, &binding.descriptor),
            )?)
        }
        AudioSpanSource::Sequence { sequence } => {
            Ok(document.audio_sequence_digest(sequence)?.clone())
        }
    }
}
const NEGATIVE_PLAN_BYTES: usize = 1024;
pub(crate) struct CompiledRetime {
    pub plans: Vec<Arc<RetimePlan>>,
    pub charge: usize,
}
pub(crate) struct ClipAdmission<'a> {
    clip: &'a Clip,
    map: &'a TimeMap,
    offset: R,
    pub charge: usize,
    pub visits: usize,
}
/// Inspect borrowed anchors first, so the mixer can evict before allocating a
/// new schedule. Unit-rate maps need only a negative lookup, regardless of size.
pub(crate) fn admit_clip<'a>(
    document: &'a CompiledDocument,
    id: &ClipId,
    bits: u64,
    max_visits: usize,
    check: &dyn Fn() -> bool,
) -> Result<ClipAdmission<'a>> {
    cancel(check)?;
    let clip = document
        .document()
        .clips
        .get(id)
        .ok_or(AudioError::Invalid("retime clip"))?;
    let visible = match &clip.source {
        ClipSource::Media { time_map, .. } | ClipSource::Sequence { time_map, .. } => time_map,
        _ => return Err(AudioError::Invalid("retime mapped source")),
    };
    let (map, offset) = clip
        .audio_context
        .as_ref()
        .map_or((visible, R::integer(0)), |c| {
            (&c.time_map, R::from_time(c.offset))
        });
    // Conservative work allowance for admission, run detection, anchor
    // projection and schedule validation/normalization.
    let visits = map.anchors.len().checked_mul(6).ok_or(AudioError::Budget)?;
    if visits > max_visits {
        return Err(AudioError::Budget);
    }
    let mut non_unit = false;
    for pair in map.anchors.windows(2) {
        cancel(check)?;
        let delta = R::from_time(pair[1].source).sub(&R::from_time(pair[0].source), bits)?;
        let direction = delta.compare(&R::integer(0));
        if direction.is_eq()
            || (direction.is_lt() && clip.audio.reverse == ReverseAudioPolicy::Mute)
        {
            continue;
        }
        let span = if direction.is_lt() {
            R::integer(0).sub(&delta, bits)?
        } else {
            delta
        };
        let duration = R::from_time(pair[1].local).sub(&R::from_time(pair[0].local), bits)?;
        non_unit |= span.compare(&duration).is_ne();
    }
    // Conservative admission includes anchor/rate vectors, allocation capacity,
    // source descriptors and run metadata. Host request/DSP ownership is separate.
    let charge = if non_unit {
        map.anchors
            .len()
            .checked_mul(1024 + bits as usize * 2)
            .and_then(|n| n.checked_add(2048))
            .ok_or(AudioError::Budget)?
    } else {
        NEGATIVE_PLAN_BYTES
    };
    Ok(ClipAdmission {
        clip,
        map,
        offset,
        charge,
        visits,
    })
}
pub(crate) fn compile_clip(
    document: &CompiledDocument,
    source: &AudioSpanSource,
    admission: ClipAdmission<'_>,
    bits: u64,
    check: &dyn Fn() -> bool,
) -> Result<CompiledRetime> {
    let ClipAdmission {
        clip,
        map,
        offset,
        charge,
        ..
    } = admission;
    if charge == NEGATIVE_PLAN_BYTES {
        return Ok(CompiledRetime {
            plans: vec![],
            charge,
        });
    }
    let sample_rate = document.document().audio_sample_rate;
    let sr = R::integer(i64::from(sample_rate));
    let origin = R::from_time(clip.timeline.start).sub(&offset, bits)?;
    let identity = source_identity(document, source)?;
    let mut plans = Vec::new();
    let mut index = 0;
    while index + 1 < map.anchors.len() {
        cancel(check)?;
        let direction = match map.anchors[index + 1]
            .source
            .compare_time(map.anchors[index].source)
        {
            std::cmp::Ordering::Greater => 1,
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => {
                index += 1;
                continue;
            }
        };
        let start = index;
        let mut non_unit = false;
        while index + 1 < map.anchors.len() {
            cancel(check)?;
            let a = map.anchors[index];
            let b = map.anchors[index + 1];
            let span = R::from_time(b.source)
                .sub(&R::from_time(a.source), bits)?
                .mul(&R::integer(direction), bits)?;
            if span.compare(&R::integer(0)).is_le() {
                break;
            }
            let duration = R::from_time(b.local).sub(&R::from_time(a.local), bits)?;
            non_unit |= span.compare(&duration).is_ne();
            index += 1;
        }
        if !non_unit || (direction < 0 && clip.audio.reverse == ReverseAudioPolicy::Mute) {
            continue;
        }
        let first = origin
            .add(&R::from_time(map.anchors[start].local), bits)?
            .mul(&sr, bits)?
            .round_integer(Rounding::Ceil)?;
        let end_time = origin
            .add(&R::from_time(map.anchors[index].local), bits)?
            .mul(&sr, bits)?;
        if end_time.compare(&R::integer(first)).is_le() {
            continue;
        }
        let local = R::integer(first).div(&sr, bits)?.sub(&origin, bits)?;
        let input_zero = map
            .sample(
                &TimeSample {
                    time: local,
                    side: SampleSide::After,
                },
                bits,
            )?
            .ok_or(AudioError::Invalid("retime run origin"))?
            .sample
            .time
            .normalized();
        let mut anchors = vec![StretchAnchor {
            output: R::integer(0),
            input: R::integer(0),
        }];
        for a in &map.anchors[start + 1..=index] {
            cancel(check)?;
            let output = origin
                .add(&R::from_time(a.local), bits)?
                .mul(&sr, bits)?
                .sub(&R::integer(first), bits)?;
            if output.compare(&R::integer(0)).is_gt() {
                anchors.push(StretchAnchor {
                    output,
                    input: R::from_time(a.source)
                        .sub(&input_zero, bits)?
                        .mul(&sr, bits)?
                        .mul(&R::integer(direction), bits)?,
                });
            }
        }
        let schedule = StretchSchedule::new(anchors, bits, check)?;
        first
            .checked_add(schedule.output_frames() as i64)
            .ok_or(AudioError::Budget)?;
        plans.push(Arc::new(RetimePlan {
            clip: clip.id.clone(),
            source: source.clone(),
            source_identity: identity.clone(),
            input_zero,
            direction,
            first_sample: first,
            sample_rate,
            schedule,
        }));
    }
    // A unit-rate clip stores only the negative lookup, not an anchor-sized cache.
    Ok(CompiledRetime {
        charge: if plans.is_empty() {
            NEGATIVE_PLAN_BYTES
        } else {
            charge
        },
        plans,
    })
}
