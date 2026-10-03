use super::*;
impl PlaybackSession {
    pub(super) fn resize(
        &mut self,
        binding: &PlaybackBinding,
        expected: u32,
        viewport: RasterViewport,
        check: &dyn Fn() -> bool,
    ) -> Result<PlaybackSessionInfo, PlaybackSessionFailure> {
        let r = self.ready(binding)?;
        if r.viewport_revision != expected {
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
        let id = mo_common::digest(
            PLAYBACK_SESSION_PROFILE,
            &(
                &binding.revision,
                r.plan.document_sha256(),
                &r.slide,
                &viewport,
                r.plan.defaults(),
                crate::PLAYBACK_PAGE_PROFILE,
            ),
        )
        .map_err(|e| fail(PlaybackSessionFailureCode::InputInvalid, e))?;
        r.plan.resize(viewport, check).map_err(computed)?;
        r.id = id;
        r.viewport_revision = next;
        let info = r.info();
        // Fence prepared completions even after A -> B -> A, without changing
        // the playback generation or invalidating its timing prefix cache.
        self.identity = Arc::new(());
        Ok(info)
    }
}
