//! Indexed pictures to shared composition resources, independent of host I/O and jobs.
mod cache;
use crate::{
    authoring::model::TimeSample,
    compile::MediaBinding,
    render::{
        HlgDisplay, InputColor, PixelLimits, RenderError, ResolvedMedia, ResolvedVisual, Result,
        SourceGeometry, VisualFrame, VisualResources, normalize_frame,
    },
    source::{SampleMode, SourceError, SourceStats, indexed::*},
};
use std::{
    cell::Cell,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::Arc,
};

#[derive(Debug, Clone, Copy)]
pub enum ColorInterpretation {
    /// Use each decoded frame's declarations. Missing metadata fails explicitly.
    Declared { hlg: Option<HlgDisplay> },
    /// Explicit host interpretation, bound for this source's entire lifetime.
    Override(InputColor),
}
#[derive(Debug, Clone, Copy)]
pub struct VisualSourceLimits {
    pub pixels: PixelLimits,
    /// Normalized RGBA32F image storage, reserved before conversion.
    /// Excludes raw sampler, decoder/transport, map metadata and caller-held Arcs.
    pub max_retained_bytes: usize,
    pub max_retained_frames: usize,
}
impl Default for VisualSourceLimits {
    fn default() -> Self {
        Self {
            pixels: PixelLimits::default(),
            max_retained_bytes: 256 << 20,
            max_retained_frames: 8,
        }
    }
}
#[derive(Debug, Default, Clone, Copy)]
pub struct VisualSourceStats {
    pub source: SourceStats,
    pub normalized: bool,
    pub normalized_cache_hit: bool,
    pub retained_bytes: usize,
}
/// One immutable stream. Host resource routing composes any number of these;
/// it must bind the sampler's reader and timeline to the declared content digest.
/// Every request revalidates timing through the sampler, even on a color-cache hit.
/// Any failed call drops both media capabilities and all owned cached images.
pub struct IndexedVisualSource<D: IndexedVideoDecoder, T: VideoTimeline> {
    sampler: Option<IndexedVideoSampler<D, T>>,
    interpretation: ColorInterpretation,
    limits: VisualSourceLimits,
    cache: cache::Cache,
    last_stats: VisualSourceStats,
}
impl<D: IndexedVideoDecoder, T: VideoTimeline> IndexedVisualSource<D, T> {
    pub fn new(
        sampler: IndexedVideoSampler<D, T>,
        interpretation: ColorInterpretation,
        limits: VisualSourceLimits,
    ) -> Result<Self> {
        if sampler.is_stopped() {
            return Err(RenderError::Stopped);
        }
        if !(1..=4096).contains(&limits.max_retained_frames)
            || limits.max_retained_bytes == 0
            || limits.max_retained_bytes > 1 << 30
            || limits.pixels.max_pixels == 0
            || limits.pixels.max_bytes == 0
        {
            return Err(RenderError::Invalid("visual source limits"));
        }
        Ok(Self {
            sampler: Some(sampler),
            interpretation,
            limits,
            cache: cache::Cache::default(),
            last_stats: VisualSourceStats::default(),
        })
    }
    pub fn is_stopped(&self) -> bool {
        self.sampler.is_none()
    }
    pub fn retained_bytes(&self) -> usize {
        self.cache.bytes
    }
    /// Stats from the most recent successful resolve. Failures reset these.
    pub fn last_stats(&self) -> VisualSourceStats {
        self.last_stats
    }
    pub fn abort(&mut self) {
        self.sampler.take();
        self.cache = cache::Cache::default();
        self.last_stats = VisualSourceStats::default();
    }
    pub fn resolve(
        &mut self,
        binding: &MediaBinding,
        sample: &TimeSample,
        mode: SampleMode,
        check: &dyn Fn() -> bool,
    ) -> Result<ResolvedMedia> {
        if self.is_stopped() {
            return Err(RenderError::Stopped);
        }
        // Latch cancellation across opaque decoder/timeline error types and all conversion steps.
        let cancellation = Cell::new(false);
        let cancelled = || {
            let value = cancellation.get() || check();
            cancellation.set(value);
            value
        };
        let result = match catch_unwind(AssertUnwindSafe(|| {
            self.resolve_inner(binding, sample, mode, &cancelled)
        })) {
            Ok(Err(_)) if cancellation.get() => Err(RenderError::Cancelled),
            Ok(value) => value,
            Err(payload) => {
                std::mem::forget(payload);
                Err(RenderError::CallbackPanic)
            }
        };
        if result.is_err() {
            self.abort();
        }
        result
    }
    fn resolve_inner(
        &mut self,
        binding: &MediaBinding,
        sample: &TimeSample,
        mode: SampleMode,
        check: &dyn Fn() -> bool,
    ) -> Result<ResolvedMedia> {
        active(check)?;
        let sampler = self.sampler.as_mut().ok_or(RenderError::Stopped)?;
        let selected = sampler
            .sample(binding, sample, mode, check)
            .map_err(source_error)?;
        active(check)?;
        let mut stats = VisualSourceStats {
            source: selected.stats,
            ..Default::default()
        };
        let visual = match selected.selection() {
            VideoTimeSelection::Outside => {
                return Err(RenderError::Invalid("visual sample outside source"));
            }
            VideoTimeSelection::Gap { movie, .. } => ResolvedVisual::VideoGap {
                interval: movie.clone(),
            },
            VideoTimeSelection::Sample {
                movie, raw_start, ..
            } => {
                let frame = if let Some(frame) = self.cache.get(*raw_start)? {
                    stats.normalized_cache_hit = true;
                    frame
                } else {
                    let decoded = selected
                        .frame()
                        .ok_or(RenderError::Invalid("indexed picture missing"))?;
                    let width = decoded
                        .info
                        .width
                        .ok_or(RenderError::Invalid("picture width"))?;
                    let height = decoded
                        .info
                        .height
                        .ok_or(RenderError::Invalid("picture height"))?;
                    let pixels = u64::from(width) * u64::from(height);
                    let bytes = usize::try_from(pixels.checked_mul(16).ok_or(RenderError::Budget)?)
                        .map_err(|_| RenderError::Budget)?;
                    if pixels > self.limits.pixels.max_pixels as u64
                        || bytes > self.limits.pixels.max_bytes
                    {
                        return Err(RenderError::Budget);
                    }
                    self.cache.reserve(bytes, self.limits)?;
                    let color = match self.interpretation {
                        ColorInterpretation::Declared { hlg } => {
                            InputColor::from_frame(&decoded.info, hlg)?
                        }
                        ColorInterpretation::Override(color) => color,
                    };
                    let geometry = SourceGeometry::from_media(&decoded.info, sampler.stream())?;
                    let image =
                        Arc::new(normalize_frame(decoded, color, self.limits.pixels, check)?.image);
                    let frame = VisualFrame { image, geometry };
                    self.cache.insert(*raw_start, bytes, frame.clone())?;
                    stats.normalized = true;
                    frame
                };
                ResolvedVisual::VideoFrame {
                    interval: movie.clone(),
                    frame,
                }
            }
        };
        active(check)?;
        stats.retained_bytes = self.cache.bytes;
        self.last_stats = stats;
        Ok(ResolvedMedia {
            binding: binding.clone(),
            visual,
        })
    }
}
impl<D: IndexedVideoDecoder, T: VideoTimeline> VisualResources for IndexedVisualSource<D, T> {
    fn media(
        &mut self,
        binding: &MediaBinding,
        sample: &TimeSample,
        check: &dyn Fn() -> bool,
    ) -> Result<ResolvedMedia> {
        self.resolve(binding, sample, SampleMode::Continue, check)
    }
}
fn active(check: &dyn Fn() -> bool) -> Result<()> {
    if check() {
        Err(RenderError::Cancelled)
    } else {
        Ok(())
    }
}
fn source_error<D: std::error::Error + 'static, T: std::error::Error + 'static>(
    error: IndexedSourceError<D, T>,
) -> RenderError {
    match error {
        IndexedSourceError::Source(error) => match error {
            SourceError::Cancelled => RenderError::Cancelled,
            SourceError::CallbackPanic => RenderError::CallbackPanic,
            SourceError::Budget => RenderError::Budget,
            SourceError::Stopped => RenderError::Stopped,
            SourceError::Invalid(reason) => RenderError::Invalid(reason),
            SourceError::Unsupported(reason) => RenderError::Unsupported(reason),
            SourceError::Time(error) => RenderError::Time(error),
            error => RenderError::Resource(error.to_string()),
        },
        IndexedSourceError::Timing(error) => match error {
            TimingError::Cancelled => RenderError::Cancelled,
            TimingError::CallbackPanic => RenderError::CallbackPanic,
            TimingError::Budget => RenderError::Budget,
            TimingError::Invalid(reason) => RenderError::Invalid(reason),
            TimingError::Unsupported(reason) => RenderError::Unsupported(reason),
            TimingError::Time(error) => RenderError::Time(error),
        },
        IndexedSourceError::Timeline(error) => RenderError::Resource(error.to_string()),
    }
}
