use mo_common::{CellId, Digest, ObjectId, ParagraphId, RunId};
use mo_pptx::{
    ExportDefaults,
    source::{SourceObjectKind, SourceObjectRef, SurfaceKind, table::SourceCellAddress},
};
use mo_presentation_compile::{
    source_editor_page::{PagePickQuery, PageTextQuery, PageTextQueryResult},
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
    Pick {
        view: Digest,
        queries: Vec<PagePickQuery>,
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
    /// Actual model identities, never inferred from XML discovery ordinals.
    /// None for raw PPTX, legacy opaque text or an authored empty cell body.
    pub model: Option<EditorParagraphIdentity>,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EditorParagraphIdentity {
    pub id: ParagraphId,
    pub runs: Vec<EditorTextRunIdentity>,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EditorTextRunIdentity {
    pub id: RunId,
    /// Half-open offsets in the same displayed paragraph, in Unicode scalars.
    /// These are identities, not editing permission or grapheme boundaries.
    pub scalar_start: u32,
    pub scalar_end: u32,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EditorTextFrameInfo {
    pub frame: u32,
    pub object: SourceObjectRef,
    /// Stable model identity for author and retained document inputs.
    pub object_id: Option<ObjectId>,
    pub cell: Option<SourceCellAddress>,
    /// Authored table cell identity. Retained cells use their paragraph/run IDs.
    pub cell_id: Option<CellId>,
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
    /// Page paint targets and their group ancestors, in source paint preorder.
    /// Hit.object and parent refer to indices in this immutable view's list.
    pub objects: Vec<EditorPageObjectInfo>,
    pub downstream_coordinate_error_bound: mo_geometry::Fixed,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EditorPageObjectInfo {
    pub object: SourceObjectRef,
    pub object_id: Option<ObjectId>,
    pub name: String,
    pub kind: SourceObjectKind,
    pub surface: SurfaceKind,
    /// None is a top-level object; the source shape-tree root is not a user group.
    pub parent: Option<u32>,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EditorObjectHit {
    pub object: u32,
    pub kind: mo_raster::picking::DrawHitKind,
    pub text_frame: Option<u32>,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EditorPickResult {
    pub hits: Vec<EditorObjectHit>,
    pub truncated: bool,
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
    Picked {
        view: Digest,
        results: Vec<EditorPickResult>,
    },
    Cleared {
        view: Digest,
    },
    Error {
        error: Box<crate::PptxResourcePageFailure>,
    },
}
