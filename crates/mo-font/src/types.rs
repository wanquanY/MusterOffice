use mo_common::{ByteLength, Digest};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FontRequest {
    pub expected_sha256: Digest,
    pub face_index: u32,
    pub characters: Vec<FontCharacter>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FontCharacter {
    pub codepoint: u32,
    pub variation_selector: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FontInspection {
    pub sha256: Digest,
    pub byte_length: ByteLength,
    pub face_count: u32,
    pub face_index: u32,
    pub sfnt_version: u32,
    pub units_per_em: u16,
    pub glyph_count: u16,
    pub font_revision_16_16: i32,
    pub tables: Vec<FontTable>,
    /// Original record order; no locale-dependent family-name selection.
    pub names: Vec<FontName>,
    /// Format 1 language tags, addressed by name language IDs starting at 0x8000.
    pub language_tags: Vec<String>,
    pub metrics: FontMetrics,
    /// OS/2 flags are font declarations, not legal authorization to distribute.
    pub os2: Option<Os2Metadata>,
    pub axes: Vec<FontAxis>,
    pub instances: Vec<FontInstance>,
    pub cmap: Option<FontCmap>,
    pub coverage: Vec<FontCoverage>,
    pub notices: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FontTable {
    pub tag: String,
    pub offset: ByteLength,
    pub byte_length: ByteLength,
    pub checksum: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FontName {
    pub platform_id: u16,
    pub encoding_id: u16,
    pub language_id: u16,
    pub name_id: u16,
    /// None for an unsupported encoding; source bytes remain bound by SHA-256.
    pub text: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FontMetrics {
    pub horizontal_ascender: i16,
    pub horizontal_descender: i16,
    pub horizontal_line_gap: i16,
    pub vertical: Option<VerticalMetrics>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VerticalMetrics {
    pub ascender: i16,
    pub descender: i16,
    pub line_gap: i16,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Os2Metadata {
    pub version: u16,
    pub weight_class: u16,
    pub width_class: u16,
    pub fs_type: u16,
    pub fs_selection: u16,
    pub typo_ascender: i16,
    pub typo_descender: i16,
    pub typo_line_gap: i16,
    pub win_ascent: u16,
    pub win_descent: u16,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FontAxis {
    pub tag: String,
    pub minimum_16_16: i32,
    pub default_16_16: i32,
    pub maximum_16_16: i32,
    pub flags: u16,
    pub name_id: u16,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FontInstance {
    pub subfamily_name_id: u16,
    pub postscript_name_id: Option<u16>,
    pub flags: u16,
    /// Exact fvar 16.16 coordinates in original axis order, before avar mapping.
    pub coordinates_16_16: Vec<i32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FontCmap {
    pub record_index: u16,
    pub platform_id: u16,
    pub encoding_id: u16,
    pub format: u16,
    pub variation_record_index: Option<u16>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FontCoverage {
    pub character: FontCharacter,
    pub outcome: CoverageOutcome,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum CoverageOutcome {
    Mapped { glyph_id: u32 },
    Missing,
    UnsupportedVariation,
    NoUnicodeCmap,
}

#[derive(Debug, Clone, Copy)]
pub struct FontLimits {
    pub max_bytes: usize,
    pub max_faces: usize,
    pub max_tables: usize,
    pub max_names: usize,
    pub max_name_bytes: usize,
    pub max_axes: usize,
    pub max_instances: usize,
    pub max_queries: usize,
}
impl Default for FontLimits {
    fn default() -> Self {
        Self {
            max_bytes: 128 * 1024 * 1024,
            max_faces: 256,
            max_tables: 256,
            max_names: 4096,
            max_name_bytes: 1024 * 1024,
            max_axes: 64,
            max_instances: 4096,
            max_queries: 100_000,
        }
    }
}
