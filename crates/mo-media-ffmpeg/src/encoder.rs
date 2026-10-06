use crate::{
    EncodeConfig, EncodeLimits, EncodeProfile, EncodeStats, EncodeVideoFrame, MediaError, WriteAt,
    ffi::encode,
};

/// Compiled profiles only; hardware availability is determined by opening the
/// exact configuration. No implicit software/profile fallback occurs.
pub fn encoding_profiles() -> Result<Vec<EncodeProfile>, MediaError> {
    encode::profiles()
}
/// A single-threaded finite output candidate. Both successful finish and any
/// failure close native state; host-owned bytes still need verification and
/// atomic publication. Hard vendor memory/time limits belong to a worker.
/// ```compile_fail
/// fn send<T: Send>() {}
/// send::<mo_media_ffmpeg::Encoder<'static>>();
/// ```
pub struct Encoder<'a> {
    context: encode::EncodeContext<'a>,
}
impl<'a> Encoder<'a> {
    pub fn open(
        sink: impl WriteAt + 'a,
        config: EncodeConfig,
        limits: EncodeLimits,
        cancel: impl FnMut() -> bool + 'a,
    ) -> Result<Self, MediaError> {
        Ok(Self {
            context: encode::EncodeContext::open(sink, config, limits, cancel)?,
        })
    }
    pub fn video(&mut self, frame: EncodeVideoFrame<'_>) -> Result<(), MediaError> {
        self.context.video(frame)
    }
    pub fn audio(&mut self, first_sample: i64, interleaved: &[f32]) -> Result<(), MediaError> {
        self.context.audio(first_sample, interleaved)
    }
    pub fn finish(&mut self) -> Result<EncodeStats, MediaError> {
        self.context.finish()
    }
    pub fn is_closed(&self) -> bool {
        self.context.is_closed()
    }
}
