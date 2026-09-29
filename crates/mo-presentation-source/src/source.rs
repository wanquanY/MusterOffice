//! Source-bound projection of native PresentationML. This is not a resolved
//! layout/model importer: opaque content stays in the immutable OPC source.
pub mod color;
mod color_mapping;
mod compatibility;
pub mod document;
pub mod drawingml;
mod edit;
pub mod effects;
pub mod fill;
pub mod geometry;
pub mod images;
mod inheritance;
pub mod line;
mod links;
mod paint;
pub mod prepared;
mod presentation;
mod preserve;
mod surface;
pub mod table;
pub mod text;
pub mod theme;
mod transform;
mod transform_edit;
mod visual;
pub use color_mapping::{SourceColorMap, SourceColorMapRef, SourceColorMapping};
pub use compatibility::{
    SourceCompatibility, SourceCompatibilityBranch, SourceCompatibilitySelection,
};
pub use inheritance::{
    PlaceholderKind, PlaceholderMatchRule, PlaceholderOrientation, PlaceholderSize,
    SourceObjectRef, SourceObjectResolution, SourcePlaceholder, SourcePlaceholderMatch,
    SourceResolvedValue,
};
pub use links::{SourcePartRef, SourceSurfaceLinks, SourceThemeStack};
pub use visual::{SourceVisualIssue, SourceVisualIssueKind};

