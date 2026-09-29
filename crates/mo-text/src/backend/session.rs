//! Calculation-scoped font residency. Only bytes borrowed from a verified
//! manifest may receive reusable handles; no pointer outlives its immutable font.
use super::*;
use mo_font::VerifiedFont;

pub struct FontSession<'a, 'font> {
    inner: &'a mut dyn TextBackend,
    fonts: &'a [VerifiedFont<'font>],
    handles: Vec<Option<u32>>,
}
impl<'a, 'font> FontSession<'a, 'font> {
    pub(crate) fn new(inner: &'a mut dyn TextBackend, fonts: &'a [VerifiedFont<'font>]) -> Self {
        Self {
            inner,
            fonts,
            handles: vec![None; fonts.len()],
        }
    }
    fn index(&self, bytes: &[u8]) -> Option<usize> {
        self.fonts.iter().position(|font| {
            let known = font.bytes();
            known.len() == bytes.len() && known.as_ptr() == bytes.as_ptr()
        })
    }
    fn handle(&mut self, font: &[u8]) -> Result<Option<u32>, TextError> {
        if !self.inner.supports_font_residency() {
            return Ok(None);
        }
        let Some(index) = self.index(font) else {
            return Ok(None);
        };
        if let Some(handle) = self.handles[index] {
            return Ok(Some(handle));
        }
        let handle = self.inner.register_font(font)?;
        if handle == 0 || self.handles.contains(&Some(handle)) {
            self.inner.invalidate();
            return Err(TextError::BackendInvalid("font registration handle"));
        }
        self.handles[index] = Some(handle);
        Ok(Some(handle))
    }
}
impl TextBackend for FontSession<'_, '_> {
    fn font_upload_bytes(&self, font: &[u8]) -> u64 {
        if self.index(font).is_some_and(|i| self.handles[i].is_some()) {
            0
        } else {
            font.len() as u64
        }
    }
    fn shape_batch(&mut self, font: &[u8], frame: &[u32]) -> Result<Vec<u32>, TextError> {
        match self.handle(font)? {
            Some(id) => self.inner.shape_registered(id, frame),
            None => self.inner.shape_batch(font, frame),
        }
    }
    fn measure_batch(&mut self, font: &[u8], frame: &[u32]) -> Result<Vec<u32>, TextError> {
        match self.handle(font)? {
            Some(id) => self.inner.measure_registered(id, frame),
            None => self.inner.measure_batch(font, frame),
        }
    }
    fn outline_batch(&mut self, font: &[u8], frame: &[u32]) -> Result<Vec<u32>, TextError> {
        match self.handle(font)? {
            Some(id) => self.inner.outline_registered(id, frame),
            None => self.inner.outline_batch(font, frame),
        }
    }
    fn invalidate(&mut self) {
        self.inner.invalidate();
    }
}
impl Drop for FontSession<'_, '_> {
    fn drop(&mut self) {
        for handle in self.handles.iter().flatten() {
            if self.inner.unregister_font(*handle).is_err() {
                self.inner.invalidate();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mo_common::Digest;
    use mo_font::FontLimits;
    use sha2::{Digest as _, Sha256};
    const FONT: &[u8] = include_bytes!("../../../../fixtures/fonts/owned.ttf");
    #[derive(Default)]
    struct Host {
        registrations: usize,
        releases: usize,
        raw: usize,
        resident: usize,
        invalid: bool,
        bad_handle: bool,
        release_failure: bool,
    }
    impl TextBackend for Host {
        fn supports_font_residency(&self) -> bool {
            true
        }
        fn register_font(&mut self, _: &[u8]) -> Result<u32, TextError> {
            self.registrations += 1;
            Ok(if self.bad_handle {
                0
            } else {
                self.registrations as u32
            })
        }
        fn unregister_font(&mut self, _: u32) -> Result<(), TextError> {
            self.releases += 1;
            if self.release_failure {
                Err(TextError::Host("release failure"))
            } else {
                Ok(())
            }
        }
        fn shape_registered(&mut self, _: u32, _: &[u32]) -> Result<Vec<u32>, TextError> {
            self.resident += 1;
            Ok(vec![])
        }
        fn measure_registered(&mut self, h: u32, w: &[u32]) -> Result<Vec<u32>, TextError> {
            self.shape_registered(h, w)
        }
        fn outline_registered(&mut self, h: u32, w: &[u32]) -> Result<Vec<u32>, TextError> {
            self.shape_registered(h, w)
        }
        fn shape_batch(&mut self, _: &[u8], _: &[u32]) -> Result<Vec<u32>, TextError> {
            self.raw += 1;
            Ok(vec![])
        }
        fn invalidate(&mut self) {
            self.invalid = true;
        }
    }
    fn fonts() -> Vec<VerifiedFont<'static>> {
        vec![
            VerifiedFont::load(
                &Digest::from_sha256(Sha256::digest(FONT).into()),
                0,
                FONT,
                FontLimits::default(),
                &|| false,
            )
            .unwrap(),
        ]
    }
    #[test]
    fn residency_is_scoped_and_only_reuses_borrowed_verified_bytes() {
        let fonts = fonts();
        let mut host = Host::default();
        {
            let mut session = FontSession::new(&mut host, &fonts);
            assert_eq!(session.font_upload_bytes(FONT), FONT.len() as u64);
            session.shape_batch(FONT, &[]).unwrap();
            assert_eq!(session.font_upload_bytes(FONT), 0);
            session.measure_batch(FONT, &[]).unwrap();
            session.outline_batch(FONT, &[]).unwrap();
            let foreign = FONT.to_vec();
            assert_eq!(session.font_upload_bytes(&foreign), FONT.len() as u64);
            session.shape_batch(&foreign, &[]).unwrap();
        }
        assert_eq!(
            (host.registrations, host.releases, host.resident, host.raw),
            (1, 1, 3, 1)
        );
        assert!(!host.invalid);
        {
            let mut next = FontSession::new(&mut host, &fonts);
            assert_eq!(next.font_upload_bytes(FONT), FONT.len() as u64);
            next.shape_batch(FONT, &[]).unwrap();
        }
        assert_eq!((host.registrations, host.releases), (2, 2));
    }
    #[test]
    fn registration_and_cleanup_failure_invalidate_the_component() {
        let fonts = fonts();
        let mut host = Host {
            bad_handle: true,
            ..Default::default()
        };
        {
            let mut session = FontSession::new(&mut host, &fonts);
            assert!(session.shape_batch(FONT, &[]).is_err());
        }
        assert!(host.invalid);
        let mut host = Host {
            release_failure: true,
            ..Default::default()
        };
        {
            let mut session = FontSession::new(&mut host, &fonts);
            session.shape_batch(FONT, &[]).unwrap();
        }
        assert_eq!(host.releases, 1);
        assert!(host.invalid);
    }
}
