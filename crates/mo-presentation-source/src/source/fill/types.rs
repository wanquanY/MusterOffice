use crate::source::drawingml::{NativeCoordinate, NativePercentage, SourceColor};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceBackgroundFillUsage {
    pub source_ordinal: u32,
    pub value: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceFill {
    pub source_ordinal: u32,
    pub definition: SourceFillDefinition,
    /// Physical nodes in the immutable source part. Consumers must resolve or
    /// diagnose these before claiming a complete brush.
    pub retained_ordinals: Vec<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SourceFillDefinition {
    None {},
    Solid { color: Option<SourceColor> },
    Gradient(SourceGradientFill),
    Pattern(SourcePatternFill),
    Image(SourceImageFill),
    Group {},
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceGradientFill {
    pub stops: Option<SourceGradientStops>,
    pub shade: Option<SourceGradientShade>,
    pub tile_rect: Option<SourceFillRect>,
    pub flip: Option<NativeTileFlip>,
    pub rotate_with_shape: Option<bool>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceGradientStops {
    pub source_ordinal: u32,
    /// Native order is significant, including repeated stop positions.
    pub entries: Vec<SourceGradientStop>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceGradientStop {
    pub source_ordinal: u32,
    pub position: NativePercentage,
    pub color: SourceColor,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SourceGradientShade {
    Linear {
        source_ordinal: u32,
        angle: Option<u32>,
        scaled: Option<bool>,
    },
    Path {
        source_ordinal: u32,
        path: Option<NativePathShade>,
        fill_to_rect: Option<SourceFillRect>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceFillRect {
    pub source_ordinal: u32,
    pub left: Option<NativePercentage>,
    pub top: Option<NativePercentage>,
    pub right: Option<NativePercentage>,
    pub bottom: Option<NativePercentage>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourcePatternFill {
    pub preset: Option<NativePattern>,
    pub foreground: Option<SourceFillColor>,
    pub background: Option<SourceFillColor>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceFillColor {
    pub source_ordinal: u32,
    pub color: SourceColor,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceImageFill {
    pub blip: Option<SourceFillBlip>,
    pub source_rect: Option<SourceFillRect>,
    pub mode: Option<SourceImageFillMode>,
    pub dpi: Option<u32>,
    pub rotate_with_shape: Option<bool>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceFillBlip {
    pub source_ordinal: u32,
    /// Relationship IDs only: the core never opens a network URL or file path.
    pub embed: Option<String>,
    pub link: Option<String>,
    pub compression: Option<NativeBlipCompression>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effect_nodes: Vec<u32>,
    /// Uninterpreted properties only; known effects use the part catalog.
    pub retained_ordinals: Vec<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SourceImageFillMode {
    Tile(SourceFillTile),
    Stretch {
        source_ordinal: u32,
        fill_rect: Option<SourceFillRect>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceFillTile {
    pub source_ordinal: u32,
    pub translate_x: Option<NativeCoordinate>,
    pub translate_y: Option<NativeCoordinate>,
    pub scale_x: Option<NativePercentage>,
    pub scale_y: Option<NativePercentage>,
    pub flip: Option<NativeTileFlip>,
    pub alignment: Option<NativeFillAlignment>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceFillReference {
    pub source_ordinal: u32,
    /// 0/1000 mean no fill; 1..999 select fills; 1001+ select background fills.
    /// Selection and inherited color substitution belong to resolution.
    pub index: u32,
    pub color: Option<SourceColor>,
    pub retained_ordinals: Vec<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceBackground {
    pub source_ordinal: u32,
    pub black_white_mode: Option<NativeBlackWhiteMode>,
    pub definition: SourceBackgroundDefinition,
    pub retained_ordinals: Vec<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SourceBackgroundDefinition {
    Properties {
        source_ordinal: u32,
        shade_to_title: Option<bool>,
        fill: Box<SourceFill>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        effects: Option<super::super::effects::SourceEffectProperties>,
        retained_ordinals: Vec<u32>,
    },
    Reference(SourceFillReference),
}

macro_rules! native_enum {
    ($name:ident { $($variant:ident => $spelling:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
        pub enum $name { $(#[serde(rename = $spelling)] $variant),+ }
    };
}
native_enum!(NativeTileFlip { None => "none", X => "x", Y => "y", Xy => "xy" });
native_enum!(NativePathShade { Shape => "shape", Circle => "circle", Rectangle => "rect" });
native_enum!(NativeFillAlignment { TopLeft => "tl", Top => "t", TopRight => "tr", Left => "l", Center => "ctr", Right => "r", BottomLeft => "bl", Bottom => "b", BottomRight => "br" });
native_enum!(NativeBlipCompression { Email => "email", Screen => "screen", Print => "print", HighQualityPrint => "hqprint", None => "none" });
native_enum!(NativeBlackWhiteMode { Color => "clr", Auto => "auto", Gray => "gray", LightGray => "ltGray", InverseGray => "invGray", GrayWhite => "grayWhite", BlackGray => "blackGray", BlackWhite => "blackWhite", Black => "black", White => "white", Hidden => "hidden" });
// ST_PresetPatternVal enumeration facts, ECMA-376 Part 1, dml-main.xsd.
native_enum!(NativePattern {
    Pct5 => "pct5", Pct10 => "pct10", Pct20 => "pct20", Pct25 => "pct25", Pct30 => "pct30", Pct40 => "pct40", Pct50 => "pct50", Pct60 => "pct60", Pct70 => "pct70", Pct75 => "pct75", Pct80 => "pct80", Pct90 => "pct90",
    Horz => "horz", Vert => "vert", LtHorz => "ltHorz", LtVert => "ltVert", DkHorz => "dkHorz", DkVert => "dkVert", NarHorz => "narHorz", NarVert => "narVert", DashHorz => "dashHorz", DashVert => "dashVert", Cross => "cross",
    DnDiag => "dnDiag", UpDiag => "upDiag", LtDnDiag => "ltDnDiag", LtUpDiag => "ltUpDiag", DkDnDiag => "dkDnDiag", DkUpDiag => "dkUpDiag", WdDnDiag => "wdDnDiag", WdUpDiag => "wdUpDiag", DashDnDiag => "dashDnDiag", DashUpDiag => "dashUpDiag", DiagCross => "diagCross",
    SmCheck => "smCheck", LgCheck => "lgCheck", SmGrid => "smGrid", LgGrid => "lgGrid", DotGrid => "dotGrid", SmConfetti => "smConfetti", LgConfetti => "lgConfetti", HorzBrick => "horzBrick", DiagBrick => "diagBrick", SolidDmnd => "solidDmnd", OpenDmnd => "openDmnd", DotDmnd => "dotDmnd", Plaid => "plaid", Sphere => "sphere", Weave => "weave", Divot => "divot", Shingle => "shingle", Wave => "wave", Trellis => "trellis", ZigZag => "zigZag"
});
