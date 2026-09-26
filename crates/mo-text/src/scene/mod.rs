//! Evaluated paragraph glyph paths; shares exact line placement and font context.
mod build;
#[cfg(test)]
pub(crate) mod test_support;
#[cfg(test)]
mod tests;
mod types;
use crate::{backend::TextBackend, geometry::evaluate_precise, *};
pub use types::*;
pub fn paragraph_paths(
    q: &ParagraphPathsRequest,
    bundle: &[u8],
    backend: &mut dyn TextBackend,
    check: &dyn Fn() -> bool,
) -> Result<ParagraphPathsResult, TextError> {
    paragraph_paths_using(q, resources::ResourceInput::Bundle(bundle), backend, check)
}
pub(crate) fn paragraph_paths_using(
    q: &ParagraphPathsRequest,
    input: resources::ResourceInput<'_, '_>,
    backend: &mut dyn TextBackend,
    check: &dyn Fn() -> bool,
) -> Result<ParagraphPathsResult, TextError> {
    Ok(paragraph_geometry_using(
        &(&q.layout).into(),
        q.bounds_tolerance,
        input,
        backend,
        check,
    )?
    .paths)
}
pub(crate) fn paragraph_geometry_using(
    q: &flow::FlowInput<'_>,
    bounds_tolerance: mo_geometry::Fixed,
    input: resources::ResourceInput<'_, '_>,
    backend: &mut dyn TextBackend,
    check: &dyn Fn() -> bool,
) -> Result<ParagraphGeometryPaths, TextError> {
    cancelled(check)?;
    if !(256..=1i128 << 32).contains(&bounds_tolerance.raw()) {
        return Err(TextError::Invalid("path bounds tolerance"));
    }
    let mut scene = None;
    let mut issues = Vec::new();
    let mut precise_layout = None;
    let layout = flow::layout_flow(
        q,
        input,
        backend,
        check,
        |g, shaping, fonts, bindings, backend, check| {
            let (result, precise) = evaluate_precise(g, shaping, fonts, bindings, backend, check)?;
            if result.layout.is_some() {
                let built = build::build(
                    build::PathInput {
                        styles: q.styles,
                        bounds_tolerance,
                    },
                    &result,
                    precise.glyphs,
                    fonts,
                    bindings,
                    backend,
                    check,
                )?;
                precise_layout = precise.layout;
                scene = built.0;
                issues = built.1;
            }
            Ok(result)
        },
    )?;
    cancelled(check)?;
    Ok(ParagraphGeometryPaths {
        precise: precise_layout,
        paths: ParagraphPathsResult {
            layout,
            scene,
            issues,
        },
    })
}
