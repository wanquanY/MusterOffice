//! Pure, revision-pinned presentation template computation. Parameters reuse
//! the ordinary atomic editor; no catalog, permissions, storage, UI or model.
mod bindings;
mod computation;
mod contract;
mod error;
pub use computation::*;
pub use contract::*;
pub use error::*;
use mo_common::*;
use mo_presentation_edit::{Snapshot, Transaction, prepare_cancellable};
use mo_presentation_model::{ValidationLimits, validate};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy)]
pub struct TemplateLimits {
    pub document: ValidationLimits,
    pub max_parameters: usize,
    pub max_label_bytes: usize,
    pub max_bytes: usize,
}
impl Default for TemplateLimits {
    fn default() -> Self {
        Self {
            document: ValidationLimits::default(),
            max_parameters: 4096,
            max_label_bytes: 4096,
            max_bytes: 32 * 1024 * 1024,
        }
    }
}

/// Owns an immutable validated source. Preparation validates every declared
/// target through the editor once, including native protected-field rules.
pub struct Template {
    source: Snapshot,
    definition: TemplateDefinition,
    template_digest: Digest,
    examples: BTreeMap<TemplateParameterId, BindingValue>,
    limits: TemplateLimits,
}
impl Template {
    pub fn new(
        source: Snapshot,
        definition: TemplateDefinition,
        limits: TemplateLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, TemplateError> {
        cancelled(check)?;
        check_size(&(source.document(), &definition), limits.max_bytes, check)?;
        if definition.source != TemplateSource::of(&source) {
            return Err(TemplateError::SourceConflict);
        }
        if definition.parameters.len() > limits.max_parameters
            || definition.parameters.len() > 10_000
        {
            return Err(TemplateError::LimitExceeded("parameter count"));
        }
        let report = validate(source.document(), limits.document);
        if !report.is_valid() {
            return Err(mo_presentation_edit::EditError::InvalidDocument(report).into());
        }
        cancelled(check)?;
        let mut targets = BTreeSet::new();
        let mut examples = BTreeMap::new();
        for (id, parameter) in &definition.parameters {
            cancelled(check)?;
            if parameter.label.len() > limits.max_label_bytes {
                return Err(TemplateError::LimitExceeded("parameter label bytes"));
            }
            if !targets.insert(bindings::target_key(&parameter.target)?) {
                return Err(TemplateError::parameter(
                    id,
                    "another parameter already owns this target",
                ));
            }
            match &parameter.target {
                ParameterTarget::TextRun {
                    min_scalars,
                    max_scalars,
                    ..
                } if min_scalars > max_scalars => {
                    return Err(TemplateError::parameter(
                        id,
                        "text constraint range is inverted",
                    ));
                }
                ParameterTarget::Resource { media_types, .. }
                    if media_types.is_empty()
                        || media_types.iter().any(|v| {
                            v.len() > 256
                                || !v.is_ascii()
                                || !v.contains('/')
                                || v.bytes()
                                    .any(|b| b.is_ascii_whitespace() || b.is_ascii_control())
                        }) =>
                {
                    return Err(TemplateError::parameter(
                        id,
                        "resource needs explicit canonical media types",
                    ));
                }
                _ => {}
            }
            examples.insert(
                id.clone(),
                bindings::example(source.document(), &parameter.target).ok_or_else(|| {
                    TemplateError::parameter(id, "target is missing or has the wrong type")
                })?,
            );
        }
        let template_digest = digest("musteroffice.presentation-template/1-draft", &definition)?;
        let prepared = Self {
            source,
            definition,
            template_digest,
            examples,
            limits,
        };
        let operations = prepared.operations(&prepared.examples, check)?;
        if !operations.is_empty() {
            prepare_cancellable(
                &prepared.source,
                &Transaction {
                    document_id: prepared.source.document().id.clone(),
                    request_id: RequestId::new("template:validate-targets")
                        .expect("static identifier"),
                    base_revision: prepared.source.revision().clone(),
                    operations,
                },
                limits.document,
                check,
            )?;
        }
        cancelled(check)?;
        Ok(prepared)
    }

    pub fn digest(&self) -> &Digest {
        &self.template_digest
    }
    pub fn definition(&self) -> &TemplateDefinition {
        &self.definition
    }
    pub fn describe(&self, check: &dyn Fn() -> bool) -> Result<TemplateDescription, TemplateError> {
        cancelled(check)?;
        let result = TemplateDescription {
            template_digest: self.template_digest.clone(),
            definition: self.definition.clone(),
            examples: self.examples.clone(),
        };
        check_size(&result, self.limits.max_bytes, check)?;
        Ok(result)
    }

    pub fn instantiate(
        &self,
        request: &InstantiateRequest,
        check: &dyn Fn() -> bool,
    ) -> Result<TemplateInstance, TemplateError> {
        cancelled(check)?;
        check_size(request, self.limits.max_bytes, check)?;
        if request.template_digest != self.template_digest {
            return Err(TemplateError::TemplateConflict {
                current: self.template_digest.clone(),
            });
        }
        if request.document_id == self.source.document().id {
            return Err(TemplateError::InvalidTemplate(
                "an instance requires a different document identity".into(),
            ));
        }
        let operations = self.operations(&request.bindings, check)?;
        cancelled(check)?;
        // The ordinary editor already makes one private copy. Bind there and
        // move that value into the new scope, avoiding a second full-deck copy.
        let (mut document, binding_transaction) = if operations.is_empty() {
            (self.source.document().clone(), None)
        } else {
            let prepared = prepare_cancellable(
                &self.source,
                &Transaction {
                    document_id: self.source.document().id.clone(),
                    request_id: request.request_id.clone(),
                    base_revision: self.source.revision().clone(),
                    operations,
                },
                self.limits.document,
                check,
            )?;
            (
                prepared.snapshot.into_record().document,
                Some(Box::new(prepared.receipt)),
            )
        };
        if let Some(bindings) = &mut document.source_bindings {
            bindings
                .identity_scope
                .get_or_insert_with(|| document.id.clone());
        }
        document.id = request.document_id.clone();
        let snapshot = Snapshot::new(document, self.limits.document)?.into_record();
        cancelled(check)?;
        let receipt = InstantiationReceipt {
            request_id: request.request_id.clone(),
            request_digest: digest("musteroffice.template-instantiation/1", request)?,
            template_digest: self.template_digest.clone(),
            source: self.definition.source.clone(),
            scope_map: DocumentScopeMap {
                source_document: self.source.document().id.clone(),
                instance_document: request.document_id.clone(),
                local_id_policy: LocalIdPolicy::Preserve,
            },
            bound_parameters: request.bindings.keys().cloned().collect(),
            revision: snapshot.revision.clone(),
            semantic_digest: snapshot.semantic_digest.clone(),
            binding_transaction,
        };
        let result = TemplateInstance { snapshot, receipt };
        check_size(&result, self.limits.max_bytes, check)?;
        cancelled(check)?;
        Ok(result)
    }
}

fn check_size(
    value: &impl serde::Serialize,
    limit: usize,
    check: &dyn Fn() -> bool,
) -> Result<(), TemplateError> {
    mo_common::check_json_size(value, limit, check)
        .map(|_| ())
        .map_err(Into::into)
}
fn cancelled(check: &dyn Fn() -> bool) -> Result<(), TemplateError> {
    if check() {
        Err(TemplateError::Cancelled)
    } else {
        Ok(())
    }
}
