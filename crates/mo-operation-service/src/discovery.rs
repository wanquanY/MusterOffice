//! Executable operation catalogue. This describes the current draft service,
//! not the complete presentation-format feature/compatibility registry.
use crate::*;
use mo_common::{ByteLength, Digest};
use mo_presentation_delivery::RendererIdentity;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum ServiceOperation {
    #[serde(rename = "presentations.import")]
    Import,
    #[serde(rename = "capabilities")]
    Capabilities,
    #[serde(rename = "schemas.get")]
    Schema,
    #[serde(rename = "presentations.create")]
    Create,
    #[serde(rename = "presentations.apply")]
    Apply,
    #[serde(rename = "presentations.export")]
    Export,
    #[serde(rename = "presentations.read")]
    ReadDocument,
    #[serde(rename = "assets.beginUpload")]
    BeginUpload,
    #[serde(rename = "assets.getUpload")]
    GetUpload,
    #[serde(rename = "assets.appendUpload")]
    AppendUpload,
    #[serde(rename = "assets.sealUpload")]
    SealUpload,
    #[serde(rename = "assets.cancelUpload")]
    CancelUpload,
    #[serde(rename = "assets.read")]
    ReadAsset,
    #[serde(rename = "assets.readRange")]
    ReadAssetRange,
    #[serde(rename = "jobs.get")]
    GetJob,
    #[serde(rename = "jobs.cancel")]
    CancelJob,
}
impl ServiceOperation {
    pub fn for_action(action: &DocumentAction) -> Self {
        match action {
            DocumentAction::Import { .. } => Self::Import,
            DocumentAction::Create { .. } => Self::Create,
            DocumentAction::Apply { .. } => Self::Apply,
            DocumentAction::Export { .. } => Self::Export,
        }
    }

    pub const ALL: [Self; 16] = [
        Self::Capabilities,
        Self::Schema,
        Self::Import,
        Self::Create,
        Self::Apply,
        Self::Export,
        Self::ReadDocument,
        Self::BeginUpload,
        Self::GetUpload,
        Self::AppendUpload,
        Self::SealUpload,
        Self::CancelUpload,
        Self::ReadAsset,
        Self::ReadAssetRange,
        Self::GetJob,
        Self::CancelJob,
    ];
    pub fn permissions(self) -> &'static [Permission] {
        use Permission as P;
        match self {
            Self::Capabilities | Self::Schema => &[],
            Self::Import => &[P::Create, P::ReadAssets],
            Self::Create => &[P::Create],
            Self::Apply => &[P::Edit],
            Self::Export => &[P::Export, P::ReadDocument, P::ReadAssets],
            Self::ReadDocument => &[P::ReadDocument],
            Self::BeginUpload
            | Self::GetUpload
            | Self::AppendUpload
            | Self::SealUpload
            | Self::CancelUpload => &[P::WriteAssets],
            Self::ReadAsset | Self::ReadAssetRange => &[P::ReadAssets],
            Self::GetJob => &[P::ReadJob],
            Self::CancelJob => &[P::CancelJob],
        }
    }
    pub fn authorize(self, context: &CallContext) -> Result<(), Failure> {
        for permission in self.permissions() {
            context.require(*permission)?;
        }
        Ok(())
    }
    pub fn profile(self) -> Option<OperationProfile> {
        match self {
            Self::Import | Self::Create | Self::Apply => Some(OperationProfile::AuthorModel),
            Self::Export => Some(OperationProfile::ResourceDelivery),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum OperationChannel {
    ControlJson,
    BinaryUpload,
    BinaryRead,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RevisionPolicy {
    NewDocument,
    CompareCurrentHead,
    ImmutableHistorical,
    ReadSelectedOrHead,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum UnavailableReason {
    PreviewRendererNotConfigured,
    ExecutionUnavailable,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationDescriptor {
    pub operation: ServiceOperation,
    pub profile_id: Option<OperationProfile>,
    pub channel: OperationChannel,
    pub required_permissions: Vec<Permission>,
    pub available: bool,
    pub unavailable_reason: Option<UnavailableReason>,
    pub revision_policy: Option<RevisionPolicy>,
    /// Empty for immediate queries/resource operations. Job preference applies
    /// only to document operations accepted into the durable owner.
    pub output_modes: Vec<OutputMode>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServiceLimits {
    pub request_bytes: ByteLength,
    pub asset_chunk_bytes: ByteLength,
    pub asset_bytes: ByteLength,
    pub scope_reserved_bytes: ByteLength,
    pub uploads_per_scope: u32,
    pub upload_ttl_ms: u32,
    pub jobs_per_principal: u32,
    pub documents_per_scope: u32,
    pub lease_ms: u32,
    pub outputs_per_job: u32,
    pub outputs_per_scope: u32,
    pub export: ExportLimits,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportLimits {
    pub pages: u32,
    pub artifacts: u32,
    pub model_bytes: ByteLength,
    pub asset_bytes: ByteLength,
    pub font_bytes: ByteLength,
    pub total_bytes: ByteLength,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum JobExecution {
    ExplicitRun,
    HostScheduled,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostCapabilities {
    pub contract_version: ContractVersion,
    pub executor_digest: Digest,
    pub operations: Vec<OperationDescriptor>,
    pub schemas: Vec<SchemaId>,
    pub renderer: Option<RendererIdentity>,
    pub limits: ServiceLimits,
    pub queued_execution: JobExecution,
    pub complete_feature_catalogue: bool,
    pub full_presentation_acceptance: bool,
}
impl HostCapabilities {
    /// Host facts and authority are injected, never accepted from tool JSON.
    /// This catalogue is advisory. Every call still rechecks its permissions.
    pub fn describe(
        context: &CallContext,
        executor_digest: Digest,
        renderer: Option<RendererIdentity>,
        limits: ServiceLimits,
        queued_execution: JobExecution,
    ) -> Self {
        let operations = ServiceOperation::ALL
            .into_iter()
            .filter(|op| op.authorize(context).is_ok())
            .map(|op| {
                let unavailable_reason = (op == ServiceOperation::Export && renderer.is_none())
                    .then_some(UnavailableReason::PreviewRendererNotConfigured);
                OperationDescriptor {
                    operation: op,
                    profile_id: op.profile(),
                    channel: match op {
                        ServiceOperation::AppendUpload => OperationChannel::BinaryUpload,
                        ServiceOperation::ReadAssetRange => OperationChannel::BinaryRead,
                        _ => OperationChannel::ControlJson,
                    },
                    required_permissions: op.permissions().to_vec(),
                    available: unavailable_reason.is_none(),
                    unavailable_reason,
                    revision_policy: match op {
                        ServiceOperation::Import | ServiceOperation::Create => {
                            Some(RevisionPolicy::NewDocument)
                        }
                        ServiceOperation::Apply => Some(RevisionPolicy::CompareCurrentHead),
                        ServiceOperation::Export => Some(RevisionPolicy::ImmutableHistorical),
                        ServiceOperation::ReadDocument => Some(RevisionPolicy::ReadSelectedOrHead),
                        _ => None,
                    },
                    output_modes: if op.profile().is_some() {
                        vec![OutputMode::Auto, OutputMode::Sync, OutputMode::Job]
                    } else {
                        Vec::new()
                    },
                }
            })
            .collect();
        Self {
            contract_version: ContractVersion::V1,
            executor_digest,
            operations,
            schemas: SchemaId::ALL.to_vec(),
            renderer,
            limits,
            queued_execution,
            complete_feature_catalogue: false,
            full_presentation_acceptance: false,
        }
    }
}
