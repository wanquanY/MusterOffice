//! Sample-clocked PCM processing. No audio device, codec, path, network or host storage.
mod channels;
mod clock;
mod envelope;
mod mixer;
mod pcm;
mod resource;
pub use mixer::*;
pub use resource::*;
mod resample;
pub use channels::*;
pub use clock::PhaseClock;
pub use pcm::*;
pub use resample::*;

pub const PROFILE: &str = "musteroffice.audio.f32-sinc64-bh4-phase1024-v1-draft";
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("audio source {resource} is not ready for [{first_sample}, {end_sample})")]
    PendingSource {
        resource: mo_common::ResourceId,
        first_sample: i64,
        end_sample: i64,
    },
    #[error("invalid audio input: {0}")]
    Invalid(&'static str),
    #[error("unsupported audio capability: {0}")]
    Unsupported(&'static str),
    #[error("audio budget exceeded")]
    Budget,
    #[error("audio cancelled")]
    Cancelled,
    #[error("audio resource: {0}")]
    Resource(String),
    #[error(transparent)]
    Time(#[from] mo_time::TimeError),
    #[error(transparent)]
    Compile(#[from] mo_video_compile::CompileError),
}
pub type Result<T> = std::result::Result<T, AudioError>;
pub(crate) fn cancel(check: &dyn Fn() -> bool) -> Result<()> {
    if check() {
        Err(AudioError::Cancelled)
    } else {
        Ok(())
    }
}
