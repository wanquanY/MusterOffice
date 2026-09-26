use mo_common::Emu;
use mo_presentation_model::{Rgba, ThemeColor};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportDefaults {
    /// Explicit host choice. Export alone does not resolve, load or embed system fonts.
    pub font_family: String,
    pub text_size: Emu,
    pub text_color: Rgba,
    pub page_background: Rgba,
    /// All 12 native theme color slots are required for generated fallback definitions.
    pub theme_colors: BTreeMap<ThemeColor, Rgba>,
    pub font_delivery: FontDelivery,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum FontDelivery {
    /// Retains editable font-family references. It is not a font fidelity guarantee.
    ReferenceOnly,
}

pub const COLOR_SLOTS: [(ThemeColor, &str); 12] = [
    (ThemeColor::Dark1, "dk1"),
    (ThemeColor::Light1, "lt1"),
    (ThemeColor::Dark2, "dk2"),
    (ThemeColor::Light2, "lt2"),
    (ThemeColor::Accent1, "accent1"),
    (ThemeColor::Accent2, "accent2"),
    (ThemeColor::Accent3, "accent3"),
    (ThemeColor::Accent4, "accent4"),
    (ThemeColor::Accent5, "accent5"),
    (ThemeColor::Accent6, "accent6"),
    (ThemeColor::Hyperlink, "hlink"),
    (ThemeColor::FollowedHyperlink, "folHlink"),
];
