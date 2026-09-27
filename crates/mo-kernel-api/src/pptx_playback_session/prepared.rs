use super::*;
use mo_presentation_compile::source_resource_page::PreparedResourceFrame;
use mo_raster::BackendReply;

/// Retains decoded resources without cloning their bytes or borrowing the owner.
pub struct PreparedPptxPlaybackRender {
    owner: Arc<()>,
    plan_id: Digest,
    playback: SourcePlaybackFrame,
    page: PreparedResourceFrame,
}
impl PreparedPptxPlaybackRender {
    pub fn words(&self) -> &[u32] {
        self.page.raster().frame()
    }
    pub fn images(&self) -> &[u8] {
        self.page.raster().images()
    }
}
impl PptxPlaybackSession {
    /// Bounded existing render request, with no component invocation.
    pub fn prepare_render_json(
        &mut self,
        input: &str,
        check: &dyn Fn() -> bool,
    ) -> Result<PreparedPptxPlaybackRender, PptxPlaybackSessionFailure> {
        if input.len() > crate::MAX_REQUEST_BYTES {
            return Err(fail(
                PlaybackSessionFailureCode::LimitExceeded,
                "playback frame request bytes",
            ));
        }
        match from_json_str(input).map_err(|e| fail(PlaybackSessionFailureCode::InputInvalid, e))? {
            PptxPlaybackSessionRequest::Render { sample } => self.prepare_render(sample, check),
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
    ) -> Result<PreparedPptxPlaybackRender, PptxPlaybackSessionFailure> {
        cancel(check)?;
        let owner = Arc::clone(&self.identity);
        let ready = self.state.ready(&sample.binding)?;
        let (playback, page) = ready
            .plan
            .prepare_frame(sample.at, sample.history.as_ref(), check)
            .map_err(|e| computed(crate::pptx_playback::failure(e)))?;
        Ok(PreparedPptxPlaybackRender {
            owner,
            plan_id: ready.info.plan_id.clone(),
            playback,
            page,
        })
    }
    pub fn complete_render(
        &mut self,
        prepared: PreparedPptxPlaybackRender,
        reply: BackendReply,
        check: &dyn Fn() -> bool,
    ) -> Result<
        (PptxPlaybackRasterInfo, Vec<u8>),
        PlaybackCompletionFailure<PptxPlaybackSessionFailure>,
    > {
        self.complete_render_result(prepared, Ok(reply), check)
    }
    pub fn complete_render_result(
        &mut self,
        prepared: PreparedPptxPlaybackRender,
        reply: Result<BackendReply, mo_raster::RasterError>,
        check: &dyn Fn() -> bool,
    ) -> Result<
        (PptxPlaybackRasterInfo, Vec<u8>),
        PlaybackCompletionFailure<PptxPlaybackSessionFailure>,
    > {
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
                "frame belongs to another playback owner",
            )));
        }
        let ready = self
            .state
            .ready(&prepared.playback.evaluated.state.binding)
            .map_err(PptxPlaybackSessionFailure::from)
            .map_err(reject)?;
        if ready.info.plan_id != prepared.plan_id {
            return Err(reject(fail(
                PlaybackSessionFailureCode::BindingConflict,
                "frame plan mismatch",
            )));
        }
        let reply = reply.map_err(|error| {
            let invalidate_backend = error.invalidates_backend();
            PlaybackCompletionFailure {
                error: computed(crate::pptx_playback::failure(
                    mo_presentation_compile::source_page::SourcePageError::from(error).into(),
                )),
                invalidate_backend,
            }
        })?;
        let page = prepared
            .page
            .complete(reply, check)
            .map_err(|e| {
                let invalidate_backend = matches!(&e, mo_presentation_compile::source_page::SourcePageError::Raster(r) if r.invalidates_backend());
                PlaybackCompletionFailure { error: computed(crate::pptx_playback::failure(e.into())), invalidate_backend }
            })?;
        Ok((
            PptxPlaybackRasterInfo {
                profile: PPTX_PLAYBACK_PROFILE.into(),
                playback: prepared.playback,
                page: page.info,
            },
            page.pixels,
        ))
    }
}
