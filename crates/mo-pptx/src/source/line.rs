//! Native line declarations. Missing values and source bindings survive reading;
//! theme/placeholder inheritance and drawing are separate computations.
pub mod colors;
mod read;
pub mod resolve;
use super::drawingml::{NativePercentage, SourceColor};
use mo_common::Emu;
pub(super) use read::{Budget, Declaration, Reader};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceLine {
    pub source_ordinal: u32,
    pub width: Option<Emu>,
    pub cap: Option<NativeLineCap>,
    pub compound: Option<NativeCompoundLine>,
    pub alignment: Option<NativePenAlignment>,
    pub fill: Option<SourceLineFill>,
    pub dash: Option<SourceLineDash>,
    pub join: Option<SourceLineJoin>,
    pub head: Option<SourceLineEnd>,
    pub tail: Option<SourceLineEnd>,
    /// Unsupported attributes/extensions bind to their owning elements. They
    /// cannot be discarded or counted as resolved line semantics.
    pub retained_ordinals: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceLineReference {
    pub source_ordinal: u32,
    pub index: u32,
    pub color: Option<SourceColor>,
    pub retained_ordinals: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SourceLineFill {
    None {
        source_ordinal: u32,
    },
    Solid {
        source_ordinal: u32,
        color: Option<SourceColor>,
    },
    Gradient {
        source_ordinal: u32,
        gradient: Box<super::fill::SourceGradientFill>,
    },
    Pattern {
        source_ordinal: u32,
        pattern: Box<super::fill::SourcePatternFill>,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum NativeRetainedLineFill {
    Gradient,
    Pattern,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SourceLineDash {
    Preset {
        source_ordinal: u32,
        value: Option<NativePresetDash>,
    },
    Custom {
        source_ordinal: u32,
        stops: Vec<SourceDashStop>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceDashStop {
    pub source_ordinal: u32,
    pub dash: NativePercentage,
    pub space: NativePercentage,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SourceLineJoin {
    Round {
        source_ordinal: u32,
    },
    Bevel {
        source_ordinal: u32,
    },
    Miter {
        source_ordinal: u32,
        limit: Option<NativePercentage>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceLineEnd {
    pub source_ordinal: u32,
    pub kind: Option<NativeLineEnd>,
    pub width: Option<NativeLineEndSize>,
    pub length: Option<NativeLineEndSize>,
}

// Exact enum spellings from the pinned ECMA-376 Transitional dml-main.xsd.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum NativeLineCap {
    #[serde(rename = "flat")]
    Flat,
    #[serde(rename = "rnd")]
    Round,
    #[serde(rename = "sq")]
    Square,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum NativeCompoundLine {
    #[serde(rename = "sng")]
    Single,
    #[serde(rename = "dbl")]
    Double,
    #[serde(rename = "thickThin")]
    ThickThin,
    #[serde(rename = "thinThick")]
    ThinThick,
    #[serde(rename = "tri")]
    Triple,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum NativePenAlignment {
    #[serde(rename = "ctr")]
    Center,
    #[serde(rename = "in")]
    Inset,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum NativePresetDash {
    #[serde(rename = "solid")]
    Solid,
    #[serde(rename = "dot")]
    Dot,
    #[serde(rename = "dash")]
    Dash,
    #[serde(rename = "lgDash")]
    LongDash,
    #[serde(rename = "dashDot")]
    DashDot,
    #[serde(rename = "lgDashDot")]
    LongDashDot,
    #[serde(rename = "lgDashDotDot")]
    LongDashDotDot,
    #[serde(rename = "sysDash")]
    SystemDash,
    #[serde(rename = "sysDot")]
    SystemDot,
    #[serde(rename = "sysDashDot")]
    SystemDashDot,
    #[serde(rename = "sysDashDotDot")]
    SystemDashDotDot,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum NativeLineEnd {
    None,
    Triangle,
    Stealth,
    Diamond,
    Oval,
    Arrow,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum NativeLineEndSize {
    #[serde(rename = "sm")]
    Small,
    #[serde(rename = "med")]
    Medium,
    #[serde(rename = "lg")]
    Large,
}
