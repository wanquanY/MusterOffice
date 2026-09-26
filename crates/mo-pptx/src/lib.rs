//! PresentationML format adapter. It serializes native objects, never a page raster.
mod definitions;
mod drawing;
pub mod source;
mod text;
pub mod timing;
mod write;
mod xml;

use mo_common::{Emu, ResourceId};
use mo_opc::{PackageLimits, ReaderAt};
use mo_presentation_model::{Rgba, ThemeColor, ValidationLimits, ValidationReport};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

pub use write::{export, export_to};

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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportDefaults {
    /// Explicit host choice. Export alone does not resolve, load or embed system fonts.
    pub font_family: String,
    pub text_size: Emu,
    pub text_color: Rgba,
    pub page_background: Rgba,
    /// All 12 native theme color slots are required for generated fallback definitions.
    pub theme_colors: BTreeMap<ThemeColor, Rgba>,
    pub font_delivery: FontDelivery,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum FontDelivery {
    /// Retains editable font-family references. It is not a font fidelity guarantee.
    ReferenceOnly,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PptxLimits {
    pub package: PackageLimits,
    pub document: ValidationLimits,
}

pub struct ResourceData<'a> {
    pub reader: &'a dyn ReaderAt,
    pub byte_length: u64,
}
pub trait Resources {
    fn open(&self, id: &ResourceId) -> Result<ResourceData<'_>, PptxError>;
}
pub struct NoResources;
impl Resources for NoResources {
    fn open(&self, id: &ResourceId) -> Result<ResourceData<'_>, PptxError> {
        Err(PptxError::ResourceRequired(id.clone()))
    }
}

pub(crate) const P: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";
pub(crate) const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
pub(crate) const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
pub(crate) const COLOR_SLOTS: [(ThemeColor, &str); 12] = [
    (ThemeColor::Dark1, "dk1"),
    (ThemeColor::Light1, "lt1"),
    (ThemeColor::Dark2, "dk2"),
    (ThemeColor::Light2, "lt2"),
    (ThemeColor::Accent1, "accent1"),
    (ThemeColor::Accent2, "accent2"),
    (ThemeColor::Accent3, "accent3"),
    (ThemeColor::Accent4, "accent4"),
    (ThemeColor::Accent5, "accent5"),
    (ThemeColor::Accent6, "accent6"),
    (ThemeColor::Hyperlink, "hlink"),
    (ThemeColor::FollowedHyperlink, "folHlink"),
];

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
