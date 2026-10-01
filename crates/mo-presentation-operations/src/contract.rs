use crate::{AssetBinding, ExportSettings, Failure, FailureCode};
use mo_common::{Digest, DocumentId, RequestId, TemplateParameterId};
use mo_presentation_edit::{OperationEntry, TransactionReceipt};
use mo_presentation_model::Document;
use mo_presentation_template::{BindingValue, InstantiationReceipt, TemplateDefinition};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MAX_OPERATION_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum ContractVersion {
    #[serde(rename = "musteroffice.computation/1-draft")]
    V1,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum OperationProfile {
    /// Revisioned authored/retained documents. The historical wire profile name
    /// is retained; it makes no full-slide quality claim.
    #[serde(rename = "presentations-author-model-v01-draft")]
    AuthorModel,
    #[serde(rename = "presentations-pptx-resource-delivery-v1-draft")]
    ResourceDelivery,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum DocumentAction {
    Import {
        document_id: DocumentId,
        source: AssetBinding,
    },
    Create {
        document: Box<Document>,
    },
    /// Ordered pages and editable text/shapes expand into the ordinary native
    /// document. This is a creation input, never a second mutable authority.
    Compose {
        presentation: Box<crate::compose::PresentationContent>,
    },
    /// Validates the caller's pinned source and parameter targets. The result
    /// contains real source values and a digest, without creating a document.
    DescribeTemplate {
        definition: Box<TemplateDefinition>,
    },
    /// The invocation snapshot is the immutable template source, not a base
    /// revision of the new document. Catalog ownership remains with the caller.
    InstantiateTemplate {
        document_id: DocumentId,
        definition: Box<TemplateDefinition>,
        template_digest: Digest,
        bindings: BTreeMap<TemplateParameterId, BindingValue>,
    },
    /// Append a bounded page batch atomically to the pinned native revision.
    Append {
        document_id: DocumentId,
        base_revision: Digest,
        slides: Vec<crate::compose::SlideContent>,
        #[serde(default)]
        resources: Vec<mo_presentation_model::Resource>,
    },
    Apply {
        document_id: DocumentId,
        base_revision: Digest,
        operations: Vec<OperationEntry>,
    },
    /// Export exactly this immutable revision; later edits do not replace it.
    Export {
        document_id: DocumentId,
        base_revision: Digest,
        settings: Box<ExportSettings>,
    },
}
impl DocumentAction {
    pub fn profile(&self) -> OperationProfile {
        match self {
            Self::Import { .. }
            | Self::Create { .. }
            | Self::Compose { .. }
            | Self::Append { .. }
            | Self::Apply { .. }
            | Self::DescribeTemplate { .. }
            | Self::InstantiateTemplate { .. } => OperationProfile::AuthorModel,
            Self::Export { .. } => OperationProfile::ResourceDelivery,
        }
    }
    pub fn name(&self) -> &'static str {
        match self {
            Self::Import { .. } => "presentations.import",
            Self::Create { .. } => "presentations.create",
            Self::Compose { .. } => "presentations.compose",
            Self::DescribeTemplate { .. } => "templates.describe",
            Self::InstantiateTemplate { .. } => "templates.instantiate",
            Self::Apply { .. } => "presentations.apply",
            Self::Append { .. } => "presentations.append",
            Self::Export { .. } => "presentations.export",
        }
    }
    pub fn document_id(&self) -> &DocumentId {
        match self {
            Self::Create { document } => &document.id,
            Self::Compose { presentation } => &presentation.id,
            Self::DescribeTemplate { definition } => &definition.source.document_id,
            Self::Import { document_id, .. }
            | Self::InstantiateTemplate { document_id, .. }
            | Self::Append { document_id, .. }
            | Self::Apply { document_id, .. }
            | Self::Export { document_id, .. } => document_id,
        }
    }
}

/// A computation request carries no identity, permissions, durable job or output mode.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationRequest {
    pub contract_version: ContractVersion,
    pub request_id: RequestId,
    pub profile_id: OperationProfile,
    pub action: DocumentAction,
}
impl OperationRequest {
    pub fn computation(&self) -> Computation<'_> {
        Computation {
            request_id: &self.request_id,
            profile_id: self.profile_id,
            action: &self.action,
        }
    }
    pub fn validate_profile(&self) -> Result<(), Failure> {
        self.computation().validate_profile()
    }
    pub fn digest(&self) -> Result<Digest, mo_common::CanonicalError> {
        self.computation().digest()
    }
}
/// Borrowed input lets compatibility adapters call the same algorithms without
/// cloning an entire document or carrying their transport/host envelope inside.
#[derive(Clone, Copy)]
pub struct Computation<'a> {
    pub request_id: &'a RequestId,
    pub profile_id: OperationProfile,
    pub action: &'a DocumentAction,
}
impl Computation<'_> {
    pub fn validate_profile(&self) -> Result<(), Failure> {
        if self.action.profile() == self.profile_id {
            Ok(())
        } else {
            Err(Failure::new(
                FailureCode::InputInvalid,
                "operation profile differs from action",
            ))
        }
    }
    pub fn digest(&self) -> Result<Digest, mo_common::CanonicalError> {
        mo_common::digest(
            "musteroffice.computation-request/1-draft",
            &(
                ContractVersion::V1,
                self.request_id,
                self.profile_id,
                self.action,
            ),
        )
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MutationReceipt {
    pub document_id: DocumentId,
    pub revision: Digest,
    pub semantic_digest: Digest,
    pub transaction: Option<Box<TransactionReceipt>>,
    /// Source and parameter evidence for a new independent document. It is not
    /// a transaction committed against the source, nor the new document's base.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<Box<InstantiationReceipt>>,
}
