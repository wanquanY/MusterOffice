use mo_common::{ByteLength, Digest, InvalidId, RequestId, ResourceId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct AssetId(RequestId);
impl AssetId {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidId> {
        RequestId::new(value).map(Self)
    }
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
impl fmt::Display for AssetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetBinding {
    pub resource_id: ResourceId,
    pub asset_id: AssetId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetDescriptor {
    pub sha256: Digest,
    pub byte_length: ByteLength,
    /// A declaration, not evidence of successful decoding or safe execution.
    pub media_type: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum AssetVerification {
    BytesSha256,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetInfo {
    pub id: AssetId,
    pub descriptor: AssetDescriptor,
    pub verification: AssetVerification,
}
