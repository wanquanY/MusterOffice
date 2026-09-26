//! Domain-independent evaluated paths and certified outward geometric bounds.
//! No font, renderer, host I/O or presentation semantics.
mod affine;
#[cfg(test)]
mod affine_tests;
mod bounds;
mod number;
#[cfg(test)]
mod tests;
mod types;
mod wide;
pub use affine::{Affine, PointEstimate};
pub use bounds::{BoundsBudget, path_bounds};
pub use number::Fixed;
use thiserror::Error;
pub use types::*;
#[derive(Debug, Error)]
pub enum GeometryError {
    #[error("geometry numeric range")]
    Numeric,
    #[error("invalid geometry: {0}")]
    Invalid(&'static str),
    #[error("geometry limit: {0}")]
    Limit(&'static str),
    #[error("geometry cancelled")]
    Cancelled,
}
