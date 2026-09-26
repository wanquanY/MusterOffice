use super::*;
use crate::{A, PptxError, cancelled};
use mo_opc::PartName;
use mo_xml::{ExpandedName, TextReplacement};
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
    let plan = super::preserve::prepare(
        source,
        request,
        &SourceTransformEdits {
            expected_source_sha256: request.expected_source_sha256.clone(),
            edits: vec![],
        },
        limits,
        check,
    )?;
    plan.write(source, limits, check)
}

pub(super) fn prepare(
    bound: &mut BoundIndex,
    request: &SourceTextEdits,
    check: &dyn Fn() -> bool,
) -> Result<BTreeMap<PartName, Vec<TextReplacement>>, PptxError> {
    let mut selected = BTreeSet::new();
    let mut replacements: BTreeMap<PartName, Vec<TextReplacement>> = BTreeMap::new();
    for edit in &request.edits {
        cancelled(check)?;
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
        let position = bound
            .object_positions
            .get(&edit.target.part)
            .and_then(|p| p.get(&edit.target.object_id))
            .copied()
            .ok_or_else(|| conflict("source object missing"))?;
        let object = surface
            .objects
            .get_mut(position)
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
    Ok(replacements)
}

/// The intended projection is computed before reading the result. The observed
/// candidate may supply only new byte metadata, never expected semantic values.
pub(super) fn verify_candidate(
    mut expected: SourceIndex,
    actual: &dyn PackageRead,
    limits: SourceLimits,
    check: &dyn Fn() -> bool,
) -> Result<(), PptxError> {
    let observed = inspect_source(actual, limits, check)?;
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
