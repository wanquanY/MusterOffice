//! Canonical decoded PCM tiles and bounded random reads. Storage, decoder
//! ownership, permission checks and atomic publication belong to the host.
mod builder;
mod reader;
mod resource;
mod retime;
pub use resource::*;
pub use retime::*;
mod source;
pub use builder::*;
use mo_common::{ByteLength, Digest, Ticks};
pub use reader::*;
use serde::{Deserialize, Serialize};
pub use source::*;

pub const PROFILE: &str = "musteroffice.pcm.f32le-tiles/1-draft";
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PcmTileRef {
    pub source: Digest,
    pub index: u32,
    pub first_sample: Ticks,
    pub frames: u32,
    pub sha256: Digest,
    pub byte_length: ByteLength,
}
/// Payload is raw interleaved f32 little-endian, without JSON or decoder padding.
#[derive(Debug)]
pub struct PcmTile {
    pub reference: PcmTileRef,
    pub bytes: Vec<u8>,
}
#[derive(Debug, thiserror::Error)]
pub enum PcmError {
    #[error("invalid canonical PCM: {0}")]
    Invalid(&'static str),
    #[error("canonical PCM budget exceeded")]
    Budget,
    #[error("canonical PCM cancelled")]
    Cancelled,
    #[error("PCM tile {index} is not materialized")]
    Missing { index: u32 },
    #[error("PCM store: {0}")]
    Store(String),
    #[error(transparent)]
    Audio(#[from] mo_video_audio::AudioError),
    #[error(transparent)]
    Time(#[from] mo_time::TimeError),
    #[error(transparent)]
    Canonical(#[from] mo_common::CanonicalError),
}
pub type Result<T> = std::result::Result<T, PcmError>;
pub(crate) fn check(cancelled: &dyn Fn() -> bool) -> Result<()> {
    if cancelled() {
        Err(PcmError::Cancelled)
    } else {
        Ok(())
    }
}
pub(crate) fn bytes(count: usize) -> Result<Vec<u8>> {
    let mut v = Vec::new();
    v.try_reserve_exact(count).map_err(|_| PcmError::Budget)?;
    Ok(v)
}
