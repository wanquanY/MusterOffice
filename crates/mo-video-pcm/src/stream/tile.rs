use super::*;
use crate::PcmTileRef;
use mo_common::{ByteLength, Ticks};
use serde::{Deserialize, Serialize};

/// May be persisted only in an unpublished host attempt. This is not accepted
/// by PcmTileReader. `seal_ref` binds it after complete decoded extent is known.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnsealedPcmTileRef {
    pub origin: Digest,
    pub index: u32,
    pub first_sample: Ticks,
    pub frames: u32,
    pub sha256: Digest,
    pub byte_length: ByteLength,
}
#[derive(Debug)]
pub struct UnsealedPcmTile {
    pub reference: UnsealedPcmTileRef,
    pub bytes: Vec<u8>,
}
impl CompletedPcmStream {
    pub fn decoded(&self) -> &InspectedPcm {
        &self.decoded
    }
    pub fn source(&self) -> &Arc<PcmSource> {
        &self.source
    }
    /// Call from the original host attempt, after ALL emitted tiles and tail
    /// have been durably stored. Authorization and attempt isolation belong to
    /// the host; this validates identity and exact geometry, not blob existence.
    pub fn seal_ref(&self, reference: &UnsealedPcmTileRef) -> Result<PcmTileRef> {
        if reference.origin != self.provisional_origin {
            return Err(PcmError::Invalid("PCM provisional origin"));
        }
        let sealed = PcmTileRef {
            source: self.source.identity().clone(),
            index: reference.index,
            first_sample: reference.first_sample,
            frames: reference.frames,
            sha256: reference.sha256.clone(),
            byte_length: reference.byte_length,
        };
        self.source.validate_ref(&sealed)?;
        Ok(sealed)
    }
}
pub(super) fn emit(
    origin: &Digest,
    index: u32,
    tile_frames: u32,
    channels: usize,
    samples: &[f32],
    cancelled: &dyn Fn() -> bool,
) -> Result<UnsealedPcmTile> {
    if channels == 0
        || samples.is_empty()
        || !samples.len().is_multiple_of(channels)
        || samples.len() / channels > tile_frames as usize
    {
        return Err(PcmError::Invalid("PCM unsealed tile shape"));
    }
    let (data, sha256) = crate::encoding::encode(samples, cancelled)?;
    let first = i64::from(index) * i64::from(tile_frames);
    Ok(UnsealedPcmTile {
        reference: UnsealedPcmTileRef {
            origin: origin.clone(),
            index,
            first_sample: Ticks::new(first),
            frames: (samples.len() / channels) as u32,
            sha256,
            byte_length: ByteLength::new(data.len() as u64),
        },
        bytes: data,
    })
}
