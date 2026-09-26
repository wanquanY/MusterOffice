//! Presentation authoring semantics, independent of rendering and host storage.
mod document;
mod geometry;
mod styles;
mod text;
mod timing;
mod validation;

pub use document::*;
pub use geometry::*;
pub use styles::*;
pub use text::*;
pub use validation::*;
