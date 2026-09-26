//! Explicit font resource/name/instance selection connected to text computation.
//! No system discovery, synthetic styles or silent family/slot substitution.
mod binding;
mod prepared;
mod selection;
#[cfg(test)]
mod tests;
mod types;
use crate::{cascade::*, paragraph::*, *};
pub use prepared::*;
pub use selection::*;
use std::collections::BTreeMap;
pub use types::*;

/// Convenience one-paragraph operation. Page compilers should prepare one
/// immutable manifest and reuse it for all their paragraph operations.
pub fn shape_paragraph(
    request: &ManifestParagraphRequest,
    bundle: &[u8],
    backend: &mut dyn backend::TextBackend,
    limits: ManifestLimits,
    check: &dyn Fn() -> bool,
) -> Result<ManifestParagraphResult, TextError> {
    PreparedManifest::load(&request.manifest, bundle, limits, check)?.shape_paragraph(
        request.into(),
        backend,
        check,
    )
}
