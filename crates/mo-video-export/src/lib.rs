//! Finite immutable-project rendering into a typed candidate encoder. No files,
//! native codec linkage, business jobs, persistence or publication authority.
mod plan;
mod session;
pub use plan::*;
pub use session::*;
pub const PROFILE: &str = "musteroffice.video.finite-cfr-export/1-draft";
