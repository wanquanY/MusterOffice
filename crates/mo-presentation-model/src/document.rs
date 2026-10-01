use crate::*;
use mo_common::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum ModelVersion {
    #[serde(rename = "musteroffice.presentation/0.1-draft")]
    V01,
}

/// Document declarations and immutable source provenance. Revisions, compilation
/// caches, clocks and decoder state live outside this model.
/// The example is a complete editable 16:9 slide with a text shape. Coordinates
/// and sizes are decimal EMU strings (12700 EMU per point). Empty theme, master,
/// layout, font and resource maps are valid; inherited text uses the caller's
/// explicit delivery defaults. Copy slides/objects with distinct IDs to expand
/// the deck. Inspect optional feature definitions only when those features are
/// needed; the example requires no system font discovery or external resources.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(example = crate::examples::document())]
pub struct Document {
    pub format: ModelVersion,
    pub id: DocumentId,
    pub title: String,
    pub page_size: Size,
    pub slide_order: Vec<SlideId>,
    pub slides: BTreeMap<SlideId, Slide>,
    pub objects: BTreeMap<ObjectId, Object>,
    pub themes: BTreeMap<ThemeId, Theme>,
    pub masters: BTreeMap<MasterId, Master>,
    pub layouts: BTreeMap<LayoutId, Layout>,
    pub fonts: BTreeMap<FontId, FontFace>,
    pub resources: BTreeMap<ResourceId, Resource>,
    /// Immutable native addresses/constraints. Known fields are edited in
    /// objects; this provenance never becomes a separate mutable document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_bindings: Option<SourceBindings>,
    /// Slide-owned animation graphs; empty storage preserves older author digests.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub timelines: BTreeMap<SlideId, mo_timeline::Timeline>,
}

impl Document {
    pub fn empty(id: DocumentId, page_size: Size) -> Self {
        Self {
            format: ModelVersion::V01,
            id,
            title: String::new(),
            page_size,
            slide_order: Vec::new(),
            slides: BTreeMap::new(),
            objects: BTreeMap::new(),
            themes: BTreeMap::new(),
            masters: BTreeMap::new(),
            layouts: BTreeMap::new(),
            fonts: BTreeMap::new(),
            resources: BTreeMap::new(),
            source_bindings: None,
            timelines: BTreeMap::new(),
        }
    }
    pub fn semantic_digest(&self) -> Result<Digest, CanonicalError> {
        digest("musteroffice.presentation/0.1-draft", self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Slide {
    pub id: SlideId,
    pub name: String,
    pub layout: Option<LayoutId>,
    pub objects: Vec<ObjectId>,
    pub background: Inherited<Fill>,
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Theme {
    pub id: ThemeId,
    pub name: String,
    pub colors: BTreeMap<ThemeColor, Rgba>,
    pub default_text: CharacterStyle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Master {
    pub id: MasterId,
    pub theme: ThemeId,
    pub objects: Vec<ObjectId>,
    pub background: Inherited<Fill>,
    pub default_text: CharacterStyle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Layout {
    pub id: LayoutId,
    pub master: MasterId,
    pub name: String,
    pub objects: Vec<ObjectId>,
    pub background: Inherited<Fill>,
    pub default_text: CharacterStyle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    content = "id",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum ContainerId {
    Slide(SlideId),
    Master(MasterId),
    Layout(LayoutId),
    Group(ObjectId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Object {
    pub id: ObjectId,
    pub parent: ContainerId,
    /// Missing only for retained native coordinates that cannot be represented
    /// as a complete direct declaration. Never substitute a resolved identity.
    pub transform: Option<Transform>,
    pub appearance: Appearance,
    pub accessibility: Accessibility,
    pub content: ObjectContent,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Accessibility {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub decorative: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ObjectContent {
    Table {
        table: Table,
    },
    /// Original geometry, styles, relationships and unknown extension semantics
    /// remain in the bound source. This is not a generic path/text promotion.
    RetainedSource {
        native_kind: RetainedObjectKind,
        children: Vec<ObjectId>,
        paragraphs: Vec<RetainedParagraph>,
    },
    Shape {
        geometry: Geometry,
        text: Option<TextBody>,
    },
    Picture {
        resource: ResourceId,
        crop: Crop,
    },
    Group {
        children: Vec<ObjectId>,
        viewport: Size,
    },
    Connector {
        start: ConnectorEndpoint,
        end: ConnectorEndpoint,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Crop {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FontFace {
    pub id: FontId,
    pub resource: ResourceId,
    pub face_index: u32,
    pub family: String,
    pub weight: u16,
    pub italic: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ResourceKind {
    Font,
    Picture,
    Audio,
    Video,
    SourcePackage,
    EmbeddedWorkbook,
    Model3d,
}

/// Opaque authorized handle and declared content identity; byte verification belongs to the host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Resource {
    pub id: ResourceId,
    pub kind: ResourceKind,
    pub sha256: Digest,
    pub media_type: String,
}
