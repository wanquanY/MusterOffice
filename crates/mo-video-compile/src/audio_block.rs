//! Span scheduling is independent of PCM processing. Child sequences remain
//! stages and are queried on their own sample lattice; pitch policies are never
//! flattened into a single net derivative.
use crate::{CompileError, CompiledDocument, Computed, MediaBinding, QueryLimits, Result, Work};
use mo_common::{Digest, RationalTime, Ticks};
use mo_time::{ExactRatio as R, Rounding};
use mo_video_model::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioBlockPlan {
    pub document_digest: Digest,
    pub sequence: SequenceId,
    pub sample_rate: u32,
    pub first_sample: Ticks,
    pub frames: u32,
    pub voices: Vec<AudioSpanVoice>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioSpanVoice {
    pub track: TrackId,
    pub clip: ClipId,
    /// Absolute indices in this sequence's output lattice, not a block-local phase.
    pub first_sample: Ticks,
    pub frames: u32,
    pub local_start: R,
    /// At first_sample. Rate is constant until the next authored time-map anchor.
    pub mapping: MappedTime,
    pub policy: AudioTimePolicy,
    pub effects: Vec<AudioSpanEffect>,
    pub source: AudioSpanSource,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum AudioSpanSource {
    Media { binding: MediaBinding },
    Sequence { sequence: SequenceId },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum AudioSpanEffect {
    Gain {
        #[serde(rename = "milliDecibels")]
        milli_decibels: i32,
    },
    /// Preserve the authored envelope, including negative local phase after a
    /// split/trim. The DSP evaluates at the absolute local sample, not block time.
    Fade {
        #[serde(rename = "fadeIn")]
        fade_in: RationalTime,
        #[serde(rename = "fadeOut")]
        fade_out: RationalTime,
        envelope: TimeRange,
    },
}
impl CompiledDocument {
    pub fn audio_block(
        &self,
        first: i64,
        frames: u32,
        limits: QueryLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Computed<AudioBlockPlan>> {
        let end = first
            .checked_add(i64::from(frames))
            .ok_or(CompileError::Outside)?;
        if first < 0 || end > self.audio_sample_count()? {
            return Err(CompileError::Outside);
        }
        self.audio_sequence_block(&self.document.root_sequence, first, frames, limits, check)
    }
    /// Allows negative/outside-sequence windows to support filter pre/post-roll.
    /// They contain explicit silence except where a real clip covers a sample.
    pub fn audio_sequence_block(
        &self,
        sequence: &SequenceId,
        first: i64,
        frames: u32,
        limits: QueryLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Computed<AudioBlockPlan>> {
        if frames == 0 || frames > 65536 {
            return Err(CompileError::Limits);
        }
        let end = first
            .checked_add(i64::from(frames))
            .ok_or(CompileError::Outside)?;
        let sequence_index = self
            .sequences
            .get(sequence)
            .ok_or(CompileError::Invariant("sequence"))?;
        let mut work = Work::new(limits, check)?;
        let sample_rate = self.document.audio_sample_rate;
        let sr = R::integer(i64::from(sample_rate));
        let begin_time = R::integer(first).div(&sr, self.bits)?;
        let end_time = R::integer(end).div(&sr, self.bits)?;
        let mut voices = vec![];
        for tid in &sequence_index.audio_tracks {
            work.visit()?;
            let track = &self.document.tracks[tid];
            for position in self.tracks[tid].range(&begin_time, &end_time, &mut work)? {
                let clip = &self.document.clips[&track.clips[position]];
                let map = match &clip.source {
                    ClipSource::Media { time_map, .. } | ClipSource::Sequence { time_map, .. } => {
                        time_map
                    }
                    _ => return Err(CompileError::Invariant("non-audio source")),
                };
                let start = R::from_time(clip.timeline.start);
                let local_begin = begin_time.sub(&start, self.bits)?;
                let right = map
                    .anchors
                    .partition_point(|a| R::from_time(a.local).compare(&local_begin).is_le());
                for pair in map.anchors[right.saturating_sub(1)..].windows(2) {
                    work.visit()?;
                    let a = start.add(&R::from_time(pair[0].local), self.bits)?;
                    let b = start.add(&R::from_time(pair[1].local), self.bits)?;
                    if a.compare(&end_time).is_ge() {
                        break;
                    }
                    let lo = a
                        .mul(&sr, self.bits)?
                        .round_integer(Rounding::Ceil)?
                        .max(first);
                    let hi = b
                        .mul(&sr, self.bits)?
                        .round_integer(Rounding::Ceil)?
                        .min(end);
                    if lo >= hi {
                        continue;
                    }
                    work.emit(1)?;
                    let local_start = R::integer(lo).div(&sr, self.bits)?.sub(&start, self.bits)?;
                    let mapping = map
                        .sample(
                            &TimeSample {
                                time: local_start.clone(),
                                side: SampleSide::After,
                            },
                            self.bits,
                        )?
                        .ok_or(CompileError::Invariant("audio span mapping"))?;
                    match mapping.rate.compare(&R::integer(0)) {
                        std::cmp::Ordering::Equal => continue, // The only current hold policy is silence.
                        std::cmp::Ordering::Less
                            if clip.audio.reverse == ReverseAudioPolicy::Mute =>
                        {
                            continue;
                        }
                        _ => (),
                    }
                    work.bytes(1024 + self.bits as usize * 4)?;
                    let mut effects = Vec::new();
                    for e in &clip.effects {
                        work.visit()?;
                        work.bytes(256 + self.bits as usize)?;
                        effects.push(match e {
                            Effect::Gain { milli_decibels } => AudioSpanEffect::Gain {
                                milli_decibels: *milli_decibels,
                            },
                            Effect::Fade {
                                fade_in,
                                fade_out,
                                envelope,
                            } => AudioSpanEffect::Fade {
                                fade_in: *fade_in,
                                fade_out: *fade_out,
                                envelope: *envelope,
                            },
                            _ => {
                                return Err(CompileError::Invariant("visual effect on audio span"));
                            }
                        });
                    }
                    let source = match &clip.source {
                        ClipSource::Media {
                            resource, stream, ..
                        } => AudioSpanSource::Media {
                            binding: self.binding(resource, stream)?,
                        },
                        ClipSource::Sequence { sequence, .. } => AudioSpanSource::Sequence {
                            sequence: sequence.clone(),
                        },
                        _ => unreachable!("audio source admitted above"),
                    };
                    voices.push(AudioSpanVoice {
                        track: tid.clone(),
                        clip: clip.id.clone(),
                        first_sample: Ticks::new(lo),
                        frames: (hi - lo) as u32,
                        local_start,
                        mapping,
                        policy: clip.audio,
                        effects,
                        source,
                    });
                }
            }
        }
        self.finish(
            AudioBlockPlan {
                document_digest: self.digest.clone(),
                sequence: sequence.clone(),
                sample_rate,
                first_sample: Ticks::new(first),
                frames,
                voices,
            },
            limits,
            work,
        )
    }
}
