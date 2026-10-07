//! Container edits over verified canonical PCM. Storage and decoding remain
//! explicit host capabilities; this adapter owns exact mapping and DSP only.
mod reader;
use crate::{audio::*, compile::MediaBinding, isobmff::movie::MovieTimeline, pcm::InspectedPcm};
use mo_time::{ExactRatio as R, Rounding};
use mo_video_operations::model::{MediaStream, TimeRange};

/// Include this mapping policy and the raw decoder/PCM profile when a host
/// materializes or caches movie-time audio. Raw content identity alone is not
/// an identity for a processed audio result.
pub const PROFILE: &str = "musteroffice.movie-audio.raw-pcm-sinc-edits/1-draft";

#[derive(Debug, Clone, Copy)]
pub struct MovieAudioLimits {
    pub max_segments: usize,
    pub exact_bits: u64,
    pub max_block_frames: usize,
    /// Output, requested source window, resampled block and retained filter.
    /// Host tile caches and caller-owned PCM are separately accounted by host.
    pub max_working_bytes: usize,
    pub resample: ResampleLimits,
}
impl Default for MovieAudioLimits {
    fn default() -> Self {
        Self {
            max_segments: 4096,
            exact_bits: 256,
            max_block_frames: 8192,
            max_working_bytes: 64 << 20,
            resample: ResampleLimits::default(),
        }
    }
}
struct Segment {
    first: i64,
    end: i64,
    source: Option<Source>,
}
struct Source {
    first: R,
    step: R,
    available: PcmRange,
}
/// Immutable plan with movie-time output and raw-media input. Its descriptor
/// preserves exact movie end even when it falls between output sample instants.
pub struct MovieAudioPlan {
    raw_binding: MediaBinding,
    raw_asset: AudioAsset,
    asset: AudioAsset,
    descriptor: MediaStream,
    segments: Vec<Segment>,
    limits: MovieAudioLimits,
}
impl MovieAudioPlan {
    /// The host must bind `timeline` to the same verified content and selected
    /// container track as `raw_binding`, and use a fresh RAW decode inspection.
    /// A demuxer that already applied these edits would apply them twice.
    pub fn new(
        timeline: &MovieTimeline,
        raw_binding: MediaBinding,
        decoded: &InspectedPcm,
        limits: MovieAudioLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Self> {
        active(cancelled)?;
        if limits.max_segments > 100_000
            || timeline.segments().len() > limits.max_segments
            || !(128..=4096).contains(&limits.exact_bits)
            || !(1..=65536).contains(&limits.max_block_frames)
            || limits.max_working_bytes == 0
            || limits.max_working_bytes > 1 << 30
        {
            return Err(AudioError::Budget);
        }
        let bits = limits.exact_bits;
        let raw_asset = decoded.asset().clone();
        raw_asset.validate_binding(&raw_binding, bits)?;
        let raw_range = decoded.decoded_range(bits).map_err(pcm_error)?;
        let MediaStream::Audio { available, .. } = &raw_binding.descriptor else {
            return Err(AudioError::Invalid("raw movie PCM binding"));
        };
        if R::from_time(available.start)
            .compare(&R::from_time(raw_range.start))
            .is_ne()
            || R::from_time(available.end)
                .compare(&R::from_time(raw_range.end))
                .is_ne()
        {
            return Err(AudioError::Invalid("raw movie PCM extent binding"));
        }
        if !timeline.end().positive() {
            return Err(AudioError::Invalid("empty audio movie"));
        }
        let rate = R::integer(i64::from(raw_asset.sample_rate));
        let position = |time: &R| -> Result<R> {
            Ok(time.sub(&raw_asset.sample_zero, bits)?.mul(&rate, bits)?)
        };
        let mut segments = Vec::new();
        segments
            .try_reserve_exact(timeline.segments().len())
            .map_err(|_| AudioError::Budget)?;
        for segment in timeline.segments() {
            active(cancelled)?;
            let first = segment
                .movie()
                .start
                .mul(&rate, bits)?
                .round_integer(Rounding::Ceil)?;
            let end = segment
                .movie()
                .end
                .mul(&rate, bits)?
                .round_integer(Rounding::Ceil)?;
            let source = if let Some(origin) = segment.media_origin() {
                if !segment.rate().positive() {
                    return Err(AudioError::Unsupported("container audio dwell policy"));
                }
                let source_end = segment
                    .media_time(&segment.movie().end, bits)
                    .map_err(|e| AudioError::Resource(e.to_string()))?
                    .ok_or(AudioError::Invalid("missing audio edit source"))?;
                if origin.compare(&R::from_time(raw_range.start)).is_lt()
                    || source_end.compare(&R::from_time(raw_range.end)).is_gt()
                {
                    return Err(AudioError::Invalid(
                        "audio edit outside decoded PCM coverage",
                    ));
                }
                let start_sample = position(origin)?.round_integer(Rounding::Ceil)?;
                let end_sample = position(&source_end)?.round_integer(Rounding::Ceil)?;
                let sample_time = R::integer(first).div(&rate, bits)?;
                let source_time = origin.add(
                    &sample_time
                        .sub(&segment.movie().start, bits)?
                        .mul(segment.rate(), bits)?,
                    bits,
                )?;
                Some(Source {
                    first: position(&source_time)?,
                    step: segment.rate().clone(),
                    available: PcmRange::new(start_sample, end_sample)?,
                })
            } else {
                None
            };
            if first < end {
                segments.push(Segment { first, end, source });
            }
        }
        let end = timeline
            .end()
            .mul(&rate, bits)?
            .round_integer(Rounding::Ceil)?;
        let asset = AudioAsset {
            sample_rate: raw_asset.sample_rate,
            layout: raw_asset.layout.clone(),
            sample_zero: R::integer(0),
            available: PcmRange::new(0, end)?,
        };
        let descriptor = MediaStream::Audio {
            available: TimeRange {
                start: R::integer(0).to_time_exact()?,
                end: timeline.end().to_time_exact()?,
            },
            sample_rate: asset.sample_rate,
            channels: asset.layout.len() as u16,
        };
        active(cancelled)?;
        Ok(Self {
            raw_binding,
            raw_asset,
            asset,
            descriptor,
            segments,
            limits,
        })
    }
    pub fn descriptor(&self) -> &MediaStream {
        &self.descriptor
    }
    pub fn asset(&self) -> &AudioAsset {
        &self.asset
    }
}
/// Supplies movie-time PCM to the same mixer/export path as other sources.
/// Empty edits are intentional silence; unavailable/corrupt source data fail.
/// Container rate changes use band-limited varispeed, not preserve-pitch DSP.
/// Filter support is zero-extended at each explicit edit's source boundaries so
/// excluded priming, padding or neighboring speech cannot leak into the edit.
pub struct MovieAudioResource<S> {
    plan: MovieAudioPlan,
    binding: MediaBinding,
    raw: S,
    filter: Option<(R, SincResampler)>,
}
impl<S: AudioResources> MovieAudioResource<S> {
    pub fn new(plan: MovieAudioPlan, binding: MediaBinding, mut raw: S) -> Result<Self> {
        if binding.digest != plan.raw_binding.digest
            || binding.stream != plan.raw_binding.stream
            || binding.descriptor != plan.descriptor
            || raw.describe(&plan.raw_binding)? != plan.raw_asset
        {
            return Err(AudioError::Invalid("movie audio resource binding"));
        }
        plan.asset
            .validate_binding(&binding, plan.limits.exact_bits)?;
        Ok(Self {
            plan,
            binding,
            raw,
            filter: None,
        })
    }
    pub fn retained_filter_bytes(&self) -> usize {
        self.filter.as_ref().map_or(0, |(_, f)| f.kernel_bytes())
    }
    pub fn clear(&mut self) {
        self.filter = None;
    }
    pub fn raw_mut(&mut self) -> &mut S {
        &mut self.raw
    }
}
fn active(cancelled: &dyn Fn() -> bool) -> Result<()> {
    if cancelled() {
        Err(AudioError::Cancelled)
    } else {
        Ok(())
    }
}
fn pcm_error(error: crate::pcm::PcmError) -> AudioError {
    AudioError::Resource(error.to_string())
}
