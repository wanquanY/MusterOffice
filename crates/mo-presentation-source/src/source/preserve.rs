//! One admission and one rewrite plan for coordinated native leaf edits.
use super::*;
use crate::{PptxError, cancelled};
use mo_opc::{PartName, RewritePlan};
use mo_xml::{AttributeRewriteLimits, TextReplacement, TextRewriteLimits};
use std::collections::BTreeSet;

pub(super) struct PreservedPlan {
    pub index: SourceIndex,
    pub original: Digest,
    text: BTreeMap<PartName, Vec<TextReplacement>>,
    transforms: BTreeMap<PartName, BTreeMap<u32, SourceTransformEdit>>,
}
pub(super) fn prepare(
    source: &dyn PackageRead,
    text: &SourceTextEdits,
    transforms: &SourceTransformEdits,
    limits: SourceLimits,
    check: &dyn Fn() -> bool,
) -> Result<PreservedPlan, PptxError> {
    admit(source.sha256(), text, transforms, limits, check)?;
    let bound = presentation::read(source, limits, check)?;
    prepare_bound(bound, text, transforms, limits, check)
}
pub(super) fn prepare_bound(
    mut bound: BoundIndex,
    text: &SourceTextEdits,
    transforms: &SourceTransformEdits,
    limits: SourceLimits,
    check: &dyn Fn() -> bool,
) -> Result<PreservedPlan, PptxError> {
    let original = bound.index.source_sha256.clone();
    admit(&original, text, transforms, limits, check)?;
    let text = edit::prepare(&mut bound, text, check)?;
    let transforms = transform_edit::prepare(&mut bound, transforms, check)?
        .into_iter()
        .map(|(part, edits)| {
            (
                part,
                edits.into_iter().map(|(i, e)| (i, e.clone())).collect(),
            )
        })
        .collect();
    resolve_projection(&mut bound.index, check)?;
    Ok(PreservedPlan {
        index: bound.index,
        original,
        text,
        transforms,
    })
}
// Budget admission precedes XML/source parsing for standalone calls, and also
// covers overlays prepared from an already inspected immutable source.
fn admit(
    identity: &Digest,
    text: &SourceTextEdits,
    transforms: &SourceTransformEdits,
    limits: SourceLimits,
    check: &dyn Fn() -> bool,
) -> Result<(), PptxError> {
    cancelled(check)?;
    if identity != &text.expected_source_sha256 || identity != &transforms.expected_source_sha256 {
        return Err(PptxError::SourceConflict(
            "source package digest changed".into(),
        ));
    }
    if text.edits.len().saturating_add(transforms.edits.len()) > limits.max_edits {
        return Err(PptxError::Limit("combined native edits"));
    }
    let mut bytes = 0usize;
    for length in text
        .edits
        .iter()
        .flat_map(|e| {
            [
                e.target.part.len(),
                e.expected_text.len(),
                e.replacement.len(),
            ]
        })
        .chain(transforms.edits.iter().map(|e| e.target.part.len()))
    {
        cancelled(check)?;
        bytes = bytes
            .checked_add(length)
            .ok_or(PptxError::Limit("source edit bytes"))?;
        if bytes > limits.max_text_bytes {
            return Err(PptxError::Limit("source edit bytes"));
        }
    }
    Ok(())
}

impl PreservedPlan {
    pub fn write<R: ReaderAt>(
        &self,
        source: &Package<R>,
        limits: SourceLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Vec<u8>, PptxError> {
        Ok(self
            .write_to(source, Vec::new(), limits, check)?
            .into_reader())
    }
    pub fn write_to<R: ReaderAt, S: mo_opc::ResultSink>(
        &self,
        source: &Package<R>,
        sink: S,
        limits: SourceLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<mo_opc::VerifiedPackage<S::Reader>, PptxError> {
        cancelled(check)?;
        if source.sha256() != &self.original {
            return Err(PptxError::SourceConflict(
                "source package digest changed".into(),
            ));
        }
        let mut plan = RewritePlan::new();
        let parts: BTreeSet<_> = self.text.keys().chain(self.transforms.keys()).collect();
        for part in parts {
            cancelled(check)?;
            let mut bytes = source.read_part(part, limits.package.xml.max_bytes as u64, check)?;
            if let Some(text) = self.text.get(part) {
                bytes = mo_xml::rewrite_text(
                    &bytes,
                    text,
                    TextRewriteLimits {
                        xml: limits.package.xml,
                        max_replacements: limits.max_edits,
                        ..Default::default()
                    },
                    check,
                )?;
            }
            if let Some(edits) = self.transforms.get(part) {
                let edits = edits.iter().map(|(i, e)| (*i, e)).collect();
                let attributes =
                    transform_edit::binding::attributes(&bytes, &edits, limits, check)?;
                bytes = mo_xml::rewrite_attributes(
                    &bytes,
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
            }
            plan.replace_part(part.clone(), bytes)?;
        }
        let package = plan.write_sealed(source, sink, check)?;
        edit::verify_candidate(self.index.clone(), package.package(), limits, check)?;
        Ok(package)
    }
}
