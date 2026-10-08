//! Pure GPU command compilation. No GPU device, browser, codec or host resource
//! owner. The same compositor computes transforms, grouping and resource checks.
mod color;
mod raw_input;
pub use raw_input::*;
mod parameters;
mod prepared;
mod program;
pub use color::*;
pub use parameters::*;
pub use prepared::*;
pub use program::*;
pub const SHADER: &str = include_str!("compose.wgsl");
pub const PROFILE: &str = "musteroffice.video.gpu-plan/1-draft";
pub const MAX_ABS_COMPONENT: f32 = 1e12;
