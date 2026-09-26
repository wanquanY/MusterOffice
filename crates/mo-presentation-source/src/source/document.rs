//! Revisioned source-backed documents. Native bytes are immutable provenance;
//! mutable known fields live in the same domain object registry as author data.
mod import;
use super::*;
use crate::{PptxError, cancelled};
use mo_common::{DocumentId, ResourceId, digest};
use mo_presentation_model::{Document, ObjectContent, ValidationLimits};

pub fn import_document(
    source: &dyn PackageRead,
    id: DocumentId,
    resource: ResourceId,
    limits: SourceLimits,
    check: &dyn Fn() -> bool,
) -> Result<Document, PptxError> {
    let bound = presentation::read(source, limits, check)?;
    let d = import::project(&bound, id, resource, check)?;
    validate(&d)?;
    Ok(d)
}
fn validate(d: &Document) -> Result<(), PptxError> {
    let report = mo_presentation_model::validate(d, ValidationLimits::default());
    if !report.is_valid() {
        return Err(PptxError::InvalidDocument(report));
    }
    Ok(())
}
fn conflict(message: &str) -> PptxError {
    PptxError::SourceConflict(message.into())
}

/// A verified field overlay over the original source, shared by native writing
/// and compilation. Constructing it does not emit or reopen a generated package.
/// A logical plan hash never claims to be a physical OPC package digest.
pub struct SourcePlan<'a> {
    document: &'a Document,
    identity: Digest,
    plan: super::preserve::PreservedPlan,
    limits: SourceLimits,
}
impl<'a> SourcePlan<'a> {
    pub fn new(
        document: &'a Document,
        source: &dyn PackageRead,
        limits: SourceLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, PptxError> {
        cancelled(check)?;
        validate(document)?;
        let bindings = document
            .source_bindings
            .as_ref()
            .ok_or_else(|| conflict("source bindings missing"))?;
        if source.sha256() != &document.resources[&bindings.resource].sha256 {
            return Err(conflict("source resource identity changed"));
        }
        let bound = presentation::read(source, limits, check)?;
        let mut baseline = import::project(
            &bound,
            document.id.clone(),
            bindings.resource.clone(),
            check,
        )?;
        let mut text = SourceTextEdits {
            expected_source_sha256: source.sha256().clone(),
            edits: vec![],
        };
        let mut transforms = SourceTransformEdits {
            expected_source_sha256: source.sha256().clone(),
            edits: vec![],
        };
        // Copy back only supported editable leaves. Equality below proves that
        // structural content, native bindings, constraints and every other
        // declaration still match the authorized immutable source projection.
        for (id, current) in &document.objects {
            cancelled(check)?;
            let old = baseline
                .objects
                .get_mut(id)
                .ok_or_else(|| conflict("unbound source object"))?;
            let native = baseline
                .source_bindings
                .as_ref()
                .expect("imported bindings")
                .objects
                .get(id)
                .ok_or_else(|| conflict("source object binding missing"))?;
            if current.transform != old.transform {
                if native.transform_constraint.is_some() {
                    return Err(conflict("protected source transform changed"));
                }
                let t = current
                    .transform
                    .ok_or_else(|| conflict("direct transform removed"))?;
                let position = bound.object_positions[&native.part][&native.native_id];
                let original = &bound.index.surfaces[&native.part].objects[position].transform;
                let expected = SourceTransformValues::from(
                    original.as_ref().expect("complete direct transform"),
                );
                let mut replacement = expected.clone();
                replacement.origin = Some(t.origin);
                replacement.size = Some(t.size);
                if t.rotation != expected.rotation.unwrap_or(0) {
                    replacement.rotation = Some(t.rotation);
                }
                if t.flip_horizontal != expected.flip_horizontal.unwrap_or(false) {
                    replacement.flip_horizontal = Some(t.flip_horizontal);
                }
                if t.flip_vertical != expected.flip_vertical.unwrap_or(false) {
                    replacement.flip_vertical = Some(t.flip_vertical);
                }
                transforms.edits.push(SourceTransformEdit {
                    target: SourceObjectRef {
                        part: native.part.clone(),
                        native_id: native.native_id,
                    },
                    expected,
                    replacement,
                });
                old.transform = current.transform;
            }
            let (
                ObjectContent::RetainedSource {
                    paragraphs: before, ..
                },
                ObjectContent::RetainedSource {
                    paragraphs: after, ..
                },
            ) = (&mut old.content, &current.content)
            else {
                return Err(conflict("retained object kind replaced"));
            };
            for (pi, p) in before.iter_mut().enumerate() {
                let Some(updated) = after.get(pi) else {
                    continue;
                };
                for (ri, run) in p.runs.iter_mut().enumerate() {
                    let Some(updated) = updated.runs.get(ri) else {
                        continue;
                    };
                    if run.text == updated.text {
                        continue;
                    }
                    if native.runs[&run.id].constraint.is_some() {
                        return Err(conflict("protected source text changed"));
                    }
                    text.edits.push(SourceTextEdit {
                        target: SourceTextTarget {
                            part: native.part.clone(),
                            object_id: native.native_id,
                            paragraph: pi as u32,
                            run: ri as u32,
                        },
                        expected_text: run.text.clone(),
                        replacement: updated.text.clone(),
                    });
                    run.text.clone_from(&updated.text);
                }
            }
        }
        if &baseline != document {
            return Err(conflict(
                "source document contains unsupported structural or provenance changes",
            ));
        }
        let identity = digest("musteroffice.source-native-plan/1", document)
            .map_err(|_| conflict("source plan identity"))?;
        let mut plan = super::preserve::prepare_bound(bound, &text, &transforms, limits, check)?;
        plan.index.source_sha256 = identity.clone();
        Ok(Self {
            document,
            identity,
            plan,
            limits,
        })
    }
    pub fn document(&self) -> &Document {
        self.document
    }
    pub fn identity(&self) -> &Digest {
        &self.identity
    }
    pub fn source_identity(&self) -> &Digest {
        &self.plan.original
    }
    pub fn declarations(&self) -> &SourceIndex {
        &self.plan.index
    }
    pub fn write_to<R: ReaderAt, S: mo_opc::ResultSink>(
        &self,
        source: &Package<R>,
        sink: S,
        check: &dyn Fn() -> bool,
    ) -> Result<mo_opc::VerifiedPackage<S::Reader>, PptxError> {
        self.plan.write_to(source, sink, self.limits, check)
    }
    pub fn write<R: ReaderAt>(
        &self,
        source: &Package<R>,
        check: &dyn Fn() -> bool,
    ) -> Result<Vec<u8>, PptxError> {
        self.plan.write(source, self.limits, check)
    }
}
