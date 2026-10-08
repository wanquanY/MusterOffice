//! Finite immutable-project rendering into a typed candidate encoder. No files,
//! native codec linkage, business jobs, persistence or publication authority.
mod plan;
mod schedule;
mod session;
pub use plan::*;
pub use schedule::*;
pub use session::*;
pub const PROFILE: &str = "musteroffice.video.finite-cfr-export/1-draft";

mod inspection;
pub use inspection::{ExportInspection, InspectionError, inspect_browser_candidate};
