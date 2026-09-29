use mo_common::{Digest, TemplateParameterId};
use mo_presentation_edit::{EditDiagnostic, EditError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    #[error("template computation cancelled")]
    Cancelled,
    #[error("template computation budget exceeded: {0}")]
    LimitExceeded(&'static str),
    #[error("template source does not match its exact document revision")]
    SourceConflict,
    #[error("template definition changed; current digest is {current}")]
    TemplateConflict { current: Digest },
    #[error("invalid template: {0}")]
    InvalidTemplate(String),
    #[error("parameter {parameter}: {message}")]
    Parameter {
        parameter: TemplateParameterId,
        message: String,
    },
    #[error(transparent)]
    Edit(#[from] EditError),
    #[error(transparent)]
    Canonical(#[from] mo_common::CanonicalError),
}
impl From<mo_common::JsonBudgetError> for TemplateError {
    fn from(error: mo_common::JsonBudgetError) -> Self {
        match error {
            mo_common::JsonBudgetError::Cancelled => Self::Cancelled,
            mo_common::JsonBudgetError::Limit => Self::LimitExceeded("serialized bytes"),
            mo_common::JsonBudgetError::Serialization => {
                Self::InvalidTemplate("input serialization".into())
            }
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TemplateDiagnosticCode {
    Cancelled,
    LimitExceeded,
    SourceConflict,
    TemplateConflict,
    InputInvalid,
    EditRejected,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TemplateDiagnostic {
    pub code: TemplateDiagnosticCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameter: Option<TemplateParameterId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_template: Option<Digest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edit: Option<EditDiagnostic>,
}
impl TemplateError {
    pub fn diagnostic(&self) -> TemplateDiagnostic {
        use TemplateDiagnosticCode as Code;
        let code = match self {
            Self::Cancelled | Self::Edit(EditError::Cancelled) => Code::Cancelled,
            Self::LimitExceeded(_) => Code::LimitExceeded,
            Self::SourceConflict => Code::SourceConflict,
            Self::TemplateConflict { .. } => Code::TemplateConflict,
            Self::Edit(_) => Code::EditRejected,
            _ => Code::InputInvalid,
        };
        TemplateDiagnostic {
            code,
            message: self.to_string(),
            parameter: match self {
                Self::Parameter { parameter, .. } => Some(parameter.clone()),
                _ => None,
            },
            current_template: match self {
                Self::TemplateConflict { current } => Some(current.clone()),
                _ => None,
            },
            edit: match self {
                Self::Edit(error) => Some(error.diagnostic()),
                _ => None,
            },
        }
    }
    pub(crate) fn parameter(id: &TemplateParameterId, message: &str) -> Self {
        Self::Parameter {
            parameter: id.clone(),
            message: message.into(),
        }
    }
}
