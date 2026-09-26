use mo_common::from_json_str;
use mo_opc::{OpcError, Package, ReaderAt};
use mo_pptx::{
    PptxError,
    source::{SourceIndex, SourceLimits, SourceTextEdits, SourceTransformEdits},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PptxSourceResponse {
    Inspected { index: Box<SourceIndex> },
    Error { error: PptxFailure },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PptxFailure {
    pub code: PptxFailureCode,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PptxFailureCode {
    InputInvalid,
    SourceConflict,
    PreservationConflict,
    MappingNotImplemented,
    ResourceRequired,
    LimitExceeded,
    Cancelled,
    ReadFailed,
}
impl std::fmt::Display for PptxFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = serde_json::to_value(self.code).expect("error code is serializable");
        write!(
            f,
            "{}: {}",
            code.as_str().expect("code is a string"),
            self.message
        )
    }
}
pub(crate) fn pptx_failure(error: PptxError) -> PptxFailure {
    use mo_xml::XmlError;
    let code = match &error {
        PptxError::Cancelled
        | PptxError::Xml(XmlError::Cancelled)
        | PptxError::Opc(OpcError::Cancelled)
        | PptxError::Opc(OpcError::Xml {
            source: XmlError::Cancelled,
            ..
        }) => PptxFailureCode::Cancelled,
        PptxError::Limit(_)
        | PptxError::Xml(XmlError::Limit(_))
        | PptxError::Opc(OpcError::Limit(_))
        | PptxError::Opc(OpcError::Xml {
            source: XmlError::Limit(_),
            ..
        }) => PptxFailureCode::LimitExceeded,
        PptxError::SourceConflict(_) | PptxError::Xml(XmlError::EditConflict(_)) => {
            PptxFailureCode::SourceConflict
        }
        PptxError::Opc(OpcError::Preservation(_)) => PptxFailureCode::PreservationConflict,
        PptxError::Unsupported(_)
        | PptxError::Opc(OpcError::Unsupported(_))
        | PptxError::Xml(XmlError::Compatibility(_)) => PptxFailureCode::MappingNotImplemented,
        PptxError::ResourceRequired(_) => PptxFailureCode::ResourceRequired,
        PptxError::Opc(OpcError::Io(e))
            if !matches!(
                e.kind(),
                std::io::ErrorKind::UnexpectedEof | std::io::ErrorKind::InvalidData
            ) =>
        {
            PptxFailureCode::ReadFailed
        }
        _ => PptxFailureCode::InputInvalid,
    };
    PptxFailure {
        code,
        message: error.to_string(),
    }
}

pub fn inspect_pptx<R: ReaderAt>(
    reader: R,
    length: u64,
    limits: SourceLimits,
    check: &dyn Fn() -> bool,
) -> PptxSourceResponse {
    let result = Package::open(reader, length, limits.package, check)
        .map_err(PptxError::from)
        .and_then(|package| mo_pptx::source::inspect_source(&package, limits, check));
    match result {
        Ok(index) => PptxSourceResponse::Inspected {
            index: Box::new(index),
        },
        Err(error) => PptxSourceResponse::Error {
            error: pptx_failure(error),
        },
    }
}

pub fn pptx_source_response_json(response: &PptxSourceResponse) -> String {
    serde_json::to_string(response).expect("typed source index is serializable")
}

pub fn inspect_pptx_json(bytes: &[u8]) -> String {
    let limits = inline_limits();
    pptx_source_response_json(&inspect_pptx(bytes, bytes.len() as u64, limits, &|| false))
}

pub fn edit_pptx_text_json(input: &str, source: &[u8]) -> Result<Vec<u8>, String> {
    if input.len() > super::MAX_REQUEST_BYTES {
        return Err("LIMIT_EXCEEDED: source edit request bytes".into());
    }
    let request =
        from_json_str::<SourceTextEdits>(input).map_err(|e| format!("INPUT_INVALID: {e}"))?;
    let limits = inline_limits();
    let package = Package::open(source, source.len() as u64, limits.package, &|| false)
        .map_err(|e| pptx_failure(e.into()).to_string())?;
    mo_pptx::source::edit_source_text(&package, &request, limits, &|| false)
        .map_err(|e| pptx_failure(e).to_string())
}

pub(crate) fn inline_limits() -> SourceLimits {
    let mut limits = SourceLimits::default();
    limits.package.max_package_bytes = super::MAX_INLINE_RESOURCE_BYTES as u64;
    limits
}

pub fn edit_pptx_transforms_json(input: &str, source: &[u8]) -> Result<Vec<u8>, String> {
    if input.len() > super::MAX_REQUEST_BYTES {
        return Err("LIMIT_EXCEEDED: source edit request bytes".into());
    }
    let request =
        from_json_str::<SourceTransformEdits>(input).map_err(|e| format!("INPUT_INVALID: {e}"))?;
    let limits = inline_limits();
    let package = Package::open(source, source.len() as u64, limits.package, &|| false)
        .map_err(|e| pptx_failure(e.into()).to_string())?;
    mo_pptx::source::edit_source_transforms(&package, &request, limits, &|| false)
        .map_err(|e| pptx_failure(e).to_string())
}
