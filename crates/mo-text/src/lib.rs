//! Font-bound shaping, paragraph preparation and line-context reshaping.
//! Host execution and authority live outside this crate.
pub mod backend;
pub mod carets;
pub mod cascade;
pub mod fallback;
pub mod flow;
pub mod geometry;
pub mod interaction;
pub mod itemize;
pub mod lines;
pub mod manifest;
pub mod metrics;
pub mod outlines;
pub mod paragraph;
mod prepare;
mod resources;
mod result;
pub mod scene;
mod types;
use backend::TextBackend;
use mo_font::{FontError, FontLimits, VerifiedFont};
use thiserror::Error;
pub use types::*;

#[derive(Debug, Error)]
pub enum TextError {
    #[error("invalid text request: {0}")]
    Invalid(&'static str),
    #[error("text limit exceeded: {0}")]
    Limit(&'static str),
    #[error(transparent)]
    Font(#[from] FontError),
    #[error(transparent)]
    FontSelection(Box<manifest::FontSelectionFailure>),
    #[error("invalid shaping component output: {0}")]
    BackendInvalid(&'static str),
    #[error("shaping component failed with status {status} at run {run}")]
    BackendFailure { status: u32, run: u32 },
    #[error("shaping host failed: {0}")]
    Host(&'static str),
    #[error("text shaping cancelled")]
    Cancelled,
}
fn cancelled(check: &dyn Fn() -> bool) -> Result<(), TextError> {
    if check() {
        Err(TextError::Cancelled)
    } else {
        Ok(())
    }
}
pub fn shape(
    request: &ShapeRequest,
    font_bytes: &[u8],
    backend: &mut dyn TextBackend,
    limits: TextLimits,
    font_limits: FontLimits,
    check: &dyn Fn() -> bool,
) -> Result<ShapedText, TextError> {
    cancelled(check)?;
    prepare::check_size(request, limits)?;
    let font = VerifiedFont::load(
        &request.expected_sha256,
        request.face_index,
        font_bytes,
        font_limits,
        check,
    )?;
    shape_verified(request, &font, backend, limits, check)
}
/// Reuse a verified immutable face across shaping calls. Identity is still
/// checked; a host cannot substitute a deserialized metadata report for bytes.
pub fn shape_verified(
    request: &ShapeRequest,
    font: &VerifiedFont<'_>,
    backend: &mut dyn TextBackend,
    limits: TextLimits,
    check: &dyn Fn() -> bool,
) -> Result<ShapedText, TextError> {
    cancelled(check)?;
    prepare::check_size(request, limits)?;
    if request.expected_sha256 != font.metadata().sha256
        || request.face_index != font.metadata().face_index
    {
        return Err(FontError::ResourceConflict.into());
    }
    let prepared = prepare::encode(request, font.metadata(), limits, check)?;
    cancelled(check)?;
    let raw = match backend.shape_batch(font.bytes(), &prepared.frame) {
        Ok(raw) => raw,
        Err(error) => {
            backend.invalidate();
            return Err(error);
        }
    };
    if let Err(error) = cancelled(check) {
        backend.invalidate();
        return Err(error);
    }
    let result = result::decode(
        request,
        font.metadata(),
        prepared.variations,
        &raw,
        limits,
        check,
    );
    if matches!(
        result,
        Err(TextError::BackendInvalid(_)
            | TextError::BackendFailure { status: 2 | 6, .. }
            | TextError::Cancelled)
    ) {
        backend.invalidate();
    }
    result
}

#[cfg(test)]
mod tests;

impl From<mo_geometry::GeometryError> for TextError {
    fn from(error: mo_geometry::GeometryError) -> Self {
        match error {
            mo_geometry::GeometryError::Numeric => Self::Limit("line geometry numeric range"),
            mo_geometry::GeometryError::Invalid(reason) => Self::Invalid(reason),
            mo_geometry::GeometryError::Limit(reason) => Self::Limit(reason),
            mo_geometry::GeometryError::Cancelled => Self::Cancelled,
        }
    }
}
