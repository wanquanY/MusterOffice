use crate::*;
use mo_presentation_compile::{
    source_image_layout::ImageLayoutError, source_image_paint::ImagePaintError,
    source_page::SourcePageError,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "stage",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum PptxResourcePageFailure {
    Request {
        error: PptxPageFailure,
    },
    Source {
        error: PptxPageFailure,
    },
    Fonts {
        error: ShapeFailure,
    },
    Page {
        error: PptxPageFailure,
        #[serde(skip_serializing_if = "Option::is_none")]
        paint_location: Option<mo_pptx::source::text::paint::TextPaintLocation>,
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<Box<PptxTextPageIssue>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        image: Option<Box<PptxResourceImageIssue>>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum PptxResourceImageIssue {
    Resource {
        result: mo_pptx::source::images::SourceImageResult,
    },
    StationaryOrientation {},
    PhysicalSize {
        resolution: mo_image::PhysicalPixelSize,
    },
}
impl From<SourcePageError> for PptxResourcePageFailure {
    fn from(error: SourcePageError) -> Self {
        page_failure(error)
    }
}
pub(crate) fn page_failure(e: SourcePageError) -> PptxResourcePageFailure {
    let mut leaf = &e;
    while let SourcePageError::AtObject { error, .. } = leaf {
        leaf = error;
    }
    let image = match leaf {
        SourcePageError::ImageResource(result) => Some(PptxResourceImageIssue::Resource {
            result: (**result).clone(),
        }),
        SourcePageError::ImagePaint(ImagePaintError::OrientationRequired) => {
            Some(PptxResourceImageIssue::StationaryOrientation {})
        }
        SourcePageError::ImageLayout(ImageLayoutError::PhysicalSize(resolution)) => {
            Some(PptxResourceImageIssue::PhysicalSize {
                resolution: *resolution,
            })
        }
        _ => None,
    };
    let PptxTextPageFailure::Page {
        error,
        paint_location,
        detail,
    } = crate::pptx_text_page::page_failure(e)
    else {
        unreachable!("common page diagnostics")
    };
    PptxResourcePageFailure::Page {
        error,
        paint_location,
        text: detail,
        image: image.map(Box::new),
    }
}
