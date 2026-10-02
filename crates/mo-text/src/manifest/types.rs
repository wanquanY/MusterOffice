use super::*;
use mo_common::Digest;
use mo_unicode::bidi::ParagraphDirection;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum FontManifestProfile {
    #[serde(rename = "explicit-font-resource-manifest-draft-v1")]
    ExplicitDraftV1,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FontNameBinding {
    /// Exact index in VerifiedFont metadata.names, preserving original record order.
    pub record: u32,
    pub expected: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestFace {
    pub font: u32,
    pub family: FontNameBinding,
    pub subfamily: FontNameBinding,
    pub postscript: Option<FontNameBinding>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestInstance {
    pub face: u32,
    /// All axes are validated, including instances unused by this paragraph.
    pub variations: Vec<ShapeVariation>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum TypefaceMappingPolicy {
    /// The native typeface must exactly match the selected verified family name.
    ExactFamily {},
    /// Explicit host-supplied substitution contract. A digest is evidence identity,
    /// not proof of user permission, font licensing or layout equivalence.
    Substitution {
        profile_sha256: Digest,
        reason: String,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum FontStyle {
    Regular,
    Bold,
    Italic,
    BoldItalic,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestTypeface {
    pub typeface: String,
    pub policy: TypefaceMappingPolicy,
    /// Ordered, explicit coverage fallbacks into this manifest's typefaces.
    /// Only these entries are tried; their own fallbacks are not expanded.
    /// Each candidate uses the requested style slot without synthesis.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fallbacks: Vec<String>,
    /// Explicit instance selection; no synthesized bold/slant or slot fallback.
    pub regular: Option<ManifestInstance>,
    pub bold: Option<ManifestInstance>,
    pub italic: Option<ManifestInstance>,
    pub bold_italic: Option<ManifestInstance>,
}
impl ManifestTypeface {
    pub(super) fn slots(&self) -> [Option<&ManifestInstance>; 4] {
        [
            self.regular.as_ref(),
            self.bold.as_ref(),
            self.italic.as_ref(),
            self.bold_italic.as_ref(),
        ]
    }
    pub(super) fn slot(&self, style: FontStyle) -> Option<&ManifestInstance> {
        match style {
            FontStyle::Regular => self.regular.as_ref(),
            FontStyle::Bold => self.bold.as_ref(),
            FontStyle::Italic => self.italic.as_ref(),
            FontStyle::BoldItalic => self.bold_italic.as_ref(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FontManifest {
    pub profile: FontManifestProfile,
    pub fonts: Vec<CascadeFont>,
    pub faces: Vec<ManifestFace>,
    pub typefaces: Vec<ManifestTypeface>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestTextStyle {
    pub typeface: String,
    pub font_style: FontStyle,
    pub language: String,
    pub features: Vec<ShapeFeature>,
    pub suppress_dotted_circle: bool,
    pub max_glyphs: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestParagraphRequest {
    pub manifest: FontManifest,
    pub text: String,
    pub direction: ParagraphDirection,
    pub spans: Vec<itemize::StyleSpan>,
    pub styles: Vec<ManifestTextStyle>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestStyleBinding {
    pub style: u32,
    pub typeface: u32,
    pub font_style: FontStyle,
    pub face: u32,
    pub candidate: FontCandidate,
    pub policy: TypefaceMappingPolicy,
    /// Candidate evidence in fallback order, after the primary candidate above.
    /// Actual selections and missing-glyph probes remain in the shaping result.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fallbacks: Vec<ManifestFallbackBinding>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestFallbackBinding {
    pub typeface: u32,
    pub face: u32,
    pub candidate: FontCandidate,
    pub policy: TypefaceMappingPolicy,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestParagraphResult {
    pub profile: String,
    pub bindings: Vec<ManifestStyleBinding>,
    /// Full hash/name/axis verification precedes shaping, including unused slots.
    pub shaping: ParagraphShapeResult,
}
#[derive(Debug, Clone, Copy)]
pub struct ManifestLimits {
    pub max_fonts: usize,
    pub max_faces: usize,
    pub max_typefaces: usize,
    pub max_name_bytes: usize,
}
impl Default for ManifestLimits {
    fn default() -> Self {
        Self {
            max_fonts: 32,
            max_faces: 256,
            max_typefaces: 256,
            max_name_bytes: 1024 * 1024,
        }
    }
}
