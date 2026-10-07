use crate::{LinearImage, RenderError, Result, SourceGeometry};
use mo_time::{ExactInterval, ExactRatio};
use mo_video_compile::MediaBinding;
use mo_video_model::{MediaStream, TimeSample};
use std::sync::Arc;

#[derive(Clone)]
pub struct VisualFrame {
    pub image: Arc<LinearImage>,
    pub geometry: SourceGeometry,
}
#[derive(Clone)]
pub enum ResolvedVisual {
    VideoFrame {
        interval: ExactInterval,
        frame: VisualFrame,
    },
    /// A real source-timeline gap, with no pixels. Lower layers remain visible.
    /// Missing resources and decoder failures must be returned as errors instead.
    VideoGap {
        interval: ExactInterval,
    },
    Image {
        frame: VisualFrame,
    },
}
#[derive(Clone)]
pub struct ResolvedMedia {
    pub binding: MediaBinding,
    pub visual: ResolvedVisual,
}
pub trait VisualResources {
    fn media(
        &mut self,
        binding: &MediaBinding,
        sample: &TimeSample,
        check: &dyn Fn() -> bool,
    ) -> Result<ResolvedMedia>;
}
impl ResolvedMedia {
    /// Shared by every composition backend, including the no-pixel gap path.
    pub(crate) fn validate(
        self,
        binding: &MediaBinding,
        sample: &TimeSample,
    ) -> Result<Option<VisualFrame>> {
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
