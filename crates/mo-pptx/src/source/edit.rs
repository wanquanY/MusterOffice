use super::*;
use crate::{A, PptxError, cancelled};
use mo_opc::{PartName, RewritePlan};
use mo_xml::{ExpandedName, TextReplacement, TextRewriteLimits};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTextEdits {
    pub expected_source_sha256: Digest,
    pub edits: Vec<SourceTextEdit>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTextEdit {
    pub target: SourceTextTarget,
    pub expected_text: String,
    pub replacement: String,
}

/// Computes an atomic candidate. Host still owns immutable source authorization,
/// persistence/CAS and publication. The source is re-indexed here, never trusted
/// from client-supplied XML offsets, object names or prior projection JSON.
pub fn edit_source_text<R: ReaderAt>(
    source: &Package<R>,
    request: &SourceTextEdits,
    limits: SourceLimits,
    check: &dyn Fn() -> bool,
) -> Result<Vec<u8>, PptxError> {
    cancelled(check)?;
    if source.sha256() != &request.expected_source_sha256 {
        return Err(conflict("source package digest changed"));
    }
    if request.edits.len() > limits.max_edits {
        return Err(PptxError::Limit("source text edits"));
    }
    let mut bound = presentation::read(source, limits, check)?;
    let mut selected = BTreeSet::new();
    let mut replacements: BTreeMap<PartName, Vec<TextReplacement>> = BTreeMap::new();
    let mut edit_bytes = 0_usize;
    for edit in &request.edits {
        cancelled(check)?;
        edit_bytes = edit_bytes
            .checked_add(edit.expected_text.len())
            .and_then(|n| n.checked_add(edit.replacement.len()))
            .ok_or(PptxError::Limit("source edit bytes"))?;
        if edit_bytes > limits.max_text_bytes {
            return Err(PptxError::Limit("source edit bytes"));
        }
        if !selected.insert(&edit.target) {
            return Err(conflict("duplicate source text target"));
        }
        let ordinal = bound
            .bindings
            .get(&edit.target)
            .ok_or_else(|| conflict("source target has no ordinary text binding"))?;
        let surface = bound
            .index
            .surfaces
            .get_mut(&edit.target.part)
            .ok_or_else(|| conflict("source surface missing"))?;
        let object = surface
            .objects
            .iter_mut()
            .find(|o| o.native_id == edit.target.object_id)
            .ok_or_else(|| conflict("source object missing"))?;
        let run = object
            .paragraphs
            .get_mut(edit.target.paragraph as usize)
            .and_then(|p| p.get_mut(edit.target.run as usize))
            .ok_or_else(|| conflict("source text run missing"))?;
        if run.text != edit.expected_text {
            return Err(conflict("source text precondition changed"));
        }
        if edit.replacement == edit.expected_text {
            continue;
        }
        if !surface.text_edit_barriers.is_empty() {
            return Err(PptxError::Unsupported(
                surface.text_edit_barriers.join("; "),
            ));
        }
        if !run.editable {
            return Err(PptxError::Unsupported(format!(
                "source text needs coordinated semantic editing: {:?}",
                run.edit_constraint
            )));
        }
        run.text = edit.replacement.clone();
        replacements
            .entry(PartName::new(&edit.target.part)?)
            .or_default()
            .push(TextReplacement {
                element_ordinal: *ordinal,
                expected_name: ExpandedName {
                    namespace: A.into(),
                    local: "t".into(),
                },
                expected_text: edit.expected_text.clone(),
                replacement: edit.replacement.clone(),
            });
    }
    let mut plan = RewritePlan::new();
    for (part, edits) in replacements {
        let original = source.read_part(&part, limits.package.xml.max_bytes as u64, check)?;
        let changed = mo_xml::rewrite_text(
            &original,
            &edits,
            TextRewriteLimits {
                xml: limits.package.xml,
                max_replacements: limits.max_edits,
                ..Default::default()
            },
            check,
        )?;
        plan.replace_part(part, changed)?;
    }
    let bytes = plan.to_bytes(source, check)?;
    verify_candidate(bound.index, &bytes, limits, check)?;
    Ok(bytes)
}

/// The intended projection is computed before reading the result. The observed
/// candidate may supply only new byte metadata, never expected semantic values.
pub(super) fn verify_candidate(
    mut expected: SourceIndex,
    bytes: &[u8],
    limits: SourceLimits,
    check: &dyn Fn() -> bool,
) -> Result<(), PptxError> {
    let actual = Package::open(bytes, bytes.len() as u64, limits.package, check)?;
    let observed = inspect_source(&actual, limits, check)?;
    expected.source_sha256 = observed.source_sha256.clone();
    expected.byte_length = observed.byte_length;
    for (part, surface) in &mut expected.surfaces {
        surface.sha256 = observed
            .surfaces
            .get(part)
            .ok_or_else(|| conflict("candidate surface missing"))?
            .sha256
            .clone();
    }
    if expected != observed {
        return Err(conflict(
            "actual candidate source projection differs from intended edit",
        ));
    }
    Ok(())
}

fn conflict(message: &str) -> PptxError {
    PptxError::SourceConflict(message.into())
}
