//! Typed, caller-owned access to retained native page sampling. No UI or clock.
mod author;
mod pipe;
mod source;
pub use author::AuthorPlayback;
use mo_common::Digest;
pub use mo_kernel_api::{
    PlaybackPrepareRequest, PlaybackRasterInfo, PlaybackSessionFailure, PlaybackSessionInfo,
    PlaybackTimingInfo, PptxPlaybackPrepareRequest, PptxPlaybackRasterInfo,
    PptxPlaybackSessionFailure, PptxPlaybackSessionInfo, PptxResourcePageRequest,
};
pub use mo_native_worker::SessionError;
pub use mo_presentation_compile::PagePaintDefaults;
pub use mo_raster::{PixelScale, RasterViewport};
pub use mo_timeline::{EventHistory, PlaybackBinding, PlaybackGeneration};
pub use source::SourcePlayback;
use std::{path::PathBuf, process::Command, time::Duration};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("native playback configuration/input: {0}")]
    Input(String),
    #[error(transparent)]
    Transport(#[from] SessionError),
    #[error("invalid native playback response: {0}")]
    Response(&'static str),
    #[error("author playback computation: {0:?}")]
    Author(Box<PlaybackSessionFailure>),
    #[error("source playback computation: {0:?}")]
    Source(Box<PptxPlaybackSessionFailure>),
}

/// Owned premultiplied sRGB RGBA8 bytes plus the unchanged core diagnostics.
/// The SDK checks dimensions, length, digest, alpha and sample identity before
/// returning it. It does not store, display or retain previously returned frames.
pub struct Frame<T> {
    pub info: T,
    pub pixels: Vec<u8>,
}

/// Trusted host configuration; paths never enter document or playback requests.
#[derive(Clone)]
pub struct NativePlayback {
    executable: PathBuf,
    sha256: Digest,
    timeout: Duration,
}
impl NativePlayback {
    pub fn new(executable: PathBuf, sha256: Digest, timeout: Duration) -> Result<Self, Error> {
        if !executable.is_absolute() || timeout.is_zero() || timeout > Duration::from_secs(3600) {
            return Err(Error::Input(
                "absolute worker path and bounded positive timeout required".into(),
            ));
        }
        let value = Self {
            executable,
            sha256,
            timeout,
        };
        value.verify()?;
        Ok(value)
    }
    pub fn worker_sha256(&self) -> &Digest {
        &self.sha256
    }
    fn verify(&self) -> Result<(), Error> {
        if super::executable_digest(&self.executable).map_err(|e| Error::Input(e.to_string()))?
            != self.sha256
        {
            return Err(Error::Input(
                "playback worker executable digest differs".into(),
            ));
        }
        Ok(())
    }
    fn command(&self, mode: &str, check: &dyn Fn() -> bool) -> Result<Command, Error> {
        if check() {
            return Err(SessionError::Cancelled.into());
        }
        self.verify()?;
        if check() {
            return Err(SessionError::Cancelled.into());
        }
        let mut command = Command::new(&self.executable);
        command.arg(mode);
        Ok(command)
    }
    pub fn prepare_author(
        &self,
        request: PlaybackPrepareRequest,
        check: &dyn Fn() -> bool,
    ) -> Result<AuthorPlayback, Error> {
        AuthorPlayback::prepare(self, request, check)
    }
    /// Source/font buffers are borrowed only for this call and may be dropped
    /// immediately after it returns. Sampling reuses the worker's prepared plan.
    pub fn prepare_source(
        &self,
        request: PptxPlaybackPrepareRequest,
        source: mo_presentation_delivery::Content<'_>,
        fonts: mo_presentation_delivery::Content<'_>,
        check: &dyn Fn() -> bool,
    ) -> Result<SourcePlayback, Error> {
        SourcePlayback::prepare(self, request, source, fonts, check)
    }
}
