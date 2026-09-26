//! Typed edits to direct native transform declarations. XML positions stay private.
mod binding;
mod types;
use super::*;
use crate::{PptxError, cancelled};
use mo_opc::{PartName, RewritePlan};
use mo_xml::AttributeRewriteLimits;
use std::collections::BTreeSet;
pub use types::{SourceTransformEdit, SourceTransformEdits, SourceTransformValues};

pub(super) struct Binding {
    pub ordinal: u32,
    pub in_alternate: bool,
}
fn conflict(message: &str) -> PptxError {
    PptxError::SourceConflict(message.into())
}
fn unsupported(message: &str) -> PptxError {
    PptxError::Unsupported(message.into())
}

fn validate(value: &SourceTransformValues) -> Result<(), PptxError> {
    // ECMA-376 dml-main ST_CoordinateUnqualified / ST_PositiveCoordinate.
    for p in [value.origin, value.child_origin].into_iter().flatten() {
        if [p.x, p.y]
            .iter()
            .any(|v| !(-27_273_042_329_600..=27_273_042_316_900).contains(&v.get()))
        {
            return Err(conflict("transform coordinate outside native range"));
        }
    }
    for s in [value.size, value.child_size].into_iter().flatten() {
        if [s.width, s.height]
            .iter()
            .any(|v| !(0..=27_273_042_316_900).contains(&v.get()))
        {
            return Err(conflict("transform extent outside native range"));
        }
    }
    Ok(())
}

/// Computes one atomic candidate without changing source bytes or publishing it.
/// Existing xfrm declarations and coordinate leaves are edited in place. Missing
/// declarations/leaves require structural insertion, a separate unfinished path.
/// Null rotation/flips remove explicit attributes; coordinate leaf presence must
/// be preserved. Root shape-tree/group child-coordinate edits retain their scope.
pub fn edit_source_transforms<R: ReaderAt>(
    source: &Package<R>,
    request: &SourceTransformEdits,
    limits: SourceLimits,
    check: &dyn Fn() -> bool,
) -> Result<Vec<u8>, PptxError> {
    cancelled(check)?;
    if source.sha256() != &request.expected_source_sha256 {
        return Err(conflict("source package digest changed"));
    }
    if request.edits.len() > limits.max_edits {
        return Err(PptxError::Limit("source transform edits"));
    }
    let mut target_bytes = 0usize;
    for edit in &request.edits {
        cancelled(check)?;
        target_bytes = target_bytes
            .checked_add(edit.target.part.len())
            .ok_or(PptxError::Limit("source transform target bytes"))?;
        if target_bytes > limits.max_text_bytes {
            return Err(PptxError::Limit("source transform target bytes"));
        }
    }
    let mut bound = presentation::read(source, limits, check)?;
    let mut selected = BTreeSet::new();
    let mut parts: BTreeMap<PartName, BTreeMap<u32, &SourceTransformEdit>> = BTreeMap::new();
    for edit in &request.edits {
        cancelled(check)?;
        let key = (edit.target.part.clone(), edit.target.native_id);
        if !selected.insert(key.clone()) {
            return Err(conflict("duplicate source transform target"));
        }
        let surface = bound
            .index
            .surfaces
            .get_mut(&edit.target.part)
            .ok_or_else(|| conflict("source surface missing"))?;
        let transform = if surface.root_object_id == edit.target.native_id {
            surface.root_group_transform.as_mut()
        } else {
            surface
                .objects
                .iter_mut()
                .find(|o| o.native_id == edit.target.native_id)
                .ok_or_else(|| conflict("source object missing"))?
                .transform
                .as_mut()
        }
        .ok_or_else(|| unsupported("source target has no direct transform declaration"))?;
        let binding = bound
            .transforms
            .get(&edit.target.part)
            .and_then(|objects| objects.get(&edit.target.native_id))
            .ok_or_else(|| conflict("source transform binding missing"))?;
        if SourceTransformValues::from(&*transform) != edit.expected {
            return Err(conflict("source transform precondition changed"));
        }
        if edit.expected == edit.replacement {
            continue;
        }
        if binding.in_alternate {
            return Err(unsupported(
                "transform in compatibility branch requires coordinated editing",
            ));
        }
        if !transform.retained_ordinals.is_empty() {
            return Err(unsupported(
                "transform contains retained semantics requiring coordinated editing",
            ));
        }
        validate(&edit.replacement)?;
        for (old, new) in [
            (
                edit.expected.origin.is_some(),
                edit.replacement.origin.is_some(),
            ),
            (
                edit.expected.size.is_some(),
                edit.replacement.size.is_some(),
            ),
            (
                edit.expected.child_origin.is_some(),
                edit.replacement.child_origin.is_some(),
            ),
            (
                edit.expected.child_size.is_some(),
                edit.replacement.child_size.is_some(),
            ),
        ] {
            if old != new {
                return Err(unsupported(
                    "transform coordinate leaf insertion/removal requires structural editing",
                ));
            }
        }
        edit.replacement.apply(transform);
        parts
            .entry(PartName::new(&edit.target.part)?)
            .or_default()
            .insert(binding.ordinal, edit);
    }
    // Propagate changed layout/master declarations before comparing the candidate.
    inheritance::resolve(&mut bound.index.surfaces, check)?;
    let mut plan = RewritePlan::new();
    for (part, edits) in parts {
        cancelled(check)?;
        let original = source.read_part(&part, limits.package.xml.max_bytes as u64, check)?;
        let attributes = binding::attributes(&original, &edits, limits, check)?;
        let changed = mo_xml::rewrite_attributes(
            &original,
            &attributes,
            AttributeRewriteLimits {
                xml: limits.package.xml,
                max_edits: limits
                    .max_edits
                    .checked_mul(11)
                    .ok_or(PptxError::Limit("transform attributes"))?,
                max_edit_bytes: limits.max_text_bytes,
            },
            check,
        )?;
        plan.replace_part(part, changed)?;
    }
    let bytes = plan.to_bytes(source, check)?;
    edit::verify_candidate(bound.index, &bytes, limits, check)?;
    Ok(bytes)
}