pub use edit::{SourceTextEdit, SourceTextEdits, edit_source_text};
use mo_common::{ByteLength, Digest, Emu};
use mo_opc::{Package, PackageLimits, PackageRead, ReaderAt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub use transform_edit::{
    SourceTransformEdit, SourceTransformEdits, SourceTransformValues, edit_source_transforms,
};

#[derive(Debug, Clone, Copy)]
pub struct SourceLimits {
    pub package: PackageLimits,
    pub max_surfaces: usize,
    pub max_objects: usize,
    pub max_text_bytes: usize,
    pub max_text_style_elements: usize,
    pub max_text_style_attribute_bytes: usize,
    pub max_table_cells: usize,
    pub max_table_styles: usize,
    pub max_table_elements: usize,
    pub max_table_attribute_bytes: usize,
    pub max_edits: usize,
    pub max_theme_parts: usize,
    pub max_theme_elements: usize,
    pub max_theme_attribute_bytes: usize,
    pub max_paint_elements: usize,
    pub max_paint_attribute_bytes: usize,
    pub max_line_elements: usize,
    pub max_line_attribute_bytes: usize,
    pub max_geometry_elements: usize,
    pub max_geometry_attribute_bytes: usize,
}
impl Default for SourceLimits {
    fn default() -> Self {
        Self {
            package: PackageLimits::default(),
            max_surfaces: 10_000,
            max_objects: 100_000,
            max_text_bytes: 64 * 1024 * 1024,
            max_text_style_elements: 1_000_000,
            max_text_style_attribute_bytes: 32 * 1024 * 1024,
            max_table_cells: 1_000_000,
            max_table_styles: 4096,
            max_table_elements: 2_000_000,
            max_table_attribute_bytes: 32 * 1024 * 1024,
            max_edits: 10_000,
            max_theme_parts: 4_096,
            max_theme_elements: 1_000_000,
            max_theme_attribute_bytes: 32 * 1024 * 1024,
            max_paint_elements: 1_000_000,
            max_paint_attribute_bytes: 32 * 1024 * 1024,
            max_line_elements: 1_000_000,
            max_line_attribute_bytes: 32 * 1024 * 1024,
            max_geometry_elements: 1_000_000,
            max_geometry_attribute_bytes: 32 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceIndex {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_styles: Option<Box<table::styles::SourceTableStylePart>>,
    #[serde(default, skip_serializing_if = "text::SourceTextCatalog::is_empty")]
    pub text: text::SourceTextCatalog,
    pub compatibility_profile: String,
    pub main_compatibility: SourceCompatibility,
    pub source_sha256: Digest,
    pub byte_length: ByteLength,
    pub main_part: String,
    pub main_content_type: String,
    pub contains_signatures: bool,
    /// Explicit native dimensions, absent if the source omits sldSz.
    pub page_size: Option<mo_presentation_model::Size>,
    pub slides: Vec<SourceSlide>,
    pub surfaces: BTreeMap<String, SourceSurface>,
    pub themes: BTreeMap<String, theme::SourceThemePart>,
    /// Unresolved constructs are reported, never counted as semantic support.
    pub notices: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceSlide {
    pub native_id: u32,
    pub part: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SurfaceKind {
    Slide,
    Master,
    Layout,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceSurface {
    #[serde(default, skip_serializing_if = "text::SourceTextCatalog::is_empty")]
    pub text: text::SourceTextCatalog,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_master_shapes: Option<bool>,
    /// Visual declarations not yet projected by a semantic source reader.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub visual_issues: Vec<SourceVisualIssue>,
    pub color_mapping: Option<SourceColorMapping>,
    /// Closest explicit map in the owning slide/layout/master hierarchy.
    pub resolved_color_mapping: Option<SourceColorMapRef>,
    pub links: SourceSurfaceLinks,
    pub effective_theme: SourceThemeStack,
    pub theme_selection: theme::SourceThemeSelection,
    pub compatibility: SourceCompatibility,
    pub kind: SurfaceKind,
    pub sha256: Digest,
    pub name: Option<String>,
    pub hidden: bool,
    pub root_object_id: u32,
    /// Shape-tree declaration; preserved independently from child transforms.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_group_transform: Option<SourceTransform>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<fill::SourceBackground>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_group_fill: Option<fill::SourceFill>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_group_effects: Option<effects::SourceEffectProperties>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub effect_nodes: BTreeMap<u32, effects::SourceEffectNode>,
    /// Preorder paint-tree objects; IDs are part-scoped, not domain/global IDs.
    pub objects: Vec<SourceObject>,
    /// These block text mutation until reference/compatibility semantics exist.
    pub text_edit_barriers: Vec<String>,
    pub notices: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SourceObjectKind {
    Shape,
    Picture,
    Group,
    Connector,
    GraphicFrame,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceObject {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<table::SourceTable>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_body_ordinal: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub visual_issues: Vec<SourceVisualIssue>,
    pub native_id: u32,
    pub name: String,
    pub kind: SourceObjectKind,
    pub parent_group: Option<u32>,
    pub placeholder: Option<SourcePlaceholder>,
    pub transform: Option<SourceTransform>,
    /// Direct declaration only. Absence is distinct from an empty a:ln.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<line::SourceLine>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_reference: Option<line::SourceLineReference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<geometry::SourceGeometry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<fill::SourceFill>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_reference: Option<fill::SourceFillReference>,
    /// Picture image content is independent of the shape properties fill.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub picture_fill: Option<fill::SourceFill>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_background_fill: Option<fill::SourceBackgroundFillUsage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effects: Option<effects::SourceEffectProperties>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect_reference: Option<effects::SourceEffectReference>,
    pub resolution: SourceObjectResolution,
    pub paragraphs: Vec<Vec<SourceRun>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTransform {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retained_ordinals: Vec<u32>,
    pub origin: Option<mo_presentation_model::Point>,
    pub size: Option<mo_presentation_model::Size>,
    pub child_origin: Option<mo_presentation_model::Point>,
    pub child_size: Option<mo_presentation_model::Size>,
    /// Explicit native value, not normalized or resolved through groups.
    pub rotation: Option<i32>,
    pub flip_horizontal: Option<bool>,
    pub flip_vertical: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SourceRunKind {
    Text,
    Break,
    Field,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SourceTextConstraint {
    CompatibilityBranch,
    StructuredLeaf,
    DynamicField,
    TimingReferences,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceRun {
    pub kind: SourceRunKind,
    pub text: String,
    /// Only ordinary run text with a concrete a:t source binding is editable.
    pub editable: bool,
    pub edit_constraint: Option<SourceTextConstraint>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTextTarget {
    pub part: String,
    pub object_id: u32,
    pub paragraph: u32,
    pub run: u32,
}

pub(crate) struct BoundIndex {
    index: SourceIndex,
    object_positions: BTreeMap<String, BTreeMap<u32, usize>>,
    bindings: BTreeMap<SourceTextTarget, usize>,
    transforms: BTreeMap<String, BTreeMap<u32, transform_edit::Binding>>,
}

pub fn inspect_source(
    source: &dyn PackageRead,
    limits: SourceLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceIndex, crate::PptxError> {
    Ok(presentation::read(source, limits, check)?.index)
}

pub(crate) fn resolve_projection(
    index: &mut SourceIndex,
    check: &dyn Fn() -> bool,
) -> Result<(), crate::PptxError> {
    inheritance::resolve(&mut index.surfaces, check)?;
    theme::select(&mut index.surfaces, &index.themes, check)
}

fn malformed(message: impl Into<String>) -> mo_xml::XmlError {
    mo_xml::XmlError::Malformed(message.into())
}
fn integer<T: std::str::FromStr>(value: Option<&str>, field: &str) -> Result<T, mo_xml::XmlError> {
    value
        .ok_or_else(|| malformed(format!("missing {field}")))?
        .trim()
        .parse()
        .map_err(|_| malformed(format!("invalid {field}")))
}
fn boolean(value: &str) -> Result<bool, mo_xml::XmlError> {
    match value.trim() {
        "1" | "true" => Ok(true),
        "0" | "false" => Ok(false),
        _ => Err(malformed("invalid native boolean")),
    }
}
fn coordinate(value: Option<&str>, field: &str) -> Result<Emu, mo_xml::XmlError> {
    Ok(Emu::new(integer(value, field)?))
}
