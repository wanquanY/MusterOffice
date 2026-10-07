//! Canonical raw PCM with an endpoint learned only at successful decoder EOF.
//! Emitted payloads are provisional. The host owns task/attempt identity, I/O,
//! original authorization, cleanup and publication of the entire sealed source.
mod record;
mod tile;
use crate::{InspectedPcm, PcmError, PcmInspectLimits, PcmInspection, PcmSource, Result, check};
use mo_common::{Digest, ResourceId, digest};
use mo_media::DecodedFrame;
use mo_video_audio::ByteOrder;
use mo_video_compile::MediaBinding;
use mo_video_model::{MediaStream, StreamId};
pub use record::*;
use std::sync::Arc;
pub use tile::*;

#[derive(Debug, Clone)]
pub struct PcmStreamOrigin {
    pub resource: ResourceId,
    pub content: Digest,
    pub stream: StreamId,
    /// Fixed decoder, timeline mode and every output-affecting option. An
    /// arbitrary label or compressed content digest alone is insufficient.
    pub decoder_profile: Digest,
}
#[derive(Debug, Clone, Copy)]
pub struct PcmStreamLimits {
    pub inspection: PcmInspectLimits,
    pub tile_frames: u32,
    pub max_tiles: u32,
    pub max_tiles_per_step: usize,
    /// Normalized block + retained carry capacity + this call's encoded tiles.
    /// Decoder frames, host I/O buffers, prior outputs and metadata are separate.
    pub max_working_bytes: usize,
}
impl Default for PcmStreamLimits {
    fn default() -> Self {
        Self {
            inspection: PcmInspectLimits::default(),
            tile_frames: 4096,
            max_tiles: 1_000_000,
            max_tiles_per_step: 1024,
            max_working_bytes: 64 << 20,
        }
    }
}
impl PcmStreamLimits {
    fn validate(&self) -> Result<()> {
        self.inspection.validate()?;
        if !(256..=65536).contains(&self.tile_frames)
            || self.max_tiles == 0
            || self.max_tiles > 10_000_000
            || !(1..=65536).contains(&self.max_tiles_per_step)
            || self.max_working_bytes == 0
            || self.max_working_bytes > 256 << 20
        {
            return Err(PcmError::Invalid("streaming PCM limits"));
        }
        Ok(())
    }
}
/// A single fresh decode, never a collection of independently sought windows.
/// Any failed push or callback panic poisons it permanently. A caller whose
/// storage fails must abandon the attempt, even if computation had succeeded.
pub struct PcmStreamBuilder {
    origin: PcmStreamOrigin,
    provisional_origin: Digest,
    inspection: PcmInspection,
    limits: PcmStreamLimits,
    carry: Vec<f32>,
    channels: usize,
    emitted: u32,
    failed: bool,
}
/// Kernel completion, not proof of persisted blobs or a host commit. The final
/// source identity uses the actual decoded span and the original PCM profile.
#[derive(Debug)]
pub struct CompletedPcmStream {
    decoded: InspectedPcm,
    source: Arc<PcmSource>,
    pub tail: Option<UnsealedPcmTile>,
    provisional_origin: Digest,
    decoder_profile: Digest,
}
impl PcmStreamBuilder {
    pub fn new(origin: PcmStreamOrigin, generation: u64, limits: PcmStreamLimits) -> Result<Self> {
        limits.validate()?;
        let provisional_origin = digest(
            "musteroffice.pcm.unsealed-f32le-tiles/1-draft",
            &(
                crate::PROFILE,
                &origin.content,
                &origin.stream,
                &origin.decoder_profile,
                limits.tile_frames,
            ),
        )?;
        let mut inspection_limits = limits.inspection;
        inspection_limits.frame.max_bytes = inspection_limits
            .frame
            .max_bytes
            .min(limits.max_working_bytes);
        Ok(Self {
            origin,
            provisional_origin,
            inspection: PcmInspection::new(generation, inspection_limits)?,
            limits,
            carry: vec![],
            channels: 0,
            emitted: 0,
            failed: false,
        })
    }
    pub fn retained_bytes(&self) -> usize {
        self.carry.capacity() * 4
    }
    pub fn emitted_tiles(&self) -> u32 {
        self.emitted
    }
    pub fn push(
        &mut self,
        frame: &DecodedFrame,
        order: ByteOrder,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Vec<UnsealedPcmTile>> {
        if self.failed {
            return Err(PcmError::Invalid("streaming PCM failed"));
        }
        self.failed = true;
        check(cancelled)?;
        let remaining_bytes = self
            .limits
            .max_working_bytes
            .saturating_sub(self.retained_bytes());
        let pcm = self
            .inspection
            .observe_bounded(frame, order, remaining_bytes, cancelled)?;
        let channels = pcm.layout().len();
        if self.channels != 0 && self.channels != channels {
            return Err(PcmError::Invalid("PCM stream channels"));
        }
        let tile_samples = self.limits.tile_frames as usize * channels;
        let pending = self
            .carry
            .len()
            .checked_add(pcm.samples().len())
            .ok_or(PcmError::Budget)?;
        let count = pending / tile_samples;
        let emitted = u32::try_from(count)
            .ok()
            .and_then(|n| self.emitted.checked_add(n))
            .ok_or(PcmError::Budget)?;
        // Include a possible incomplete tail in the whole-source tile budget.
        if emitted
            .checked_add(u32::from(pending % tile_samples != 0))
            .is_none_or(|n| n > self.limits.max_tiles)
            || count > self.limits.max_tiles_per_step
        {
            return Err(PcmError::Budget);
        }
        let working = pcm
            .samples()
            .len()
            .checked_add(self.carry.capacity().max(tile_samples))
            .and_then(|n| n.checked_add(count * tile_samples))
            .and_then(|n| n.checked_mul(4))
            .ok_or(PcmError::Budget)?;
        if working > self.limits.max_working_bytes {
            return Err(PcmError::Budget);
        }
        if self.carry.capacity() < tile_samples {
            self.carry
                .try_reserve_exact(tile_samples - self.carry.len())
                .map_err(|_| PcmError::Budget)?;
        }
        self.channels = channels;
        let mut out = Vec::new();
        out.try_reserve_exact(count).map_err(|_| PcmError::Budget)?;
        let mut remaining = pcm.samples();
        while !remaining.is_empty() {
            check(cancelled)?;
            let n = remaining.len().min(tile_samples - self.carry.len());
            self.carry.extend_from_slice(&remaining[..n]);
            remaining = &remaining[n..];
            if self.carry.len() == tile_samples {
                out.push(tile::emit(
                    &self.provisional_origin,
                    self.emitted,
                    self.limits.tile_frames,
                    channels,
                    &self.carry,
                    cancelled,
                )?);
                self.emitted += 1;
                self.carry.clear();
            }
        }
        check(cancelled)?;
        self.failed = false;
        Ok(out)
    }
    /// The SAME decoder must have reported a successful EOF. An I/O error,
    /// caller stop or cancellation is not EOF. Consumes the only continuation.
    pub fn finish_at_eof(self, cancelled: &dyn Fn() -> bool) -> Result<CompletedPcmStream> {
        check(cancelled)?;
        if self.failed {
            return Err(PcmError::Invalid("streaming PCM failed"));
        }
        let decoded = self.inspection.finish_at_eof(cancelled)?;
        let asset = decoded.asset();
        let binding = MediaBinding {
            resource: self.origin.resource,
            digest: self.origin.content,
            stream: self.origin.stream,
            descriptor: MediaStream::Audio {
                available: decoded.decoded_range(self.limits.inspection.exact_bits)?,
                sample_rate: asset.sample_rate,
                channels: asset.layout.len() as u16,
            },
        };
        let source = Arc::new(PcmSource::new(
            binding,
            asset.clone(),
            &self.origin.decoder_profile,
            self.limits.tile_frames,
            self.limits.inspection.exact_bits,
        )?);
        let count = self
            .emitted
            .checked_add(u32::from(!self.carry.is_empty()))
            .ok_or(PcmError::Budget)?;
        if count != source.tile_count() || count > self.limits.max_tiles {
            return Err(PcmError::Invalid("PCM stream extent"));
        }
        let charge = self
            .carry
            .capacity()
            .checked_add(self.carry.len())
            .and_then(|n| n.checked_mul(4))
            .ok_or(PcmError::Budget)?;
        if charge > self.limits.max_working_bytes {
            return Err(PcmError::Budget);
        }
        let tail = if self.carry.is_empty() {
            None
        } else {
            Some(tile::emit(
                &self.provisional_origin,
                self.emitted,
                self.limits.tile_frames,
                self.channels,
                &self.carry,
                cancelled,
            )?)
        };
        check(cancelled)?;
        Ok(CompletedPcmStream {
            decoded,
            source,
            tail,
            provisional_origin: self.provisional_origin,
            decoder_profile: self.origin.decoder_profile,
        })
    }
}
