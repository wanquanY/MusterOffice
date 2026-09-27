use crate::{AssetDescriptor, AssetInfo, Failure, UnixMillis, UploadId};
use mo_common::{ByteLength, RequestId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Binary data is transferred separately, never inside tool JSON.
pub const ASSET_CHUNK_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UploadRequest {
    pub request_id: RequestId,
    pub descriptor: AssetDescriptor,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum UploadState {
    Uploading,
    Verifying,
    Sealed,
    Failed,
    Cancelled,
}
impl UploadState {
    pub fn terminal(self) -> bool {
        matches!(self, Self::Sealed | Self::Failed | Self::Cancelled)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UploadInfo {
    pub id: UploadId,
    pub request_id: RequestId,
    pub descriptor: AssetDescriptor,
    pub state: UploadState,
    pub chunk_bytes: u32,
    pub received_bytes: ByteLength,
    pub created_at: UnixMillis,
    pub updated_at: UnixMillis,
    pub expires_at: Option<UnixMillis>,
    pub asset: Option<AssetInfo>,
    pub error: Option<Failure>,
}
