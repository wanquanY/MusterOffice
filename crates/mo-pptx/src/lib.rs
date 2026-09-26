//! PresentationML format adapter. It serializes native objects, never a page raster.
mod definitions;
mod drawing;
pub use mo_presentation_source::source;
mod native;
pub mod timing;
mod write;
mod xml;

use mo_opc::PackageLimits;
use mo_presentation_model::ValidationLimits;

pub use write::{export, export_plan_to, export_to};

pub use mo_presentation_source::PptxError;

pub use mo_presentation_source::author::{AuthorPlan, ExportDefaults, FontDelivery};

#[derive(Debug, Clone, Copy, Default)]
pub struct PptxLimits {
    pub package: PackageLimits,
    pub document: ValidationLimits,
}

pub use mo_presentation_source::author::{NoResources, ResourceData, Resources};

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
