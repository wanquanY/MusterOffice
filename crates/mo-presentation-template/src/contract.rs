use mo_common::*;
use mo_presentation_edit::{Snapshot, SnapshotRecord, TransactionReceipt};
use mo_presentation_model::{Resource, Rgba, ThemeColor, Transform};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum TemplateVersion {
    #[serde(rename = "musteroffice.presentation-template/1-draft")]
    V1,
}

/// A template is an exact document revision plus typed editable parameters.
/// Catalog identity, ownership, storage and inference are outside this contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TemplateDefinition {
    pub format: TemplateVersion,
    pub source: TemplateSource,
    pub parameters: BTreeMap<TemplateParameterId, Parameter>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TemplateSource {
    pub document_id: DocumentId,
    pub revision: Digest,
    pub semantic_digest: Digest,
}
impl TemplateSource {
    pub fn of(snapshot: &Snapshot) -> Self {
        Self {
            document_id: snapshot.document().id.clone(),
            revision: snapshot.revision().clone(),
            semantic_digest: snapshot.semantic_digest().clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Parameter {
    pub label: String,
    pub required: bool,
    pub target: ParameterTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ParameterTarget {
    /// Replaces one complete run, preserving its style, paragraph and identity.
    /// Newlines/structured runs still obey the original editor/source rules.
    TextRun {
        object: ObjectId,
        paragraph: ParagraphId,
        run: RunId,
        min_scalars: u32,
        max_scalars: u32,
    },
    /// Replaces this declaration for every consumer of this resource identity.
    /// Never rewrites source-package provenance or authorizes resource bytes.
    Resource {
        resource: ResourceId,
        media_types: BTreeSet<String>,
    },
    ThemeColor {
        theme: ThemeId,
        slot: ThemeColor,
    },
    Transform {
        object: ObjectId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum BindingValue {
    Text(String),
    Resource(Resource),
    Color(Rgba),
    Transform(Transform),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TemplateDescription {
    pub template_digest: Digest,
    pub definition: TemplateDefinition,
    /// Examples are the real values of the pinned source, never inferred text.
    pub examples: BTreeMap<TemplateParameterId, BindingValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstantiateRequest {
    pub request_id: RequestId,
    pub template_digest: Digest,
    pub document_id: DocumentId,
    pub bindings: BTreeMap<TemplateParameterId, BindingValue>,
}

/// Total mapping for whole-document instantiation: (source document, local ID)
/// becomes (instance document, same local ID). This preserves every internal
/// reference, including opaque native relationships, without an O(n) identity
/// table. It is not a mapping for copying a page into an existing document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DocumentScopeMap {
    pub source_document: DocumentId,
    pub instance_document: DocumentId,
    pub local_id_policy: LocalIdPolicy,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum LocalIdPolicy {
    Preserve,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstantiationReceipt {
    pub request_id: RequestId,
    pub request_digest: Digest,
    pub template_digest: Digest,
    pub source: TemplateSource,
    pub scope_map: DocumentScopeMap,
    pub bound_parameters: BTreeSet<TemplateParameterId>,
    pub revision: Digest,
    pub semantic_digest: Digest,
    /// Pure parameter computation against the immutable source. This is not a
    /// commit to the template or the instance. Scope mapping follows binding.
    pub binding_transaction: Option<Box<TransactionReceipt>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TemplateInstance {
    pub snapshot: SnapshotRecord,
    pub receipt: InstantiationReceipt,
}
