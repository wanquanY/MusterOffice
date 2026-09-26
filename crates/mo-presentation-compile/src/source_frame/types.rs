use crate::source_text::{SourceParagraphPlan, SourceTextIssue};
use mo_common::Digest;
use mo_geometry::{Fixed, Point, Rect};
use mo_presentation_source::source::{
    SourceObjectRef, SourceResolvedValue,
    geometry::evaluate::{GeometryOrigin, GeometryUnresolved},
    text::{NativeTextAlign, body::*, cascade::*},
};
use mo_text::{
    flow::{LineWidths, OverflowPolicy},
    geometry::LineSpacing,
    manifest::ManifestGeometryPaths,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceFrameRequest {
    pub expected_source_sha256: Digest,
    pub object: SourceObjectRef,
    pub bounds_tolerance: Fixed,
}
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SourceFrameIssue {
    Body {
        reason: TextBodyUnresolved,
    },
    Text {
        reason: SourceTextIssue,
    },
    Geometry {
        reason: GeometryUnresolved,
    },
    BodyProperty {
        property: TextBodyProperty,
    },
    Autofit,
    ParagraphProperty {
        paragraph: u32,
        property: ParagraphProperty,
    },
    ParagraphDeclaration {
        paragraph: u32,
        declaration: TextStyleDeclaration,
    },
    InvalidRegion,
    IncompleteParagraph {
        paragraph: u32,
        flow: Vec<mo_text::flow::FlowIssue>,
        geometry: Vec<mo_text::geometry::GeometryIssue>,
        paths: Vec<mo_text::scene::PathSceneIssue>,
    },
}
#[derive(Debug, thiserror::Error)]
pub enum SourceFrameError {
    #[error("native frame mapping required: {0:?}")]
    Mapping(Box<SourceFrameIssue>),
    #[error(transparent)]
    Source(#[from] mo_presentation_source::PptxError),
    #[error(transparent)]
    SourceText(#[from] crate::source_text::SourceTextError),
    #[error(transparent)]
    Text(#[from] mo_text::TextError),
    #[error(transparent)]
    Geometry(#[from] mo_geometry::GeometryError),
    #[error(transparent)]
    Coordinate(#[from] crate::CompileError),
    #[error("native text frame limit exceeded: {0}")]
    Limit(&'static str),
    #[error("native text frame cancelled")]
    Cancelled,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TextRectangleSource {
    Declaration {
        origin: GeometryOrigin,
    },
    ShapeBounds {
        extent: SourceResolvedValue<mo_presentation_model::Size>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceFrameRegion {
    pub source: TextRectangleSource,
    /// Shape-local Q32 text rectangle after native insets, before world placement.
    pub inner: Rect,
    /// Conversion from evaluated binary64 geometry/exact lexical insets to Q32.
    /// Does not certify native formula evaluation or a final raster coordinate.
    pub conversion_error_bound: Fixed,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrameParagraphSpec {
    pub source_ordinal: u32,
    pub widths: LineWidths,
    pub left: Fixed,
    pub indent: Fixed,
    pub indent_from_right: bool,
    pub spacing: LineSpacing,
    pub before: ParagraphSpacing,
    pub after: ParagraphSpacing,
    /// Native percentage conversion uncertainty, rounded up to raw Q32 units.
    pub spacing_conversion_error: Fixed,
    pub alignment: NativeTextAlign,
    pub overflow: OverflowPolicy,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum ParagraphSpacing {
    Fixed(Fixed),
    /// Evaluated against the first (before) or last (after) logical line.
    StyleMaximum {
        heights: Vec<Fixed>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrameParagraph {
    pub spec: FrameParagraphSpec,
    pub computed: ManifestGeometryPaths,
    /// One translation per precise line, in the shape-local coordinate system.
    pub line_offsets: Vec<Point>,
    /// Effective spacing after first/last paragraph suppression, before anchor.
    pub applied_before: Fixed,
    pub applied_after: Fixed,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrameGlyph {
    pub paragraph: u32,
    /// Index into this paragraph's original ParagraphPathScene.glyphs.
    pub glyph: u32,
    pub origin: Point,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrameWork {
    pub component_calls: u32,
    pub font_upload_bytes: u64,
    pub request_words: u64,
    pub glyphs: u32,
    pub path_commands: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceFramePlan {
    pub profile: String,
    pub text: CascadedText,
    pub inputs: Vec<SourceParagraphPlan>,
    pub body: EffectiveTextBody,
    pub region: SourceFrameRegion,
    pub paragraphs: Vec<FrameParagraph>,
    pub content_height: Fixed,
    pub glyphs: Vec<FrameGlyph>,
    pub bounds: Option<Rect>,
    /// Maximum Q32 half-rounding from horizontal/vertical alignment offsets.
    pub alignment_rounding_bound: Fixed,
    pub work: FrameWork,
}
#[derive(Debug, Clone, Copy)]
pub struct SourceFrameLimits {
    pub max_paragraphs: usize,
    pub max_component_calls: u32,
    pub max_font_upload_bytes: u64,
    pub max_request_words: u64,
    pub max_glyphs: u32,
    pub max_path_commands: u32,
}
impl Default for SourceFrameLimits {
    fn default() -> Self {
        Self {
            max_paragraphs: 128,
            max_component_calls: 4096,
            max_font_upload_bytes: 1024 * 1024 * 1024,
            max_request_words: 16_000_000,
            max_glyphs: 262144,
            max_path_commands: 524288,
        }
    }
}
