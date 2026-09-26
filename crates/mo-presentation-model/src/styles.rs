use mo_common::{Emu, FontId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum Inherited<T> {
    #[default]
    Inherit,
    Value(T),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Rgba {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Color {
    Srgb { rgba: Rgba },
    Theme { slot: ThemeColor },
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum ThemeColor {
    Dark1,
    Light1,
    Dark2,
    Light2,
    Accent1,
    Accent2,
    Accent3,
    Accent4,
    Accent5,
    Accent6,
    Hyperlink,
    FollowedHyperlink,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Fill {
    None,
    Solid { color: Color },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Stroke {
    None {},
    Solid {
        color: Color,
        width: Emu,
        /// Absent retains an unresolved declaration, not an implicit flat cap.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cap: Option<LineCap>,
        /// Absent retains the source/default distinction.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        join: Option<LineJoin>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum LineCap {
    Flat,
    Round,
    Square,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum LineJoin {
    Round {},
    Bevel {},
    Miter {
        /// Ratio in 1/100000 units: 400000 denotes four times line width.
        /// The ratio compares full miter length with the full stroke width.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        limit: Option<u32>,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Appearance {
    #[serde(default)]
    pub fill: Inherited<Fill>,
    #[serde(default)]
    pub stroke: Inherited<Stroke>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterStyle {
    #[serde(default)]
    pub font: Inherited<FontId>,
    #[serde(default)]
    pub size: Inherited<Emu>,
    #[serde(default)]
    pub color: Inherited<Color>,
    #[serde(default)]
    pub bold: Inherited<bool>,
    #[serde(default)]
    pub italic: Inherited<bool>,
    #[serde(default)]
    pub underline: Inherited<bool>,
    #[serde(default)]
    pub language: Inherited<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Alignment {
    #[default]
    Start,
    Center,
    End,
    Justify,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum TextDirection {
    #[default]
    LeftToRight,
    RightToLeft,
    VerticalRightToLeft,
    VerticalLeftToRight,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParagraphStyle {
    #[serde(default)]
    pub alignment: Inherited<Alignment>,
    #[serde(default)]
    pub direction: Inherited<TextDirection>,
    #[serde(default)]
    pub space_before: Inherited<Emu>,
    #[serde(default)]
    pub space_after: Inherited<Emu>,
}
