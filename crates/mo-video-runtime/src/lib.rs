//! Video computation and explicit media capabilities for an embedding host.
//!
//! This public entry keeps every value in one versioned Rust type graph. It does
//! not introduce another document, job, renderer, storage or authority system.
//! Authoring-only consumers should depend on `mo-video-operations` instead.
//!
//! The default feature set has no native worker or GPU dependency. `native`
//! exposes the separately packaged, externally pinned codec worker through
//! bounded ReadAt/WriteAt capabilities; codecs are never linked into this SDK.
//! `gpu` exposes the existing explicit-device compositor. A feature describes
//! compiled adapters, not a promise that a user's device supports every codec.
pub mod audio_movie;
pub mod visual_source;
pub use mo_common as common;
pub use mo_media as media;
pub use mo_media_isobmff as isobmff;
pub use mo_time as time;
pub use mo_video_audio as audio;
pub use mo_video_compile as compile;
pub use mo_video_export as export;
pub use mo_video_operations as authoring;
pub use mo_video_pcm as pcm;
pub use mo_video_render as render;
pub use mo_video_source as source;

#[cfg(feature = "native")]
pub use mo_native_limits as process_limits;
#[cfg(feature = "native")]
pub use mo_native_media as native;
#[cfg(feature = "gpu")]
pub use mo_video_gpu as gpu;

pub const PROFILE: &str = "musteroffice.video.runtime/1-draft";
