use crate::{
    DocumentAction, ExportReceipt, Failure, FailureCode, MutationReceipt, OperationRequest,
};
use mo_common::{Digest, RequestId};
use mo_presentation_edit::SnapshotRecord;
use mo_presentation_template::TemplateDescription;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The operation and its caller-supplied base. Resources are transferred outside
/// JSON; their logical identities are already bound by the operation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Invocation {
    pub request: OperationRequest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<Box<SnapshotRecord>>,
}
impl Invocation {
    pub fn validate(&self) -> Result<(), Failure> {
        self.validate_cancellable(&|| false)
    }
    pub fn validate_cancellable(&self, cancelled: &dyn Fn() -> bool) -> Result<(), Failure> {
        self.request.validate_profile()?;
        match (&self.request.action, &self.snapshot) {
            (DocumentAction::Create { .. } | DocumentAction::Import { .. }, Some(_)) => {
                Err(Failure::new(
                    FailureCode::InputInvalid,
                    "new document has no base snapshot",
                ))
            }
            (
                DocumentAction::Apply { .. }
                | DocumentAction::Export { .. }
                | DocumentAction::DescribeTemplate { .. }
                | DocumentAction::InstantiateTemplate { .. },
                None,
            ) => Err(Failure::new(
                FailureCode::NotFound,
                "base snapshot must be provided",
            )),
            _ => Ok(()),
        }?;
        crate::budget::check_size(
            &self.request,
            crate::MAX_OPERATION_BYTES,
            "operation bytes",
            cancelled,
        )?;
        if let Some(snapshot) = &self.snapshot {
            crate::budget::check_size(
                snapshot,
                crate::MAX_OPERATION_BYTES,
                "snapshot bytes",
                cancelled,
            )?;
        }
        Ok(())
    }
}

/// A completed computation, never a durable task or a product publication.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComputationReceipt {
    pub request_id: RequestId,
    pub request_digest: Digest,
    pub result: ComputationResult,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ComputationResult {
    DescribedTemplate {
        description: Box<TemplateDescription>,
    },
    Mutated {
        snapshot: Box<SnapshotRecord>,
        receipt: MutationReceipt,
    },
    Exported {
        receipt: Box<ExportReceipt>,
    },
}

// A request and a base may each occupy their existing 32 MiB budget.
pub const MAX_INVOCATION_BYTES: usize = 65 * 1024 * 1024;

pub fn decode_invocation(input: &str) -> Result<Invocation, Failure> {
    if input.len() > MAX_INVOCATION_BYTES {
        return Err(Failure::new(FailureCode::LimitExceeded, "invocation bytes"));
    }
    let invocation: Invocation = mo_common::from_json_str(input)
        .map_err(|_| Failure::new(FailureCode::InputInvalid, "invalid invocation"))?;
    invocation.validate()?;
    Ok(invocation)
}
