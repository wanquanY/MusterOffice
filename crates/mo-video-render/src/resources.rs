use crate::{LinearImage, RenderError, Result, SourceGeometry};
use mo_time::{ExactInterval, ExactRatio};
use mo_video_compile::MediaBinding;
use mo_video_model::{MediaStream, TimeSample};
use std::sync::Arc;

/// Metadata of a linear-light premultiplied source. A carrier may own CPU
/// pixels or an opaque host GPU capability; the compositor never forces readback.
/// Hosts must normalize colors into this profile before executing pixel work.
pub trait LinearSource {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn has_hdr(&self) -> bool;
}
impl LinearSource for LinearImage {
    fn width(&self) -> u32 {
        self.width()
    }
    fn height(&self) -> u32 {
        self.height()
    }
    fn has_hdr(&self) -> bool {
        self.has_hdr()
    }
}
impl<T: LinearSource> LinearSource for Arc<T> {
    fn width(&self) -> u32 {
        self.as_ref().width()
    }
    fn height(&self) -> u32 {
        self.as_ref().height()
    }
    fn has_hdr(&self) -> bool {
        self.as_ref().has_hdr()
    }
}
#[derive(Clone)]
pub struct VisualFrame<I = Arc<LinearImage>> {
    pub image: I,
    pub geometry: SourceGeometry,
}
#[derive(Clone)]
pub enum ResolvedVisual<I = Arc<LinearImage>> {
    VideoFrame {
        interval: ExactInterval,
        frame: VisualFrame<I>,
    },
    /// A real source-timeline gap, with no pixels. Lower layers remain visible.
    /// Missing resources and decoder failures must be returned as errors instead.
    VideoGap {
        interval: ExactInterval,
    },
    Image {
        frame: VisualFrame<I>,
    },
}
#[derive(Clone)]
pub struct ResolvedMedia<I = Arc<LinearImage>> {
    pub binding: MediaBinding,
    pub visual: ResolvedVisual<I>,
}
pub trait VisualResources<I = Arc<LinearImage>> {
    fn media(
        &mut self,
        binding: &MediaBinding,
        sample: &TimeSample,
        check: &dyn Fn() -> bool,
    ) -> Result<ResolvedMedia<I>>;
}
impl<I: LinearSource> ResolvedMedia<I> {
    /// Shared by every composition backend, including the no-pixel gap path.
    pub(crate) fn validate(
        self,
        binding: &MediaBinding,
        sample: &TimeSample,
    ) -> Result<Option<VisualFrame<I>>> {
        if &self.binding != binding {
            return Err(RenderError::Invalid("resolved resource binding"));
        }
        sample.time.check_budget(4096)?;
        let (frame, dimensions) = match (self.visual, &binding.descriptor) {
            (
                visual @ (ResolvedVisual::VideoFrame { .. } | ResolvedVisual::VideoGap { .. }),
                MediaStream::Video {
                    available,
                    dimensions,
                    ..
                },
            ) => {
                if !available.valid() || !sample.covers(*available) {
                    return Err(RenderError::Invalid("source sample availability"));
                }
                let (interval, frame) = match visual {
                    ResolvedVisual::VideoFrame { interval, frame } => (interval, Some(frame)),
                    ResolvedVisual::VideoGap { interval } => (interval, None),
                    _ => unreachable!(),
                };
                interval.validate(4096)?;
                if !interval.contains(&sample.time, sample.side)
                    || interval
                        .start
                        .compare(&ExactRatio::from_time(available.start))
                        .is_lt()
                    || interval
                        .end
                        .compare(&ExactRatio::from_time(available.end))
                        .is_gt()
                {
                    return Err(RenderError::Invalid("resolved video presentation interval"));
                }
                (frame, dimensions)
            }
            (ResolvedVisual::Image { frame }, MediaStream::Image { dimensions }) => {
                (Some(frame), dimensions)
            }
            _ => return Err(RenderError::Invalid("resolved visual media kind")),
        };
        if dimensions.width == 0 || dimensions.height == 0 {
            return Err(RenderError::Invalid("resolved visual dimensions"));
        }
        if let Some(frame) = &frame
            && (frame.image.width(), frame.image.height()) != (dimensions.width, dimensions.height)
        {
            return Err(RenderError::Invalid("resolved visual dimensions"));
        }
        Ok(frame)
    }
}
