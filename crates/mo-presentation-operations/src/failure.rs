use mo_pptx::PptxError;
use mo_presentation_delivery::DeliveryError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FailureCode {
    InputInvalid,
    NotFound,
    RequestIdReused,
    RevisionConflict,
    ReferenceConflict,
    DocumentExists,
    LimitExceeded,
    Cancelled,
    ExecutionInterrupted,
    ExecutorMismatch,
    IoFailure,
    ResultMismatch,
    ResourceConflict,
    ResourceIncomplete,
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
pub fn delivery_failure(error: DeliveryError) -> Failure {
    // Transparent error sources may delegate past the io::Error itself.
    let storage = match &error {
        DeliveryError::Io(e)
        | DeliveryError::Png(mo_image::png::PngError::Io(e))
        | DeliveryError::Pptx(PptxError::Opc(mo_opc::OpcError::Io(e))) => Some(e),
        _ => None,
    };
    if let Some(io) = storage {
        if let Some(f) = io.get_ref().and_then(|e| e.downcast_ref::<Failure>()) {
            return f.clone();
        }
        return Failure::new(
            FailureCode::IoFailure,
            "export input/output operation failed",
        );
    }
    // Preserve computational diagnostics through OPC/PNG/io wrappers.
    let mut cause: Option<&(dyn std::error::Error + 'static)> = Some(&error);
    while let Some(current) = cause {
        if let Some(f) = current.downcast_ref::<Failure>() {
            return f.clone();
        }
        if let Some(io) = current.downcast_ref::<std::io::Error>() {
            if let Some(f) = io.get_ref().and_then(|e| e.downcast_ref::<Failure>()) {
                return f.clone();
            }
            return Failure::new(
                FailureCode::IoFailure,
                "export input/output operation failed",
            );
        }
        cause = current.source();
    }
    fn xml_code(e: &mo_xml::XmlError) -> FailureCode {
        match e {
            mo_xml::XmlError::Cancelled => FailureCode::Cancelled,
            mo_xml::XmlError::Limit(_) => FailureCode::LimitExceeded,
            _ => FailureCode::InputInvalid,
        }
    }
    fn opc_code(e: &mo_opc::OpcError) -> FailureCode {
        match e {
            mo_opc::OpcError::Cancelled => FailureCode::Cancelled,
            mo_opc::OpcError::Limit(_) => FailureCode::LimitExceeded,
            mo_opc::OpcError::Preservation(_) => FailureCode::ResourceConflict,
            mo_opc::OpcError::Unsupported(_) => FailureCode::MappingNotImplemented,
            mo_opc::OpcError::Xml { source, .. } => xml_code(source),
            mo_opc::OpcError::Io(_) => FailureCode::IoFailure,
            _ => FailureCode::InputInvalid,
        }
    }
    let code = match &error {
        DeliveryError::Cancelled => FailureCode::Cancelled,
        DeliveryError::Limit(_) => FailureCode::LimitExceeded,
        DeliveryError::Io(e) => {
            if let Some(f) = e.get_ref().and_then(|v| v.downcast_ref::<Failure>()) {
                return f.clone();
            }
            FailureCode::IoFailure
        }
        DeliveryError::Preview { .. } => FailureCode::RenderFailure,
        DeliveryError::Pptx(p) => match p {
            PptxError::Cancelled => FailureCode::Cancelled,
            PptxError::Limit(_) => FailureCode::LimitExceeded,
            PptxError::Unsupported(_) => FailureCode::MappingNotImplemented,
            PptxError::ResourceRequired(_) => FailureCode::ResourceIncomplete,
            PptxError::SourceConflict(_) => FailureCode::ResourceConflict,
            PptxError::Opc(e) => opc_code(e),
            PptxError::Xml(e) => xml_code(e),
            _ => FailureCode::InputInvalid,
        },
        DeliveryError::Png(p) => match p {
            mo_image::png::PngError::Cancelled => FailureCode::Cancelled,
            mo_image::png::PngError::Limit(_) => FailureCode::LimitExceeded,
            mo_image::png::PngError::Io(_) => FailureCode::IoFailure,
            _ => FailureCode::InputInvalid,
        },
        DeliveryError::Invalid(_) | DeliveryError::Serialization => FailureCode::InputInvalid,
    };
    let mut failure = Failure::new(code, error.to_string());
    if let DeliveryError::Preview { diagnostic, .. } = error {
        failure.detail = diagnostic;
    }
    failure
}

impl From<DeliveryError> for Failure {
    fn from(error: DeliveryError) -> Self {
        delivery_failure(error)
    }
}

impl From<mo_presentation_edit::EditError> for Failure {
    fn from(error: mo_presentation_edit::EditError) -> Self {
        use mo_presentation_edit::EditDiagnosticCode as Code;
        let diagnostic = error.diagnostic();
        let code = match diagnostic.code {
            Code::Cancelled => FailureCode::Cancelled,
            Code::RevisionConflict => FailureCode::RevisionConflict,
            Code::ReferenceConflict => FailureCode::ReferenceConflict,
            Code::RequestIdReused => FailureCode::RequestIdReused,
            Code::InputInvalid => FailureCode::InputInvalid,
        };
        Self {
            code,
            message: diagnostic.message.clone(),
            detail: Some(Box::new(
                serde_json::to_value(diagnostic).expect("typed edit diagnostic"),
            )),
        }
    }
}

impl From<mo_presentation_template::TemplateError> for Failure {
    fn from(error: mo_presentation_template::TemplateError) -> Self {
        use mo_presentation_template::TemplateError as Error;
        let code = match &error {
            Error::Cancelled => FailureCode::Cancelled,
            Error::LimitExceeded(_) => FailureCode::LimitExceeded,
            Error::SourceConflict | Error::TemplateConflict { .. } => FailureCode::RevisionConflict,
            _ => FailureCode::InputInvalid,
        };
        if let Error::Edit(error) = error {
            return error.into();
        }
        Self {
            code,
            message: error.to_string(),
            detail: Some(Box::new(
                serde_json::to_value(error.diagnostic()).expect("typed template diagnostic"),
            )),
        }
    }
}
