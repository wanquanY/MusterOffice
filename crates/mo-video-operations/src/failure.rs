use mo_video_edit::{EditError, ObjectKey};
use mo_video_model::{ValidationError, ValidationReport};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FailureCode {
    InputInvalid,
    SnapshotCorrupt,
    StaleSnapshot,
    Conflict,
    Locked,
    LinkedSelection,
    NotFound,
    LimitExceeded,
    Cancelled,
    OutsideProject,
    InternalInvariant,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, thiserror::Error)]
#[error("{code:?}: {message}")]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Failure {
    pub code: FailureCode,
    pub message: String,
    pub operation: Option<u32>,
    pub objects: Vec<ObjectKey>,
    pub validation: Option<ValidationReport>,
}
impl Failure {
    pub fn new(code: FailureCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            operation: None,
            objects: vec![],
            validation: None,
        }
    }
}
impl From<EditError> for Failure {
    fn from(error: EditError) -> Self {
        use FailureCode::*;
        let (code, message, objects, validation) = match error {
            EditError::Operation { index, source } => {
                let mut result = Self::from(*source);
                result.operation = Some(index as u32);
                return result;
            }
            EditError::InvalidDocument(report) => {
                (InputInvalid, "invalid video document", vec![], Some(report))
            }
            EditError::Conflict(objects) => (
                Conflict,
                "conditional inverse conflicts with current objects",
                objects,
                None,
            ),
            EditError::StaleSnapshot => (
                StaleSnapshot,
                "expected a different document digest",
                vec![],
                None,
            ),
            EditError::Missing(key) => (NotFound, "object not found", vec![key], None),
            EditError::Duplicate(key) => (InputInvalid, "duplicate object", vec![key], None),
            EditError::Locked(id) => (Locked, "track is locked", vec![ObjectKey::Track(id)], None),
            EditError::LinkedSelection(id) => (
                LinkedSelection,
                "linked selection is incomplete",
                vec![ObjectKey::LinkGroup(id)],
                None,
            ),
            EditError::InvalidOperation(message) => (InputInvalid, message, vec![], None),
            EditError::Budget => (
                LimitExceeded,
                "edit exceeds configured budget",
                vec![],
                None,
            ),
            EditError::Cancelled => (Cancelled, "computation cancelled", vec![], None),
            EditError::Time(error) => return error.into(),
            EditError::Validation(error) => return error.into(),
            EditError::Canonical(_) => (InputInvalid, "noncanonical authoring value", vec![], None),
        };
        Self {
            code,
            message: message.into(),
            operation: None,
            objects,
            validation,
        }
    }
}
impl From<ValidationError> for Failure {
    fn from(error: ValidationError) -> Self {
        use FailureCode::*;
        Self::new(
            match error {
                ValidationError::Cancelled => Cancelled,
                ValidationError::Budget => LimitExceeded,
                ValidationError::Time(e) => return e.into(),
                _ => InputInvalid,
            },
            "video validation failed",
        )
    }
}
impl From<mo_time::TimeError> for Failure {
    fn from(error: mo_time::TimeError) -> Self {
        Self::new(
            if error == mo_time::TimeError::Budget {
                FailureCode::LimitExceeded
            } else {
                FailureCode::InputInvalid
            },
            error.to_string(),
        )
    }
}
impl From<mo_video_compile::CompileError> for Failure {
    fn from(error: mo_video_compile::CompileError) -> Self {
        use FailureCode::*;
        use mo_video_compile::CompileError as E;
        Self::new(
            match error {
                E::Cancelled => Cancelled,
                E::Budget => LimitExceeded,
                E::Outside => OutsideProject,
                E::Invalid(report) => return EditError::InvalidDocument(report).into(),
                E::Validation(e) => return e.into(),
                E::Time(e) => return e.into(),
                E::Invariant(_) => InternalInvariant,
                _ => InputInvalid,
            },
            "video compilation failed",
        )
    }
}
impl From<mo_common::JsonBudgetError> for Failure {
    fn from(error: mo_common::JsonBudgetError) -> Self {
        Self::new(
            match error {
                mo_common::JsonBudgetError::Cancelled => FailureCode::Cancelled,
                mo_common::JsonBudgetError::Limit => FailureCode::LimitExceeded,
                _ => FailureCode::InternalInvariant,
            },
            "JSON output unavailable",
        )
    }
}
