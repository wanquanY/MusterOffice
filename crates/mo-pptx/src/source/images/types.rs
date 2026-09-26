use crate::source::fill::resolve::{
    EffectiveImageFill, FillOrigin, FillRedirect, FillResolveLimits, FillTarget, FillUnresolved,
    SourceFillQuery,
};
use mo_common::{ByteLength, Digest};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Explicit source policy. An embedded snapshot is never silently substituted
/// for a requested linked source, nor does inspection grant network authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ImageSourceSelection {
    EmbeddedSnapshot,
    LinkedSource,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceImageQuery {
    pub fill: SourceFillQuery,
    pub selection: ImageSourceSelection,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncodedImageResource {
    pub part: String,
    pub content_type: String,
    pub sha256: Digest,
    pub byte_length: ByteLength,
    /// Contiguous offset in the separately extracted encoded-resource bundle.
    pub offset: ByteLength,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceImageReference {
    pub declared_by: FillOrigin,
    pub owner_part: String,
    pub relationship_id: String,
    pub target_uri: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceImageBinding {
    pub declared_by: FillOrigin,
    pub image: EffectiveImageFill,
    pub redirects: Vec<FillRedirect>,
    pub reference: SourceImageReference,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ImageReferenceIssue {
    MissingSelectedReference {},
    MissingDeclaringPart {},
    MissingRelationship {
        owner_part: String,
        relationship_id: String,
    },
    WrongRelationshipType {
        reference: SourceImageReference,
        relationship_type: String,
    },
    WrongTargetMode {
        reference: SourceImageReference,
    },
    Fragment {
        reference: SourceImageReference,
    },
    NonImageContentType {
        reference: SourceImageReference,
        content_type: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum SourceImageOutcome {
    Available {
        resource: u32,
        binding: Box<SourceImageBinding>,
    },
    /// A request for host-authorized bytes, never a command to fetch the URI.
    ExternalRequired {
        binding: Box<SourceImageBinding>,
    },
    UnresolvedFill {
        reason: Box<FillUnresolved>,
    },
    UnresolvedReference {
        issue: Box<ImageReferenceIssue>,
    },
    NotImage {},
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceImageResult {
    pub target: FillTarget,
    pub outcome: SourceImageOutcome,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceImageResources {
    pub source_sha256: Digest,
    pub surface: String,
    pub selection: ImageSourceSelection,
    pub targets: Vec<SourceImageResult>,
    pub resources: Vec<EncodedImageResource>,
    pub bundle_byte_length: ByteLength,
}
#[derive(Debug, Clone, Copy)]
pub struct SourceImageLimits {
    pub fills: FillResolveLimits,
    pub max_relationship_steps: usize,
    pub max_resources: usize,
    pub max_metadata_bytes: usize,
    pub max_resource_bytes: u64,
    pub max_bundle_bytes: u64,
}
impl Default for SourceImageLimits {
    fn default() -> Self {
        Self {
            fills: FillResolveLimits::default(),
            max_relationship_steps: 1_000_000,
            max_resources: 4096,
            max_metadata_bytes: 4 * 1024 * 1024,
            max_resource_bytes: 32 * 1024 * 1024,
            max_bundle_bytes: 64 * 1024 * 1024,
        }
    }
}
