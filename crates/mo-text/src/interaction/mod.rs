//! Interaction geometry computed from the same line planner and exact pen as rendering.
pub(crate) mod build;
mod query;
#[cfg(test)]
mod tests;
mod types;
use crate::{TextError, backend::TextBackend, cancelled};
use mo_geometry::{Fixed, Point};
use mo_unicode::TextBoundary;
pub use types::*;

pub fn paragraph_interaction(
    q: &ParagraphInteractionRequest,
    bundle: &[u8],
    backend: &mut dyn TextBackend,
    check: &dyn Fn() -> bool,
) -> Result<ParagraphInteractionResult, TextError> {
    paragraph_using(
        &(&q.layout).into(),
        &q.queries,
        crate::resources::ResourceInput::Bundle(bundle),
        backend,
        check,
    )
}
pub(crate) fn paragraph_using(
    q: &crate::flow::FlowInput<'_>,
    queries: &[TextQuery],
    input: crate::resources::ResourceInput<'_, '_>,
    backend: &mut dyn TextBackend,
    check: &dyn Fn() -> bool,
) -> Result<ParagraphInteractionResult, TextError> {
    cancelled(check)?;
    if queries.len() > 64 {
        return Err(TextError::Limit("paragraph interaction queries"));
    }
    let segmentation = mo_unicode::segment(
        &q.paragraph.text,
        mo_unicode::UnicodeLimits::default(),
        check,
    )
    .map_err(crate::cascade::unicode_error)?;
    query::preflight(queries, &segmentation.boundaries, check)?;
    let mut map = None;
    let layout = crate::flow::layout_flow(
        q,
        input,
        backend,
        check,
        |g, shaping, fonts, bindings, backend, check| {
            let (result, precise) =
                crate::geometry::evaluate_interaction(g, shaping, fonts, bindings, backend, check)?;
            if result.layout.is_some() {
                map = Some(build::build(
                    build::Input {
                        q: g,
                        geometry: &result,
                        precise: &precise,
                    },
                    segmentation.boundaries,
                    fonts,
                    bindings,
                    backend,
                    check,
                )?);
            }
            Ok(result)
        },
    )?;
    let results = if let Some(map) = &map {
        map.query(queries, check)?
    } else {
        vec![]
    };
    cancelled(check)?;
    Ok(ParagraphInteractionResult {
        layout,
        map,
        results,
    })
}
