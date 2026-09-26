use mo_common::Emu;
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct NativeShapeType(u8);
impl NativeShapeType {
    pub fn name(&self) -> &'static str {
        super::names::NAMES[usize::from(self.0)]
    }
    pub(super) fn index(self) -> usize {
        usize::from(self.0)
    }
}
impl TryFrom<String> for NativeShapeType {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        super::names::NAMES
            .binary_search(&value.as_str())
            .map(|index| Self(index as u8))
            .map_err(|_| "invalid native shape type".into())
    }
}
impl From<NativeShapeType> for String {
    fn from(v: NativeShapeType) -> Self {
        v.name().into()
    }
}
impl JsonSchema for NativeShapeType {
    fn schema_name() -> Cow<'static, str> {
        "NativeShapeType".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({"type":"string","enum":super::names::NAMES})
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceGeometry {
    pub source_ordinal: u32,
    pub definition: SourceGeometryDefinition,
    /// Unknown properties retain physical bindings and block complete evaluation.
    pub retained_ordinals: Vec<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SourceGeometryDefinition {
    Preset {
        preset: NativeShapeType,
        adjustments: Option<SourceGeometryList<SourceGuide>>,
    },
    Custom(Box<SourceCustomGeometry>),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceCustomGeometry {
    pub adjustments: Option<SourceGeometryList<SourceGuide>>,
    pub guides: Option<SourceGeometryList<SourceGuide>>,
    pub handles: Option<SourceGeometryList<SourceAdjustHandle>>,
    pub connections: Option<SourceGeometryList<SourceConnectionSite>>,
    pub text_rect: Option<SourceGeometryRect>,
    pub paths: SourceGeometryList<SourceGeometryPath>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceGeometryList<T> {
    pub source_ordinal: u32,
    /// Native order, including valid empty lists and duplicate guide names.
    pub entries: Vec<T>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceGuide {
    pub source_ordinal: u32,
    pub name: String,
    /// Original formula text. Syntax/name resolution belongs to evaluation.
    pub formula: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceGeometryPoint {
    pub source_ordinal: u32,
    /// ST_AdjCoordinate union: native lexical coordinates or guide names.
    pub x: String,
    pub y: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceGeometryRect {
    pub source_ordinal: u32,
    pub left: String,
    pub top: String,
    pub right: String,
    pub bottom: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SourceAdjustHandle {
    Xy {
        source_ordinal: u32,
        position: SourceGeometryPoint,
        guide_x: Option<String>,
        min_x: Option<String>,
        max_x: Option<String>,
        guide_y: Option<String>,
        min_y: Option<String>,
        max_y: Option<String>,
    },
    Polar {
        source_ordinal: u32,
        position: SourceGeometryPoint,
        guide_radius: Option<String>,
        min_radius: Option<String>,
        max_radius: Option<String>,
        guide_angle: Option<String>,
        min_angle: Option<String>,
        max_angle: Option<String>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceConnectionSite {
    pub source_ordinal: u32,
    pub angle: String,
    pub position: SourceGeometryPoint,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceGeometryPath {
    pub source_ordinal: u32,
    /// Absence and explicit zero remain distinct author declarations.
    pub width: Option<Emu>,
    pub height: Option<Emu>,
    pub fill: Option<NativePathFill>,
    pub stroke: Option<bool>,
    pub extrusion_ok: Option<bool>,
    pub commands: Vec<SourceGeometryCommand>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum NativePathFill {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "norm")]
    Normal,
    #[serde(rename = "lighten")]
    Lighten,
    #[serde(rename = "lightenLess")]
    LightenLess,
    #[serde(rename = "darken")]
    Darken,
    #[serde(rename = "darkenLess")]
    DarkenLess,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SourceGeometryCommand {
    Move {
        source_ordinal: u32,
        to: SourceGeometryPoint,
    },
    Line {
        source_ordinal: u32,
        to: SourceGeometryPoint,
    },
    Arc {
        source_ordinal: u32,
        width_radius: String,
        height_radius: String,
        start_angle: String,
        sweep_angle: String,
    },
    Quadratic {
        source_ordinal: u32,
        control: SourceGeometryPoint,
        to: SourceGeometryPoint,
    },
    Cubic {
        source_ordinal: u32,
        control1: SourceGeometryPoint,
        control2: SourceGeometryPoint,
        to: SourceGeometryPoint,
    },
    Close {
        source_ordinal: u32,
    },
}
