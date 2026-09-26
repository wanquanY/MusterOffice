use mo_common::{ByteLength, Digest, DocumentId, RequestId, SlideId};
use mo_pptx::{
    ExportDefaults,
    source::{color::ColorContext, images::ImageSourceSelection},
};
use mo_raster::ImageSampling;
use mo_text::manifest::FontManifest;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const PROFILE: &str = "presentations-pptx-resource-delivery-v1-draft";
pub const MODEL_MIME: &str = "application/vnd.musteroffice.presentation+json";
pub const CONTEXT_MIME: &str = "application/vnd.musteroffice.presentation-context+json";
pub const PPTX_MIME: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.presentation";

/// Explicit render/export choices are input identity. Pixels preserve page
/// aspect ratio; the pipeline derives a viewport covering the entire page.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeliverySettings {
    pub defaults: ExportDefaults,
    pub preview_width: u32,
    pub color_context: ColorContext,
    pub image_source: ImageSourceSelection,
    pub sampling: ImageSampling,
    pub fonts: Option<FontManifest>,
}
impl DeliverySettings {
    pub fn input_digest(
        &self,
        font_sha256: &Digest,
        renderer: &RendererIdentity,
    ) -> Result<Digest, mo_common::CanonicalError> {
        mo_common::digest(
            "musteroffice.delivery-input/1",
            &(self, font_sha256, renderer),
        )
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RendererIdentity {
    pub implementation_sha256: Digest,
    pub profile: String,
}
#[derive(Debug, Clone, Copy)]
pub struct DeliveryLimits {
    pub max_pages: usize,
    pub max_artifacts: usize,
    pub max_model_bytes: u64,
    pub max_asset_bytes: u64,
    pub max_font_bytes: u64,
    pub max_total_bytes: u64,
}
impl Default for DeliveryLimits {
    fn default() -> Self {
        Self {
            max_pages: 256,
            max_artifacts: 1024,
            max_model_bytes: 32 * 1024 * 1024,
            max_asset_bytes: 128 * 1024 * 1024,
            max_font_bytes: 32 * 1024 * 1024,
            max_total_bytes: 512 * 1024 * 1024,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AssetRole {
    EditableDocument,
    Pptx,
    Preview,
    QualityReport,
    Image,
    Font,
    Audio,
    Video,
    Model3d,
    Embedded,
    Source,
    Other,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeliveryAsset {
    pub id: RequestId,
    pub sha256: Digest,
    pub byte_length: ByteLength,
    pub media_type: String,
    pub role: AssetRole,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeliveredDocument {
    pub document_id: DocumentId,
    pub revision: Digest,
    pub model_asset_id: RequestId,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", tag = "mode", deny_unknown_fields)]
pub enum PreviewSample {
    Editor {},
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Preview {
    pub page_id: SlideId,
    pub image_asset_id: RequestId,
    pub width: u32,
    pub height: u32,
    pub sample: PreviewSample,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ClaimKind {
    Structure,
    Layout,
    NativeEditability,
    Playback,
    TargetApplication,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStatus {
    Passed,
    Failed,
    NotProven,
    NotApplicable,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ClaimBasis {
    None,
    StaticInspection,
    Roundtrip,
    ApplicationTest,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Claim {
    pub kind: ClaimKind,
    pub status: ClaimStatus,
    pub subject_sha256: Digest,
    pub basis: ClaimBasis,
    pub profile_id: String,
    pub evidence_asset_ids: Vec<RequestId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Versions {
    pub engine: String,
    pub document_schema: String,
    pub operation_schema: String,
    pub rules: String,
    pub feature_registry_sha256: Digest,
    pub font_profile_sha256: Digest,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeliveryBundle {
    pub version: String,
    pub document: DeliveredDocument,
    pub profile_id: String,
    pub versions: Versions,
    pub pptx_asset_id: RequestId,
    pub assets: Vec<DeliveryAsset>,
    pub previews: Vec<Preview>,
    pub claims: Vec<Claim>,
}
