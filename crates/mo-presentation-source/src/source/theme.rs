//! Theme author declarations and per-family source bindings. No host fonts,
//! implicit system colors, rasterization or effect evaluation occur here.
mod defaults;
mod read;
pub use super::drawingml::{
    ColorSlot, NativePercentage, PresetColor, SchemeColor, SourceColor, SourceColorTransform,
    SourceColorValue, SourceTextFont, SystemColor,
};
use super::*;
use crate::{PptxError, cancelled};
pub use defaults::{SourceThemeDefaultKind, SourceThemeTextDefault, SourceThemeTextDefaults};
use mo_opc::PartName;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SourceThemeKind {
    Theme,
    Override,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceThemePart {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_defaults: Option<SourceThemeTextDefaults>,
    pub sha256: Digest,
    pub kind: SourceThemeKind,
    pub name: Option<String>,
    pub color_scheme: Option<SourceColorScheme>,
    pub font_scheme: Option<SourceFontScheme>,
    pub format_scheme: Option<SourceFormatScheme>,
    pub compatibility: SourceCompatibility,
    pub notices: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub effect_nodes: BTreeMap<u32, effects::SourceEffectNode>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceColorScheme {
    pub source_ordinal: u32,
    pub name: String,
    pub colors: BTreeMap<ColorSlot, SourceColor>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceSupplementalFont {
    pub script: String,
    pub typeface: String,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceFontCollection {
    pub latin: Option<SourceTextFont>,
    pub east_asian: Option<SourceTextFont>,
    pub complex_script: Option<SourceTextFont>,
    /// Preserve order and duplicate declarations for later font-profile policy.
    pub supplemental: Vec<SourceSupplementalFont>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceFontScheme {
    pub source_ordinal: u32,
    /// Unknown font-scheme declarations prevent claiming resolved font semantics.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retained_ordinals: Vec<u32>,
    pub name: String,
    pub major: SourceFontCollection,
    pub minor: SourceFontCollection,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceStyleEntry {
    pub source_ordinal: u32,
    /// DrawingML kind; content remains in the digest-bound source part.
    pub local_name: String,
    /// Parsed line style declaration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<line::SourceLine>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<fill::SourceFill>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect_style: Option<effects::SourceEffectStyle>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceFormatScheme {
    pub source_ordinal: u32,
    pub name: Option<String>,
    pub fills: Vec<SourceStyleEntry>,
    pub lines: Vec<SourceStyleEntry>,
    pub effects: Vec<SourceStyleEntry>,
    pub background_fills: Vec<SourceStyleEntry>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceThemeSchemeRef {
    pub part: String,
    pub source_ordinal: u32,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceThemeSelection {
    pub colors: Option<SourceThemeSchemeRef>,
    pub fonts: Option<SourceThemeSchemeRef>,
    pub format: Option<SourceThemeSchemeRef>,
}

struct Budget {
    elements: usize,
    attribute_bytes: usize,
}
pub(super) fn load(
    package: &dyn PackageRead,
    surfaces: &mut BTreeMap<String, SourceSurface>,
    limits: SourceLimits,
    line_budget: &mut line::Budget,
    paint_budget: &mut paint::Budget,
    text_budget: &mut text::Budget,
    check: &dyn Fn() -> bool,
) -> Result<BTreeMap<String, SourceThemePart>, PptxError> {
    let mut kinds = BTreeMap::new();
    for surface in surfaces.values() {
        for (reference, kind) in [
            (surface.links.theme.as_ref(), SourceThemeKind::Theme),
            (
                surface.links.theme_override.as_ref(),
                SourceThemeKind::Override,
            ),
        ] {
            if let Some(reference) = reference {
                if kinds
                    .insert(reference.part.clone(), kind)
                    .is_some_and(|prior| prior != kind)
                {
                    return Err(crate::value(
                        &reference.part,
                        "conflicting theme part roles",
                    ));
                }
                if kinds.len() > limits.max_theme_parts {
                    return Err(PptxError::Limit("theme parts"));
                }
            }
        }
    }
    let mut themes = BTreeMap::new();
    let mut budget = Budget {
        elements: 0,
        attribute_bytes: 0,
    };
    for (part, kind) in kinds {
        cancelled(check)?;
        let name = PartName::new(&part)?;
        let bytes = package.read_part(&name, limits.package.xml.max_bytes as u64, check)?;
        let theme = read::parse(
            &bytes,
            kind,
            package.parts()[&name].sha256.clone(),
            limits,
            &mut budget,
            line_budget,
            paint_budget,
            text_budget,
            check,
        )?;
        themes.insert(part, theme);
    }
    select(surfaces, &themes, check)?;
    Ok(themes)
}

pub(super) fn select(
    surfaces: &mut BTreeMap<String, SourceSurface>,
    themes: &BTreeMap<String, SourceThemePart>,
    check: &dyn Fn() -> bool,
) -> Result<(), PptxError> {
    for surface in surfaces.values_mut() {
        cancelled(check)?;
        let mut selection = SourceThemeSelection::default();
        for reference in surface
            .effective_theme
            .base
            .iter()
            .chain(&surface.effective_theme.overrides)
        {
            let theme = &themes[&reference.part];
            let bound = |source_ordinal| SourceThemeSchemeRef {
                part: reference.part.clone(),
                source_ordinal,
            };
            if let Some(scheme) = &theme.color_scheme {
                selection.colors = Some(bound(scheme.source_ordinal));
            }
            if let Some(scheme) = &theme.font_scheme {
                selection.fonts = Some(bound(scheme.source_ordinal));
            }
            if let Some(scheme) = &theme.format_scheme {
                selection.format = Some(bound(scheme.source_ordinal));
            }
        }
        surface.theme_selection = selection;
    }
    Ok(())
}
