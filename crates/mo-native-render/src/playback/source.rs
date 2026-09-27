use super::{
    pipe::{Pipe, empty},
    *,
};
use mo_common::RationalTime;
use mo_kernel_api::{
    PlaybackSampleRequest, PptxPlaybackSessionRequest as Q, PptxPlaybackSessionResponse as R,
};
use mo_presentation_delivery::Content;
use std::time::Instant;

/// Source bytes/fonts are sent once. The worker owns decoded/laid-out resources
/// until this value is disposed/dropped; no frame history is retained here.
pub struct SourcePlayback {
    pipe: Pipe<R>,
    info: PptxPlaybackSessionInfo,
    size: (u32, u32),
}
impl SourcePlayback {
    pub(super) fn prepare(
        config: &NativePlayback,
        request: PptxPlaybackPrepareRequest,
        source: Content<'_>,
        fonts: Content<'_>,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, Error> {
        let mut pipe = Pipe::start(config, "--pptx-playback-session", true, check)?;
        let binding = request.binding.clone();
        let slide = request.page.page.slide.clone();
        let size = (
            request.page.page.viewport.width,
            request.page.page.viewport.height,
        );
        let sha = request.page.page.expected_source_sha256.clone();
        let response = pipe.call(
            &Q::Prepare {
                request: Box::new(request),
            },
            source,
            fonts,
            check,
        )?;
        if !response.pixels.is_empty() {
            return pipe.invalid("source prepare contains pixels");
        }
        match response.info {
            R::Prepared { info }
                if info.profile == mo_kernel_api::PPTX_PLAYBACK_SESSION_PROFILE
                    && info.binding == binding
                    && info.slide == slide
                    && info.source_sha256 == sha =>
            {
                Ok(Self {
                    pipe,
                    info: *info,
                    size,
                })
            }
            R::Error { error } => Err(Error::Source(Box::new(error))),
            _ => pipe.invalid("source prepare identity differs"),
        }
    }
    pub fn info(&self) -> &PptxPlaybackSessionInfo {
        &self.info
    }
    pub fn is_stopped(&self) -> bool {
        self.pipe.is_stopped()
    }
    pub fn sample(
        &mut self,
        at: RationalTime,
        history: Option<EventHistory>,
        check: &dyn Fn() -> bool,
    ) -> Result<Frame<PptxPlaybackRasterInfo>, Error> {
        let request = Q::Render {
            sample: PlaybackSampleRequest {
                binding: self.info.binding.clone(),
                at,
                history,
            },
        };
        let reply = self.pipe.call(&request, empty(), empty(), check)?;
        match reply.info {
            R::Rendered { info }
                if info.profile == mo_kernel_api::PPTX_PLAYBACK_PROFILE
                    && info.playback.evaluated.state.binding == self.info.binding
                    && info.playback.evaluated.state.time == at.normalized()
                    && info.playback.source_sha256 == self.info.source_sha256
                    && info.playback.slide == self.info.slide =>
            {
                self.pipe.validate_pixels(
                    &info.page.page.scene.raster,
                    &reply.pixels,
                    self.size,
                    check,
                )?;
                Ok(Frame {
                    info: *info,
                    pixels: reply.pixels,
                })
            }
            R::Error { error } if reply.pixels.is_empty() => Err(Error::Source(Box::new(error))),
            _ => self
                .pipe
                .invalid("source sample identity or response kind differs"),
        }
    }
    pub fn timing(&mut self, check: &dyn Fn() -> bool) -> Result<PlaybackTimingInfo, Error> {
        match self.pipe.control(
            &Q::InspectTiming {
                binding: self.info.binding.clone(),
            },
            check,
        )? {
            R::TimingInspected { info } if info.binding == self.info.binding => Ok(*info),
            R::Error { error } => Err(Error::Source(Box::new(error))),
            _ => self.pipe.invalid("source timing identity differs"),
        }
    }
    pub fn advance(
        &mut self,
        generation: PlaybackGeneration,
        check: &dyn Fn() -> bool,
    ) -> Result<&PptxPlaybackSessionInfo, Error> {
        let mut binding = self.info.binding.clone();
        binding.generation = generation;
        match self.pipe.control(
            &Q::Advance {
                binding: self.info.binding.clone(),
                generation,
            },
            check,
        )? {
            R::Advanced { info }
                if info.profile == self.info.profile
                    && info.binding == binding
                    && info.plan_id == self.info.plan_id
                    && info.slide == self.info.slide
                    && info.source_sha256 == self.info.source_sha256 =>
            {
                self.info = *info;
                Ok(&self.info)
            }
            R::Error { error } => Err(Error::Source(Box::new(error))),
            _ => self.pipe.invalid("source generation response differs"),
        }
    }
    pub fn dispose(mut self, check: &dyn Fn() -> bool) -> Result<(), Error> {
        let start = Instant::now();
        match self.pipe.control(
            &Q::Dispose {
                binding: self.info.binding.clone(),
            },
            check,
        )? {
            R::Disposed { binding } if binding == self.info.binding => {
                self.pipe.finish(start.elapsed(), check)
            }
            R::Error { error } => Err(Error::Source(Box::new(error))),
            _ => self.pipe.invalid("source disposal identity differs"),
        }
    }
}
