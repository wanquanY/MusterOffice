//! Private frame ownership joins compilation and completion across a host yield.
use super::*;
use mo_presentation_compile::playback::{PlaybackImage, PreparedPlaybackFrame};
use mo_raster::BackendReply;

pub struct PreparedPlaybackRender {
    owner: Arc<()>,
    plan_id: Digest,
    frame: PreparedPlaybackFrame,
}
impl PreparedPlaybackRender {
    pub fn words(&self) -> &[u32] {
        self.frame.raster().frame()
    }
}
impl PlaybackSession {
    /// Bounded existing render request, with no component invocation.
    pub fn prepare_render_json(
        &mut self,
        input: &str,
        check: &dyn Fn() -> bool,
    ) -> Result<PreparedPlaybackRender, PlaybackSessionFailure> {
        if input.len() > crate::MAX_REQUEST_BYTES {
            return Err(fail(
                PlaybackSessionFailureCode::LimitExceeded,
                "playback frame request bytes",
            ));
        }
        match from_json_str(input).map_err(|e| fail(PlaybackSessionFailureCode::InputInvalid, e))? {
            PlaybackSessionRequest::Render { sample } => self.prepare_render(sample, check),
            _ => Err(fail(
                PlaybackSessionFailureCode::InputInvalid,
                "expected render request",
            )),
        }
    }
    pub fn prepare_render(
        &mut self,
        sample: PlaybackSampleRequest,
        check: &dyn Fn() -> bool,
    ) -> Result<PreparedPlaybackRender, PlaybackSessionFailure> {
        cancel(check)?;
        let owner = Arc::clone(&self.identity);
        let ready = self.ready(&sample.binding)?;
        let frame = ready
            .plan
            .prepare_frame(sample.at, sample.history.as_ref(), check)
            .map_err(computed)?;
        Ok(PreparedPlaybackRender {
            owner,
            plan_id: ready.id.clone(),
            frame,
        })
    }
    /// Consumes this frame exactly once. Owner identity is private and independent
    /// of host-supplied session strings; generation/disposal are rechecked here.
    /// Component errors have the same diagnostics as synchronous render. The host
    /// still owns component quarantine and result publication.
    pub fn complete_render(
        &mut self,
        prepared: PreparedPlaybackRender,
        reply: BackendReply,
        check: &dyn Fn() -> bool,
    ) -> Result<PlaybackImage, PlaybackCompletionFailure<PlaybackSessionFailure>> {
        self.complete_render_result(prepared, Ok(reply), check)
    }
    pub fn complete_render_result(
        &mut self,
        prepared: PreparedPlaybackRender,
        reply: Result<BackendReply, mo_raster::RasterError>,
        check: &dyn Fn() -> bool,
    ) -> Result<PlaybackImage, PlaybackCompletionFailure<PlaybackSessionFailure>> {
        self.complete_render_reply(prepared, reply.map(Into::into), check)
    }
    /// Accepts a sealed pixel validation proof; owner/plan/generation checks still run.
    pub fn complete_render_reply(
        &mut self,
        prepared: PreparedPlaybackRender,
        reply: Result<mo_raster::RasterCompletionReply, mo_raster::RasterError>,
        check: &dyn Fn() -> bool,
    ) -> Result<PlaybackImage, PlaybackCompletionFailure<PlaybackSessionFailure>> {
        let reject = |error| PlaybackCompletionFailure {
            error,
            invalidate_backend: match &reply {
                Ok(reply) => reply
                    .check_status()
                    .is_err_and(|error| error.invalidates_backend()),
                Err(error) => error.invalidates_backend(),
            },
        };
        cancel(check).map_err(reject)?;
        if !Arc::ptr_eq(&self.identity, &prepared.owner) {
            return Err(reject(fail(
                PlaybackSessionFailureCode::BindingConflict,
                "frame belongs to another owner or viewport revision",
            )));
        }
        let ready = self.ready(prepared.frame.binding()).map_err(reject)?;
        if ready.id != prepared.plan_id {
            return Err(reject(fail(
                PlaybackSessionFailureCode::BindingConflict,
                "frame plan mismatch",
            )));
        }
        let reply = reply.map_err(|error| {
            let invalidate_backend = error.invalidates_backend();
            PlaybackCompletionFailure {
                error: computed(mo_presentation_compile::PageError::from(error).into()),
                invalidate_backend,
            }
        })?;
        prepared.frame.complete(reply, check).map_err(|e| {
            let invalidate_backend = matches!(&e, mo_presentation_compile::playback::PlaybackError::Page(mo_presentation_compile::PageError::Raster(r)) if r.invalidates_backend());
            PlaybackCompletionFailure { error: computed(e), invalidate_backend }
        })
    }
}
