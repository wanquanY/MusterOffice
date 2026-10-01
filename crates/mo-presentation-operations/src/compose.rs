//! One-way authoring convenience: ordered pages and text/shape declarations
//! expand into the existing Document. No alternate rendering or storage model.
pub(crate) mod append;
pub mod authoring;
mod lower;
#[cfg(test)]
mod tests;

use mo_common::{DocumentId, Emu, ObjectId, SlideId};
use mo_presentation_model::{
    Accessibility, Alignment, Color, Fill, Geometry, Insets, Stroke, TextDirection,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Compact creation input for native editable text, shapes and pictures. Coordinates and
/// font sizes are decimal EMU strings (12700 EMU per point). This is expanded
/// once into Document; all subsequent editing, rendering and export use that
/// same document. Use `create` with Document for other object kinds.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[schemars(example = example())]
pub struct PresentationContent {
    pub id: DocumentId,
    pub title: String,
    pub page_size: mo_presentation_model::Size,
    /// Array order is slide order. IDs must be unique across this presentation.
    pub slides: Vec<SlideContent>,
    /// Source identities only. The host supplies separately authorized bytes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resources: Vec<mo_presentation_model::Resource>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SlideContent {
    pub id: SlideId,
    #[serde(default)]
    pub name: String,
    /// Omitted background inherits the same caller delivery defaults as Document.
    #[serde(default)]
    pub background: Option<Fill>,
    /// Array order is paint order, back to front.
    pub elements: Vec<ElementContent>,
}

/// Ordered native shapes and pictures; legacy shape declarations remain valid.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum ElementContent {
    Picture(PictureContent),
    Shape(Box<ShapeContent>),
}
// Dispatch on the explicit picture field instead of losing nested diagnostics
// through an untagged enum. Wire shapes remain backwards compatible.
impl<'de> Deserialize<'de> for ElementContent {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(decoder)?;
        let text = serde_json::to_string(&value).map_err(serde::de::Error::custom)?;
        let decode = |error: mo_common::JsonDecodeError| {
            serde::de::Error::custom(format!("{}: {}", error.path, error.message))
        };
        if value.get("picture").is_some() {
            mo_common::from_json_str_with_path(&text)
                .map(Self::Picture)
                .map_err(decode)
        } else {
            mo_common::from_json_str_with_path(&text)
                .map(Self::Shape)
                .map_err(decode)
        }
    }
}
impl ElementContent {
    pub fn id(&self) -> &ObjectId {
        match self {
            Self::Shape(s) => &s.id,
            Self::Picture(p) => &p.id,
        }
    }
    pub fn frame(&self) -> ShapeFrame {
        match self {
            Self::Shape(s) => s.frame,
            Self::Picture(p) => p.frame,
        }
    }
    pub fn accessibility(&self) -> &Accessibility {
        match self {
            Self::Shape(s) => &s.accessibility,
            Self::Picture(p) => &p.accessibility,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PictureContent {
    pub id: ObjectId,
    pub frame: ShapeFrame,
    pub picture: PictureSource,
    #[serde(default)]
    pub accessibility: Accessibility,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PictureSource {
    pub resource: mo_common::ResourceId,
    /// Native crop fractions: 100000 is the full source extent.
    #[serde(default)]
    pub crop: mo_presentation_model::Crop,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShapeContent {
    pub id: ObjectId,
    pub frame: ShapeFrame,
    /// Omitted geometry is a rectangle; text remains native editable text.
    #[serde(default = "rectangle")]
    pub geometry: Geometry,
    /// Omitted fill/stroke mean explicit none, not implicit theme paint.
    #[serde(default)]
    pub fill: Option<Fill>,
    #[serde(default)]
    pub stroke: Option<Stroke>,
    #[serde(default)]
    pub text: Option<PlainText>,
    #[serde(default)]
    pub accessibility: Accessibility,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShapeFrame {
    pub x: Emu,
    pub y: Emu,
    pub width: Emu,
    pub height: Emu,
    /// DrawingML units: 60000 per degree, just as the native model.
    #[serde(default)]
    pub rotation: i32,
    #[serde(default)]
    pub flip_horizontal: bool,
    #[serde(default)]
    pub flip_vertical: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlainText {
    /// LF, CRLF and CR each separate paragraphs; tabs become native tab runs.
    /// Empty paragraphs and a trailing paragraph separator are preserved.
    pub text: String,
    /// Omitted properties inherit explicit delivery defaults. Font selection
    /// uses the host's explicit default; use full Document creation for custom
    /// font resources. This operation never searches installed system fonts.
    #[serde(default)]
    pub style: PlainTextStyle,
    #[serde(default = "zero_insets")]
    pub insets: Insets,
    #[serde(default = "yes")]
    pub wrap: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlainTextStyle {
    #[serde(default)]
    pub size: Option<Emu>,
    #[serde(default)]
    pub color: Option<Color>,
    #[serde(default)]
    pub bold: Option<bool>,
    #[serde(default)]
    pub italic: Option<bool>,
    #[serde(default)]
    pub underline: Option<bool>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub alignment: Option<Alignment>,
    #[serde(default)]
    pub direction: Option<TextDirection>,
    #[serde(default)]
    pub space_before: Option<Emu>,
    #[serde(default)]
    pub space_after: Option<Emu>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_spacing: Option<mo_presentation_model::ParagraphLineSpacing>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left_margin: Option<Emu>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right_margin: Option<Emu>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indent: Option<Emu>,
}

fn rectangle() -> Geometry {
    Geometry::Rectangle
}
fn yes() -> bool {
    true
}
fn zero_insets() -> Insets {
    Insets {
        left: Emu::new(0),
        top: Emu::new(0),
        right: Emu::new(0),
        bottom: Emu::new(0),
    }
}

pub(crate) fn example() -> serde_json::Value {
    serde_json::json!({
        "id":"document:example", "title":"Editable presentation",
        "pageSize":{"width":"12192000","height":"6858000"},
        "slides":[{"id":"slide:1","name":"Introduction","elements":[{
            "id":"object:title",
            "frame":{"x":"914400","y":"914400","width":"10363200","height":"914400"},
            "text":{"text":"Your title","style":{"size":"457200","bold":true}}
        }]}]
    })
}
