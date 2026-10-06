use crate::{contract::*, effects, index::IntervalIndex, *};
use mo_common::{Digest, JsonBudgetError, Ticks, check_json_size};
use mo_time::{ExactRatio as R, Rounding};
use mo_video_model::*;
use std::collections::BTreeMap;

/// An immutable validated document and its temporal indexes. Rebuild after an accepted edit;
/// the host owns cache eviction. This first compiler does not claim incremental recompilation.
#[derive(Debug)]
pub struct CompiledDocument {
    pub(crate) document: VideoDocument,
    pub(crate) digest: Digest,
    pub(crate) tracks: BTreeMap<TrackId, IntervalIndex>,
    pub(crate) sequences: BTreeMap<SequenceId, SequenceIndex>,
    pub(crate) bits: u64,
}
#[derive(Debug)]
pub(crate) struct SequenceIndex {
    duration: R,
    visual_tracks: Vec<TrackId>,
    pub(crate) audio_tracks: Vec<TrackId>,
}

impl CompiledDocument {
    pub fn new(
        document: VideoDocument,
        limits: CompileLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self> {
        let report = validate(&document, limits.validation, check)?;
        if !report.is_valid() {
            return Err(CompileError::Invalid(report));
        }
        let mut tracks = BTreeMap::new();
        for (id, track) in &document.tracks {
            let mut entries = Vec::with_capacity(track.clips.len());
            for (order, clip) in track.clips.iter().enumerate() {
                if check() {
                    return Err(CompileError::Cancelled);
                }
                entries.push((
                    order,
                    document.clips.get(clip).expect("validated clip").timeline,
                ));
            }
            tracks.insert(id.clone(), IntervalIndex::new(entries, check)?);
        }
        let mut sequences = BTreeMap::new();
        for (id, sequence) in &document.sequences {
            let mut duration = R::integer(0);
            let mut visual_tracks = vec![];
            let mut audio_tracks = vec![];
            for tid in &sequence.tracks {
                if check() {
                    return Err(CompileError::Cancelled);
                }
                let track = document.tracks.get(tid).expect("validated track");
                for cid in &track.clips {
                    if check() {
                        return Err(CompileError::Cancelled);
                    }
                    let end = R::from_time(
                        document
                            .clips
                            .get(cid)
                            .expect("validated clip")
                            .timeline
                            .end,
                    );
                    if end.compare(&duration).is_gt() {
                        duration = end;
                    }
                }
                if !track.muted {
                    if track.kind == TrackKind::Audio {
                        audio_tracks.push(tid.clone());
                    } else {
                        visual_tracks.push(tid.clone());
                    }
                }
            }
            // Solo is scoped to each bus in this sequence. Muted solo tracks do not solo silence.
            for bus in [&mut visual_tracks, &mut audio_tracks] {
                if bus
                    .iter()
                    .any(|t| document.tracks.get(t).expect("validated track").solo)
                {
                    bus.retain(|t| document.tracks.get(t).expect("validated track").solo);
                }
            }
            sequences.insert(
                id.clone(),
                SequenceIndex {
                    duration,
                    visual_tracks,
                    audio_tracks,
                },
            );
        }
        if check() {
            return Err(CompileError::Cancelled);
        }
        let digest = document.content_digest()?;
        if check() {
            return Err(CompileError::Cancelled);
        }
        Ok(Self {
            document,
            digest,
            tracks,
            sequences,
            bits: limits.validation.exact_bits,
        })
    }
    pub fn document(&self) -> &VideoDocument {
        &self.document
    }
    pub fn digest(&self) -> &Digest {
        &self.digest
    }
    pub fn duration(&self) -> &R {
        &self.sequences[&self.document.root_sequence].duration
    }
    /// Every output frame whose absolute start lies before the project end is included.
    pub fn frame_count(&self) -> Result<i64> {
        let rate = self.document.frame_rate;
        let period = rate.frame_time(1, self.bits)?;
        Ok(self
            .duration()
            .div(&period, self.bits)?
            .round_integer(Rounding::Ceil)?)
    }
    pub fn audio_sample_count(&self) -> Result<i64> {
        Ok(self
            .duration()
            .mul(
                &R::integer(i64::from(self.document.audio_sample_rate)),
                self.bits,
            )?
            .round_integer(Rounding::Ceil)?)
    }
    pub fn frame(
        &self,
        index: i64,
        limits: QueryLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Computed<VideoFramePlan>> {
        let mut work = Work::new(limits, check)?;
        let sample = TimeSample {
            time: self.document.frame_rate.frame_time(index, self.bits)?,
            side: SampleSide::After,
        };
        if index < 0 || sample.time.compare(self.duration()).is_ge() {
            return Err(CompileError::Outside);
        }
        let layers = self.visual(&self.document.root_sequence, &sample, 1, &mut work)?;
        self.finish(
            VideoFramePlan {
                document_digest: self.digest.clone(),
                frame_index: Ticks::new(index),
                sample,
                canvas: self.document.canvas,
                layers,
            },
            limits,
            work,
        )
    }
    pub fn visual_at(
        &self,
        sample: &TimeSample,
        limits: QueryLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Computed<Vec<VisualNode>>> {
        sample.time.check_budget(self.bits)?;
        let mut work = Work::new(limits, check)?;
        let layers = self.visual(&self.document.root_sequence, sample, 1, &mut work)?;
        self.finish(layers, limits, work)
    }
    /// Semantic point query for diagnostics and cross-target tests. Not the real-time PCM loop.
    pub fn audio_sample(
        &self,
        index: i64,
        limits: QueryLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Computed<AudioSamplePlan>> {
        let mut work = Work::new(limits, check)?;
        let sample = TimeSample {
            time: R::integer(index).div(
                &R::integer(i64::from(self.document.audio_sample_rate)),
                self.bits,
            )?,
            side: SampleSide::After,
        };
        if index < 0 || sample.time.compare(self.duration()).is_ge() {
            return Err(CompileError::Outside);
        }
        let voices = self.audio(&self.document.root_sequence, &sample, 1, &mut work)?;
        self.finish(
            AudioSamplePlan {
                document_digest: self.digest.clone(),
                sample_index: Ticks::new(index),
                sample_rate: self.document.audio_sample_rate,
                sample,
                voices,
            },
            limits,
            work,
        )
    }
    pub fn audio_at(
        &self,
        sample: &TimeSample,
        limits: QueryLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Computed<Vec<AudioNode>>> {
        sample.time.check_budget(self.bits)?;
        let mut work = Work::new(limits, check)?;
        let voices = self.audio(&self.document.root_sequence, sample, 1, &mut work)?;
        self.finish(voices, limits, work)
    }
    pub(crate) fn finish<T: serde::Serialize>(
        &self,
        value: T,
        limits: QueryLimits,
        work: Work<'_>,
    ) -> Result<Computed<T>> {
        check_json_size(&value, limits.max_output_bytes, work.check).map_err(|e| match e {
            JsonBudgetError::Cancelled => CompileError::Cancelled,
            _ => CompileError::Budget,
        })?;
        Ok(Computed {
            value,
            stats: work.stats,
        })
    }
    fn local(&self, clip: &Clip, sample: &TimeSample) -> Result<TimeSample> {
        Ok(TimeSample {
            time: sample
                .time
                .sub(&R::from_time(clip.timeline.start), self.bits)?,
            side: sample.side,
        })
    }
    fn mapping(&self, map: &TimeMap, local: &TimeSample) -> Result<MappedTime> {
        map.sample(local, self.bits)?
            .ok_or(CompileError::Invariant("selected clip lacks time mapping"))
    }
    pub(crate) fn binding(
        &self,
        resource: &mo_common::ResourceId,
        stream: &StreamId,
    ) -> Result<MediaBinding> {
        let r = self
            .document
            .resources
            .get(resource)
            .ok_or(CompileError::Invariant("resource"))?;
        let ResourceDescriptor::Media { streams } = &r.descriptor else {
            return Err(CompileError::Invariant("media descriptor"));
        };
        let descriptor = streams
            .get(stream)
            .ok_or(CompileError::Invariant("stream"))?
            .clone();
        Ok(MediaBinding {
            resource: resource.clone(),
            digest: r.digest.clone(),
            stream: stream.clone(),
            descriptor,
        })
    }
    fn node_budget(&self, depth: usize, work: &mut Work<'_>) -> Result<()> {
        work.emit(depth)?;
        // Conservative charge for bounded identifiers and computed fractions; variable text and
        // effects are additionally charged before cloning. Final serialized size is also checked.
        work.bytes(1024 + self.bits as usize * 4)
    }
    fn visual(
        &self,
        sequence: &SequenceId,
        sample: &TimeSample,
        depth: usize,
        work: &mut Work<'_>,
    ) -> Result<Vec<VisualNode>> {
        work.visit()?;
        let mut layers = vec![];
        for tid in &self.sequences[sequence].visual_tracks {
            work.visit()?;
            let track = &self.document.tracks[tid];
            for position in self.tracks[tid].at(sample, work)? {
                self.node_budget(depth, work)?;
                let clip = &self.document.clips[&track.clips[position]];
                let local = self.local(clip, sample)?;
                let effects = effects::visual(&clip.effects, &local, self.bits, work)?;
                let source = match &clip.source {
                    ClipSource::Media {
                        resource,
                        stream,
                        time_map,
                    } => VisualSource::Media {
                        binding: self.binding(resource, stream)?,
                        mapping: self.mapping(time_map, &local)?,
                    },
                    ClipSource::Sequence { sequence, time_map } => {
                        let mapping = self.mapping(time_map, &local)?;
                        let children = self.visual(sequence, &mapping.sample, depth + 1, work)?;
                        VisualSource::Group {
                            sequence: sequence.clone(),
                            mapping,
                            layers: children,
                        }
                    }
                    ClipSource::Solid { rgba } => VisualSource::Solid { rgba: *rgba },
                    ClipSource::Text {
                        text,
                        font,
                        font_size_milli_pixels,
                        rgba,
                    } => {
                        work.bytes(text.len())?;
                        let resource = &self.document.resources[font];
                        let ResourceDescriptor::Font { face_index } = resource.descriptor else {
                            return Err(CompileError::Invariant("font"));
                        };
                        VisualSource::Text {
                            text: text.clone(),
                            font: font.clone(),
                            font_digest: resource.digest.clone(),
                            face_index,
                            font_size_milli_pixels: *font_size_milli_pixels,
                            rgba: *rgba,
                        }
                    }
                };
                layers.push(VisualNode {
                    track: tid.clone(),
                    clip: clip.id.clone(),
                    local,
                    effects,
                    source,
                });
            }
        }
        Ok(layers)
    }
    fn audio(
        &self,
        sequence: &SequenceId,
        sample: &TimeSample,
        depth: usize,
        work: &mut Work<'_>,
    ) -> Result<Vec<AudioNode>> {
        work.visit()?;
        let mut voices = vec![];
        for tid in &self.sequences[sequence].audio_tracks {
            work.visit()?;
            let track = &self.document.tracks[tid];
            for position in self.tracks[tid].at(sample, work)? {
                self.node_budget(depth, work)?;
                let clip = &self.document.clips[&track.clips[position]];
                let local = self.local(clip, sample)?;
                let map = match &clip.source {
                    ClipSource::Media { time_map, .. } | ClipSource::Sequence { time_map, .. } => {
                        time_map
                    }
                    _ => return Err(CompileError::Invariant("non-audio source")),
                };
                let mapping = self.mapping(map, &local)?;
                match mapping.rate.compare(&R::integer(0)) {
                    std::cmp::Ordering::Less if clip.audio.reverse == ReverseAudioPolicy::Mute => {
                        continue;
                    }
                    std::cmp::Ordering::Equal => match clip.audio.hold {
                        HoldAudioPolicy::Silence => continue,
                    },
                    _ => {}
                }
                let effects = effects::audio(&clip.effects, &local, self.bits, work)?;
                let source = match &clip.source {
                    ClipSource::Media {
                        resource, stream, ..
                    } => AudioSource::Media {
                        binding: self.binding(resource, stream)?,
                    },
                    ClipSource::Sequence { sequence, .. } => AudioSource::Group {
                        sequence: sequence.clone(),
                        voices: self.audio(sequence, &mapping.sample, depth + 1, work)?,
                    },
                    _ => return Err(CompileError::Invariant("non-audio source")),
                };
                voices.push(AudioNode {
                    track: tid.clone(),
                    clip: clip.id.clone(),
                    local,
                    mapping,
                    policy: clip.audio,
                    effects,
                    source,
                });
            }
        }
        Ok(voices)
    }
}
