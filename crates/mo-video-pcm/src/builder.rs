use crate::{PcmError, PcmSource, PcmTile, PcmTileRef, Result, bytes, check};
use mo_common::{ByteLength, Digest, Ticks};
use mo_video_audio::PcmBlock;
use sha2::{Digest as _, Sha256};
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub struct PcmBuildLimits {
    /// New candidate payload: its carry capacity plus all encoded output tiles.
    /// Caller-owned input, old builder and host storage allocations are separate.
    pub max_candidate_bytes: usize,
    pub max_tiles_per_step: usize,
}
impl Default for PcmBuildLimits {
    fn default() -> Self {
        Self {
            max_candidate_bytes: 32 << 20,
            max_tiles_per_step: 1024,
        }
    }
}
#[derive(Debug)]
pub struct PcmTileBuilder {
    source: Arc<PcmSource>,
    raw_end: Option<i64>,
    next_tile: u32,
    carry: Vec<f32>,
    complete: bool,
}
#[derive(Debug)]
pub struct PreparedPcmTiles {
    /// Host atomically publishes these immutable blobs/references before adopting
    /// `next`. On storage failure, discard the candidate; the original is intact.
    pub tiles: Vec<PcmTile>,
    pub next: PcmTileBuilder,
}
impl PcmTileBuilder {
    pub fn new(source: Arc<PcmSource>) -> Self {
        Self {
            source,
            raw_end: None,
            next_tile: 0,
            carry: vec![],
            complete: false,
        }
    }
    pub fn source(&self) -> &PcmSource {
        &self.source
    }
    pub fn complete(&self) -> bool {
        self.complete
    }
    pub fn completed_tiles(&self) -> u32 {
        self.next_tile
    }
    pub fn retained_bytes(&self) -> usize {
        self.carry.capacity() * 4
    }
    /// A fresh continuous decode prefix is required. Published tile boundaries
    /// are independent of codec frame sizes and host scheduling block sizes.
    pub fn prepare(
        &self,
        input: &PcmBlock,
        limits: PcmBuildLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<PreparedPcmTiles> {
        check(cancelled)?;
        if self.complete {
            return Err(PcmError::Invalid("PCM source already complete"));
        }
        let asset = self.source.asset();
        if input.frames() == 0
            || input.sample_rate() != asset.sample_rate
            || input.layout() != &asset.layout
        {
            return Err(PcmError::Invalid("PCM builder input binding"));
        }
        if let Some(end) = self.raw_end {
            if input.first_frame() != end {
                return Err(PcmError::Invalid("discontinuous PCM decode"));
            }
        } else if input.first_frame() > asset.available.first {
            return Err(PcmError::Invalid("missing decoded prefix"));
        }
        let first = input.first_frame().max(asset.available.first);
        let end = input.end_frame().min(asset.available.end).max(first);
        let frames = usize::try_from(end - first).map_err(|_| PcmError::Budget)?;
        let samples = frames
            .checked_mul(asset.layout.len())
            .ok_or(PcmError::Budget)?;
        let tile_samples = self.source.tile_frames() as usize * asset.layout.len();
        let pending = self
            .carry
            .len()
            .checked_add(samples)
            .ok_or(PcmError::Budget)?;
        let finishing = input.end_frame() >= asset.available.end;
        let tiles = pending / tile_samples + usize::from(finishing && pending % tile_samples != 0);
        let encoded = if finishing {
            pending
        } else {
            pending / tile_samples * tile_samples
        };
        let charge = encoded
            .checked_add(tile_samples)
            .and_then(|n| n.checked_mul(4))
            .ok_or(PcmError::Budget)?;
        if tiles > limits.max_tiles_per_step
            || tiles > 65536
            || charge > limits.max_candidate_bytes
            || charge > 256 << 20
        {
            return Err(PcmError::Budget);
        }
        let mut carry = Vec::new();
        carry
            .try_reserve_exact(tile_samples)
            .map_err(|_| PcmError::Budget)?;
        carry.extend_from_slice(&self.carry);
        let mut next = Self {
            source: Arc::clone(&self.source),
            raw_end: Some(input.end_frame()),
            next_tile: self.next_tile,
            carry,
            complete: false,
        };
        let mut output = Vec::new();
        output
            .try_reserve_exact(tiles)
            .map_err(|_| PcmError::Budget)?;
        if samples != 0 {
            let offset = (first - input.first_frame()) as usize * asset.layout.len();
            let mut remaining = &input.samples()[offset..offset + samples];
            while !remaining.is_empty() {
                check(cancelled)?;
                let n = remaining.len().min(tile_samples - next.carry.len());
                next.carry.extend_from_slice(&remaining[..n]);
                remaining = &remaining[n..];
                if next.carry.len() == tile_samples {
                    output.push(next.take_tile(cancelled)?);
                }
            }
        }
        if finishing {
            if !next.carry.is_empty() {
                output.push(next.take_tile(cancelled)?);
            }
            if next.next_tile != self.source.tile_count() {
                return Err(PcmError::Invalid("incomplete decoded PCM range"));
            }
            next.carry = vec![];
            next.complete = true;
        }
        check(cancelled)?;
        Ok(PreparedPcmTiles {
            tiles: output,
            next,
        })
    }
    fn take_tile(&mut self, cancelled: &dyn Fn() -> bool) -> Result<PcmTile> {
        let (first, frames) = self.source.tile_window(self.next_tile)?;
        if self.carry.len() != frames as usize * self.source.asset().layout.len() {
            return Err(PcmError::Invalid("PCM tile assembly length"));
        }
        let mut data = bytes(self.carry.len() * 4)?;
        let mut hash = Sha256::new();
        for chunk in self.carry.chunks(4096) {
            check(cancelled)?;
            let begin = data.len();
            for value in chunk {
                data.extend_from_slice(&value.to_le_bytes());
            }
            hash.update(&data[begin..]);
        }
        let sha256 = Digest::from_sha256(hash.finalize().into());
        let reference = PcmTileRef {
            source: self.source.identity().clone(),
            index: self.next_tile,
            first_sample: Ticks::new(first),
            frames,
            sha256,
            byte_length: ByteLength::new(data.len() as u64),
        };
        self.next_tile += 1;
        self.carry.clear();
        Ok(PcmTile {
            reference,
            bytes: data,
        })
    }
    /// EOF before all attested source samples exist is an error, not tail silence.
    pub fn verify_eof(&self) -> Result<()> {
        if self.complete {
            Ok(())
        } else {
            Err(PcmError::Invalid("EOF before declared PCM end"))
        }
    }
}
