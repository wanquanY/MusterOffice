//! Native DrawingML geometry declarations. Formula execution and derived paths
//! are separate: reading retains author choices, omissions and source identity.
pub mod evaluate;
mod names;
mod read;
mod types;
pub(super) use read::{Budget, Reader};
pub use types::*;
