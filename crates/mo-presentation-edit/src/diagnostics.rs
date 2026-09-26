//! Domain diagnostics shared by transport envelopes. No host or wire ownership.
use crate::EditError;
use mo_common::{Digest, OperationId};
use mo_presentation_model::ValidationReport;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EditDiagnosticCode {
    Cancelled,
    InputInvalid,
    RevisionConflict,
    ReferenceConflict,
    RequestIdReused,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditDiagnostic {
    pub code: EditDiagnosticCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operation_ids: Vec<OperationId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_revision: Option<Digest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report: Option<ValidationReport>,
}

impl EditError {
    pub fn diagnostic(&self) -> EditDiagnostic {
        let mut leaf = self;
        let mut operation_ids = Vec::new();
        while let Self::Operation {
            operation_id,
            source,
        } = leaf
        {
            operation_ids.push(operation_id.clone());
            leaf = source;
        }
        let (code, current_revision, report) = match leaf {
            Self::Cancelled => (EditDiagnosticCode::Cancelled, None, None),
            Self::RevisionConflict { current } => (
                EditDiagnosticCode::RevisionConflict,
                Some(current.clone()),
                None,
            ),
            Self::ReferenceConflict(_) => (EditDiagnosticCode::ReferenceConflict, None, None),
            Self::RequestIdReused => (EditDiagnosticCode::RequestIdReused, None, None),
            Self::InvalidDocument(report) => {
                (EditDiagnosticCode::InputInvalid, None, Some(report.clone()))
            }
            _ => (EditDiagnosticCode::InputInvalid, None, None),
        };
        EditDiagnostic {
            code,
            message: self.to_string(),
            operation_ids,
            current_revision,
            report,
        }
    }
}
