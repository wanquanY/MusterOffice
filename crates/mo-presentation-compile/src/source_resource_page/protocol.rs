//! Narrow resource-page transport shared by adapters and computation entrypoints.
//! The diagnostic payload is owned by the entrypoint; transports preserve it
//! without depending on an aggregate API or interpreting domain failures.
use super::{ResourcePageRequest, SourceResourcePageRasterInfo};
use crate::source_page::SourcePageRequest;
use mo_pptx::source::images::ImageSourceSelection;
use mo_text::manifest::FontManifest;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MAX_REQUEST_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_SOURCE_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_FONT_BYTES: usize = 128 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PptxResourceDocumentRequest {
    pub profile: PptxResourcePageProfile,
    pub pages: Vec<ResourcePageRequest>,
    pub fonts: Option<FontManifest>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum PptxResourcePageProfile {
    #[serde(rename = "drawingml-resource-page-q32-v1-draft")]
    NativeResourcesDraftV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
#[schemars(rename = "PptxResourcePageRasterResponse")]
pub enum ResourcePageRasterResponse<E> {
    Rendered {
        info: Box<SourceResourcePageRasterInfo>,
    },
    Error {
        error: Box<E>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PptxResourcePageRequest {
    pub profile: PptxResourcePageProfile,
    pub page: SourcePageRequest,
    pub image_source: ImageSourceSelection,
    pub sampling: mo_raster::ImageSampling,
    /// None disables source text; a visible text body then requires resources.
    /// Fonts are explicit names/content ranges, never OS discovery or paths.
    pub fonts: Option<FontManifest>,
}
