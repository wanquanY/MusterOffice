use crate::*;
#[cfg(test)]
mod tests;
use mo_presentation_delivery::{DeliveryExpectation, DeliveryLimits, RendererIdentity};
use mo_presentation_edit::SnapshotRecord;
use mo_presentation_operations::{
    AssetId, AssetInfo, DocumentAction, ExportReceipt, OperationRequest,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    io::{Read, Write},
};

pub(crate) const VERSION: &str = "musteroffice.native-export/2-draft";
// Snapshot + operation + bounded asset descriptors; no resource bytes in JSON.
pub(crate) const MAX_METADATA: usize = 65 * 1024 * 1024;
pub(crate) const MAX_INPUT_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Request {
    pub version: String,
    pub request: OperationRequest,
    pub snapshot: SnapshotRecord,
    pub assets: Vec<AssetInfo>,
}
#[derive(Serialize, Deserialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum Response {
    Prepared {
        request_digest: Digest,
        receipt: Box<ExportReceipt>,
    },
    Failed {
        error: Failure,
    },
}
impl Request {
    pub fn validate(&self, renderer: &RendererIdentity) -> Result<DeliveryExpectation, Failure> {
        self.request.validate_profile()?;
        let DocumentAction::Export {
            document_id,
            base_revision,
            settings,
        } = &self.request.action
        else {
            return Err(invalid("export action required"));
        };
        if self.version != VERSION || &settings.renderer != renderer {
            return Err(Failure::new(
                FailureCode::ExecutorMismatch,
                "export worker version or renderer differs",
            ));
        }
        if &self.snapshot.document.id != document_id || &self.snapshot.revision != base_revision {
            return Err(Failure::new(
                FailureCode::RevisionConflict,
                "export snapshot differs",
            ));
        }
        let limits = DeliveryLimits::default();
        if self.assets.len() > limits.max_artifacts {
            return Err(invalid("export asset count"));
        }
        let required: BTreeSet<&AssetId> = settings
            .resources
            .iter()
            .map(|r| &r.asset_id)
            .chain(settings.font_asset_id.iter())
            .collect();
        let mut ids = BTreeSet::new();
        let mut total = 0u64;
        for asset in &self.assets {
            let n = asset.descriptor.byte_length.get();
            if !ids.insert(&asset.id)
                || !required.contains(&asset.id)
                || n > limits.max_asset_bytes
                || asset.descriptor.media_type.len() > 255
            {
                return Err(invalid("export input asset binding or limit"));
            }
            total = total
                .checked_add(n)
                .filter(|n| *n <= MAX_INPUT_BYTES)
                .ok_or_else(|| invalid("export input bytes"))?;
        }
        if ids != required {
            return Err(invalid("export input asset coverage"));
        }
        let font_digest = match settings.font_asset_id.as_ref() {
            Some(id) => {
                let asset = self
                    .assets
                    .iter()
                    .find(|a| &a.id == id)
                    .expect("coverage checked");
                if asset.descriptor.byte_length.get() > limits.max_font_bytes {
                    return Err(invalid("export font bytes"));
                }
                asset.descriptor.sha256.clone()
            }
            None => {
                use sha2::Digest as _;
                Digest::from_sha256(sha2::Sha256::digest([]).into())
            }
        };
        Ok(DeliveryExpectation {
            document_id: document_id.clone(),
            revision: base_revision.clone(),
            semantic_digest: self.snapshot.semantic_digest.clone(),
            renderer: renderer.clone(),
            settings_digest: settings
                .delivery
                .input_digest(&font_digest, renderer)
                .map_err(|_| invalid("export settings digest"))?,
        })
    }
}
pub(crate) fn encode(value: &impl Serialize) -> Result<Vec<u8>, Failure> {
    encode_limited(value, MAX_METADATA)
}
// One bounded allocation path for metadata, including the length prefix.
// Streaming JSON serialization stops at the limit instead of first allocating
// an arbitrarily large JSON buffer and then copying it into a framed buffer.
fn encode_limited(value: &impl Serialize, limit: usize) -> Result<Vec<u8>, Failure> {
    struct Buffer {
        bytes: Vec<u8>,
        limit: usize,
    }
    impl Write for Buffer {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let length = self
                .bytes
                .len()
                .checked_add(bytes.len())
                .filter(|n| *n <= self.limit)
                .ok_or_else(|| std::io::Error::other("export metadata byte limit"))?;
            if length > self.bytes.capacity() {
                let capacity = length
                    .max(self.bytes.capacity().saturating_mul(2))
                    .min(self.limit);
                self.bytes
                    .try_reserve_exact(capacity - self.bytes.len())
                    .map_err(std::io::Error::other)?;
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut buffer = Buffer {
        bytes: vec![0; 4],
        limit: limit + 4,
    };
    serde_json::to_writer(&mut buffer, value)
        .map_err(|_| invalid("export metadata serialization or limit"))?;
    let length = (buffer.bytes.len() - 4) as u32;
    buffer.bytes[..4].copy_from_slice(&length.to_le_bytes());
    Ok(buffer.bytes)
}
pub(crate) fn read<T: serde::de::DeserializeOwned>(reader: &mut impl Read) -> Result<T, Failure> {
    let mut length = [0; 4];
    reader.read_exact(&mut length).map_err(storage_failure)?;
    let length = u32::from_le_bytes(length) as usize;
    if length > MAX_METADATA {
        return Err(invalid("export metadata bytes"));
    }
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes).map_err(storage_failure)?;
    mo_common::from_json_str(
        std::str::from_utf8(&bytes).map_err(|_| invalid("export metadata UTF-8"))?,
    )
    .map_err(|_| invalid("export metadata schema"))
}
pub(crate) fn write(output: &mut impl Write, response: &Response) -> Result<(), Failure> {
    output
        .write_all(&encode(response)?)
        .map_err(storage_failure)
}
