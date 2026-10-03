use mo_common::{Digest, ObjectId};
use mo_pptx::{
    ExportDefaults,
    source::{SourceObjectRef, table::SourceCellAddress},
};
use mo_presentation_compile::{
    source_editor_page::{PageTextQuery, PageTextQueryResult},
    source_frame::FrameWork,
    source_page::SourcePageInfo,
    source_resource_page::{ResourcePageRequest, protocol::AuthorResourceRange},
};
use mo_presentation_model::Document;
use mo_text::manifest::FontManifest;
use mo_unicode::TextBoundary;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum EditorPageInput {
    /// Unmodified inspected OPC bytes in the separate material channel.
    Pptx {},
    /// Direct semantic author projection; no temporary PPTX export/import.
    Author {
        document: Box<Document>,
        defaults: ExportDefaults,
        resources: Vec<AuthorResourceRange>,
    },
    /// Validated field overlay over the original OPC material bytes.
    Retained { document: Box<Document> },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditorPagePreparation {
    pub input: EditorPageInput,
    pub page: ResourcePageRequest,
    /// Explicit even for an empty page. An empty manifest needs no font bytes.
    pub fonts: FontManifest,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "operation", rename_all = "camelCase", deny_unknown_fields)]
pub enum EditorPageRequest {
    Prepare {
        request: Box<EditorPagePreparation>,
    },
    Query {
        view: Digest,
        queries: Vec<PageTextQuery>,
    },
    Clear {
        view: Digest,
    },
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EditorParagraphInfo {
    pub text: String,
    pub source_ordinal: u32,
    pub boundaries: Vec<TextBoundary>,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EditorTextFrameInfo {
    pub frame: u32,
    pub object: SourceObjectRef,
    /// Stable model identity for author and retained document inputs.
    pub object_id: Option<ObjectId>,
    pub cell: Option<SourceCellAddress>,
    pub paragraphs: Vec<EditorParagraphInfo>,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EditorPageInfo {
    pub page: SourcePageInfo,
    pub viewport: mo_raster::RasterViewport,
    pub resources_sha256: Digest,
    pub text_work: FrameWork,
    pub text_frames: Vec<EditorTextFrameInfo>,
    pub downstream_coordinate_error_bound: mo_geometry::Fixed,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum EditorPageResponse {
    Prepared {
        view: Digest,
        info: Box<EditorPageInfo>,
    },
    Queried {
        view: Digest,
        results: Vec<PageTextQueryResult>,
    },
    Cleared {
        view: Digest,
    },
    Error {
        error: Box<crate::PptxResourcePageFailure>,
    },
}
