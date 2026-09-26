//! Native presentation declarations and source bindings, with pure format
//! reading/resolution. This crate has no writer, renderer, host or publication
//! dependency; both author lowering and source admission target its semantics.
pub mod source;
pub mod timing;
use mo_common::ResourceId;
use mo_presentation_model::ValidationReport;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum PptxError {
    #[error("PPTX source conflict: {0}")]
    SourceConflict(String),
    #[error("invalid document: {0:?}")]
    InvalidDocument(ValidationReport),
    #[error("PPTX value at {path}: {message}")]
    Value { path: String, message: String },
    #[error("PPTX mapping not implemented: {0}")]
    Unsupported(String),
    #[error("authorized resource required: {0}")]
    ResourceRequired(ResourceId),
    #[error("PPTX limit exceeded: {0}")]
    Limit(&'static str),
    #[error("PPTX computation cancelled")]
    Cancelled,
    #[error(transparent)]
    Opc(#[from] mo_opc::OpcError),
    #[error(transparent)]
    Xml(#[from] mo_xml::XmlError),
}

pub(crate) const P: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";
pub(crate) const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
pub(crate) const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
pub(crate) fn value(path: impl Into<String>, message: impl Into<String>) -> PptxError {
    PptxError::Value {
        path: path.into(),
        message: message.into(),
    }
}
pub(crate) fn cancelled(check: &dyn Fn() -> bool) -> Result<(), PptxError> {
    if check() {
        Err(PptxError::Cancelled)
    } else {
        Ok(())
    }
}
