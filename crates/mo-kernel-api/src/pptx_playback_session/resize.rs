use super::*;
impl PptxPlaybackSession {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn resize(
        &mut self,
        binding: &PlaybackBinding,
        expected: u32,
        viewport: mo_raster::RasterViewport,
        source: &[u8],
        decoder: &mut dyn mo_image::ImageDecoder,
        check: &dyn Fn() -> bool,
    ) -> Result<Box<PptxPlaybackSessionInfo>, PptxPlaybackSessionFailure> {
        use mo_presentation_compile::source_page::SourcePageError;
        let r = self.state.ready(binding)?;
        if r.info.viewport_revision != expected {
            return Err(fail(
                PlaybackSessionFailureCode::ViewportConflict,
                "viewport revision changed",
            ));
        }
        let next = expected.checked_add(1).ok_or_else(|| {
            fail(
                PlaybackSessionFailureCode::LimitExceeded,
                "viewport revision exhausted",
            )
        })?;
        let page_error = |e: SourcePageError| computed(crate::pptx_playback::failure(e.into()));
        viewport.validate().map_err(|e| page_error(e.into()))?;
        // Package::open bounds and hashes the original bytes. Reuse the retained
        // source index and text paths; never re-import XML or re-shape fonts here.
        let package = mo_opc::Package::open(
            source,
            source.len() as u64,
            crate::pptx_source::inline_limits().package,
            check,
        )
        .map_err(|e| page_error(mo_pptx::PptxError::from(e).into()))?;
        let mut request = r.request.clone();
        request.page.viewport = viewport.clone();
        let candidate = r
            .plan
            .prepare_resize(&package, viewport.clone(), decoder, check)
            .map_err(|e| computed(crate::pptx_playback::failure(e)))?;
        let mut info = r.info.clone();
        info.binding = binding.clone();
        info.viewport = viewport;
        info.viewport_revision = next;
        info.preparation = candidate.preparation().clone();
        info.plan_id = mo_common::digest(
            PPTX_PLAYBACK_SESSION_PROFILE,
            &(
                &request,
                &binding.revision,
                &info.preparation,
                PPTX_PLAYBACK_PROFILE,
            ),
        )
        .map_err(|e| fail(PlaybackSessionFailureCode::InputInvalid, e))?;
        cancel(check)?;
        candidate.commit();
        r.request = request;
        r.info = info;
        let info = r.info();
        self.identity = Arc::new(());
        Ok(info)
    }
}
