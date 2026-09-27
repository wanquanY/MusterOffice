use super::{
    pipe::{Pipe, empty},
    *,
};
use mo_common::RationalTime;
use mo_kernel_api::{
    PlaybackSampleRequest, PlaybackSessionRequest as Q, PlaybackSessionResponse as R,
};
use std::time::Instant;

/// One immutable author page/revision and one bounded core timing sampler.
/// This owner's mutable methods serialize calls; the host schedules/display them.
pub struct AuthorPlayback {
    pipe: Pipe<R>,
    info: PlaybackSessionInfo,
    size: (u32, u32),
}
impl AuthorPlayback {
    pub(super) fn prepare(
        config: &NativePlayback,
        request: PlaybackPrepareRequest,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, Error> {
        let mut pipe = Pipe::start(config, "--playback-session", false, check)?;
        let binding = request.binding.clone();
        let slide = request.slide.clone();
        let document_sha256 = request.snapshot.semantic_digest.clone();
        let size = (request.viewport.width, request.viewport.height);
        let response = pipe.control(
            &Q::Prepare {
                request: Box::new(request),
            },
            check,
        )?;
        match response {
            R::Prepared { info }
                if info.profile == mo_kernel_api::PLAYBACK_SESSION_PROFILE
                    && info.binding == binding
                    && info.document_sha256 == document_sha256
                    && info.slide == slide =>
            {
                Ok(Self { pipe, info, size })
            }
            R::Error { error } => Err(Error::Author(Box::new(error))),
            _ => pipe.invalid("author prepare identity differs"),
        }
    }
    pub fn info(&self) -> &PlaybackSessionInfo {
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
    ) -> Result<Frame<PlaybackRasterInfo>, Error> {
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
                if info.profile == mo_kernel_api::PLAYBACK_PAGE_PROFILE
                    && info.frame.state.binding == self.info.binding
                    && info.frame.state.time == at.normalized() =>
            {
                self.pipe.validate_pixels(
                    &info.page.scene.raster,
                    &reply.pixels,
                    self.size,
                    check,
                )?;
                Ok(Frame {
                    info: *info,
                    pixels: reply.pixels,
                })
            }
            R::Error { error } if reply.pixels.is_empty() => Err(Error::Author(Box::new(error))),
            _ => self
                .pipe
                .invalid("author sample identity or response kind differs"),
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
            R::Error { error } => Err(Error::Author(Box::new(error))),
            _ => self.pipe.invalid("author timing identity differs"),
        }
    }
    /// Generation changes invalidate old event histories without rebuilding the
    /// immutable page. The core rejects non-increasing generations atomically.
    pub fn advance(
        &mut self,
        generation: PlaybackGeneration,
        check: &dyn Fn() -> bool,
    ) -> Result<&PlaybackSessionInfo, Error> {
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
                    && info.document_sha256 == self.info.document_sha256 =>
            {
                self.info = info;
                Ok(&self.info)
            }
            R::Error { error } => Err(Error::Author(Box::new(error))),
            _ => self.pipe.invalid("author generation response differs"),
        }
    }
    /// Explicit verified disposal. Drop also kills/reaps an undisposed worker.
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
            R::Error { error } => Err(Error::Author(Box::new(error))),
            _ => self.pipe.invalid("author disposal identity differs"),
        }
    }
}
