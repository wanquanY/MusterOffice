//! Presentation authoring semantics, independent of rendering and host storage.
mod dependencies;
mod document;
mod geometry;
mod source;
mod source_validation;
mod styles;
mod text;
mod timing;
mod validation;

pub use dependencies::PageDependencies;
pub use document::*;
pub use geometry::*;
pub use source::*;
pub use styles::*;
pub use text::*;
pub use validation::*;
