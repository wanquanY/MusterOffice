use crate::*;
use mo_common::{Digest, DocumentId, RequestId};
use mo_presentation_edit::{OperationEntry, SnapshotRecord, TransactionReceipt};
use mo_presentation_model::Document;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const MAX_OPERATION_BYTES: usize = 32 * 1024 * 1024;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum ContractVersion {
    #[serde(rename = "musteroffice.operations/1-draft")]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum OutputMode {
    Auto,
    Sync,
    Job,
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
    pub fn service_operation(&self) -> ServiceOperation {
        match self {
            Self::Import { .. } => ServiceOperation::Import,
            Self::Create { .. } => ServiceOperation::Create,
            Self::Apply { .. } => ServiceOperation::Apply,
            Self::Export { .. } => ServiceOperation::Export,
        }
    }
    pub fn name(&self) -> &'static str {
        match self {
            Self::Import { .. } => "presentations.import",
            Self::Create { .. } => "presentations.create",
            Self::Apply { .. } => "presentations.apply",
            Self::Export { .. } => "presentations.export",
        }
    }
    pub fn document_id(&self) -> &DocumentId {
        match self {
            Self::Create { document } => &document.id,
            Self::Import { document_id, .. }
            | Self::Apply { document_id, .. }
            | Self::Export { document_id, .. } => document_id,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationRequest {
    pub contract_version: ContractVersion,
    pub request_id: RequestId,
    pub profile_id: OperationProfile,
    pub output_mode: OutputMode,
    pub action: DocumentAction,
}
impl OperationRequest {
    pub fn validate_profile(&self) -> Result<(), Failure> {
        let valid = self.action.service_operation().profile() == Some(self.profile_id);
        if valid {
            Ok(())
        } else {
            Err(Failure::new(
                FailureCode::InputInvalid,
                "operation profile differs from action",
            ))
        }
    }
    /// Delivery preference is not part of logical operation identity. Retrying a
    /// queued operation through a synchronous client must observe the same job.
    pub fn digest(&self) -> Result<Digest, mo_common::CanonicalError> {
        mo_common::digest(
            "musteroffice.operation-request/1-draft",
            &(
                &self.contract_version,
                &self.request_id,
                &self.profile_id,
                &self.action,
            ),
        )
    }
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum Permission {
    Create,
    Edit,
    Export,
    ReadDocument,
    ReadJob,
    CancelJob,
    WriteAssets,
    ReadAssets,
}
/// No Deserialize implementation: transports must authenticate and inject this.
#[derive(Debug, Clone)]
pub struct CallContext {
    pub principal: PrincipalId,
    pub scope: ScopeId,
    pub permissions: BTreeSet<Permission>,
}
impl CallContext {
    pub fn require(&self, permission: Permission) -> Result<(), Failure> {
        if self.permissions.contains(&permission) {
            Ok(())
        } else {
            Err(Failure::new(
                FailureCode::NotAuthorized,
                "operation is not authorized",
            ))
        }
    }
    pub fn authorize(&self, request: &OperationRequest) -> Result<(), Failure> {
        request.validate_profile()?;
        request.action.service_operation().authorize(self)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FailureCode {
    InputInvalid,
    NotAuthorized,
    NotFound,
    RequestIdReused,
    RevisionConflict,
    ReferenceConflict,
    DocumentExists,
    LimitExceeded,
    Cancelled,
    ExecutionInterrupted,
    ExecutorMismatch,
    StaleExecution,
    /// Temporary storage contention; the accepted request identity remains reusable.
    StorageBusy,
    StorageFailure,
    ResourceConflict,
    ResourceExpired,
    ResourceIncomplete,
    ResourceBusy,
    MappingNotImplemented,
    RenderFailure,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, thiserror::Error)]
#[error("{code:?}: {message}")]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Failure {
    pub code: FailureCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<Box<serde_json::Value>>,
}
impl Failure {
    pub fn new(code: FailureCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            detail: None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}
impl JobState {
    pub fn terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MutationReceipt {
    pub document_id: DocumentId,
    pub revision: Digest,
    pub semantic_digest: Digest,
    pub transaction: Option<Box<TransactionReceipt>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "outcome", rename_all = "camelCase", deny_unknown_fields)]
pub enum TerminalResult {
    Succeeded { receipt: Box<OperationReceipt> },
    Failed { error: Failure },
}

/// The mutation wire shape is preserved for existing durable receipts. Both
/// alternatives deny unknown fields and have distinct required members.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum OperationReceipt {
    Mutation(MutationReceipt),
    Export(Box<ExportReceipt>),
}
impl OperationReceipt {
    pub fn document_id(&self) -> &DocumentId {
        match self {
            Self::Mutation(r) => &r.document_id,
            Self::Export(r) => &r.document_id,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JobInfo {
    pub contract_version: ContractVersion,
    pub id: JobId,
    pub request_id: RequestId,
    pub operation: String,
    pub document_id: DocumentId,
    pub request_digest: Digest,
    pub executor_digest: Digest,
    pub state: JobState,
    pub cancel_requested: bool,
    pub fence: JobFence,
    pub created_at: UnixMillis,
    pub updated_at: UnixMillis,
    pub lease_until: Option<UnixMillis>,
    /// Receipts remain durable until explicit host lifecycle management; there is
    /// no advertised expiry or automatic deletion of committed documents.
    pub result: Option<TerminalResult>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "operation",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum HostRequest {
    Capabilities {},
    GetSchema {
        id: SchemaId,
    },
    BeginUpload {
        request: UploadRequest,
    },
    GetUpload {
        upload_id: UploadId,
    },
    SealUpload {
        upload_id: UploadId,
    },
    CancelUpload {
        upload_id: UploadId,
    },
    ReadAsset {
        asset_id: AssetId,
    },
    Submit {
        request: Box<OperationRequest>,
    },
    GetJob {
        job_id: JobId,
    },
    CancelJob {
        job_id: JobId,
    },
    ReadDocument {
        document_id: DocumentId,
        revision: Option<Digest>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "outcome",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum HostResponse {
    Accepted {
        job: Box<JobInfo>,
        poll_after_ms: u32,
    },
    Succeeded {
        result: HostResult,
    },
    Failed {
        error: Failure,
        job: Option<Box<JobInfo>>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum HostResult {
    Capabilities { capabilities: Box<HostCapabilities> },
    Schema { document: Box<SchemaDocument> },
    Upload { upload: Box<UploadInfo> },
    Asset { asset: AssetInfo },
    Job { job: Box<JobInfo> },
    Document { snapshot: Box<SnapshotRecord> },
}
impl HostResponse {
    pub fn upload(upload: UploadInfo) -> Self {
        match upload.error.as_ref() {
            Some(error) => Self::Failed {
                error: error.clone(),
                job: None,
            },
            None => Self::Succeeded {
                result: HostResult::Upload {
                    upload: Box::new(upload),
                },
            },
        }
    }
    pub fn job(job: JobInfo) -> Self {
        match &job.result {
            Some(TerminalResult::Succeeded { .. }) => Self::Succeeded {
                result: HostResult::Job { job: Box::new(job) },
            },
            Some(TerminalResult::Failed { error }) => Self::Failed {
                error: error.clone(),
                job: Some(Box::new(job)),
            },
            None => Self::Accepted {
                job: Box::new(job),
                poll_after_ms: 1000,
            },
        }
    }
}
