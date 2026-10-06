use crate::{PROFILE, PcmError, PcmTileRef, Result};
use mo_common::{Digest, Ticks, digest};
use mo_time::{ExactRatio as R, Rounding};
use mo_video_audio::{AudioAsset, ByteOrder, Channel, PcmBlock, PcmLimits, normalize_pcm};
use mo_video_compile::MediaBinding;
use serde::Serialize;

/// Host binds this identity to a fresh continuous decoder run, including the
/// pinned decoder build and its output-affecting options. Seeking into a lossy
/// stream must not be used to independently synthesize canonical tiles.
#[derive(Debug, Clone)]
pub struct PcmSource {
    binding: MediaBinding,
    asset: AudioAsset,
    identity: Digest,
    tile_frames: u32,
    tile_count: u32,
    bits: u64,
}
impl PcmSource {
    pub fn new(
        binding: MediaBinding,
        asset: AudioAsset,
        decoder_profile: &Digest,
        tile_frames: u32,
        bits: u64,
    ) -> Result<Self> {
        if !(256..=65536).contains(&tile_frames) || !(128..=4096).contains(&bits) {
            return Err(PcmError::Invalid("source limits"));
        }
        asset.validate_binding(&binding, bits)?;
        let frames = i128::from(asset.available.end) - i128::from(asset.available.first);
        if frames <= 0 {
            return Err(PcmError::Invalid("empty PCM source"));
        }
        let tile_count =
            u32::try_from((frames + i128::from(tile_frames) - 1) / i128::from(tile_frames))
                .map_err(|_| PcmError::Budget)?;
        // Resource IDs are document-local. Content digest + stream/descriptor and
        // processing profile permit safe cross-project byte deduplication.
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Parameters<'a> {
            binding_digest: &'a Digest,
            stream: &'a str,
            descriptor: &'a mo_video_model::MediaStream,
            decoder_profile: &'a Digest,
            layout: &'a [Channel],
            sample_zero: R,
            first: Ticks,
            end: Ticks,
            tile_frames: u32,
        }
        let identity = digest(
            PROFILE,
            &Parameters {
                binding_digest: &binding.digest,
                stream: binding.stream.as_str(),
                descriptor: &binding.descriptor,
                decoder_profile,
                layout: asset.layout.channels(),
                sample_zero: asset.sample_zero.normalized(),
                first: Ticks::new(asset.available.first),
                end: Ticks::new(asset.available.end),
                tile_frames,
            },
        )?;
        Ok(Self {
            binding,
            asset,
            identity,
            tile_frames,
            tile_count,
            bits,
        })
    }
    pub fn binding(&self) -> &MediaBinding {
        &self.binding
    }
    pub fn asset(&self) -> &AudioAsset {
        &self.asset
    }
    pub fn identity(&self) -> &Digest {
        &self.identity
    }
    pub fn tile_frames(&self) -> u32 {
        self.tile_frames
    }
    pub fn tile_count(&self) -> u32 {
        self.tile_count
    }
    pub fn tile_window(&self, index: u32) -> Result<(i64, u32)> {
        if index >= self.tile_count {
            return Err(PcmError::Invalid("tile index"));
        }
        let first = i128::from(self.asset.available.first)
            + i128::from(index) * i128::from(self.tile_frames);
        let frames =
            (i128::from(self.asset.available.end) - first).min(i128::from(self.tile_frames));
        Ok((
            i64::try_from(first).map_err(|_| PcmError::Budget)?,
            frames as u32,
        ))
    }
    pub fn tile_index(&self, sample: i64) -> Result<u32> {
        if sample < self.asset.available.first || sample >= self.asset.available.end {
            return Err(PcmError::Invalid("sample outside PCM source"));
        }
        u32::try_from(
            (i128::from(sample) - i128::from(self.asset.available.first))
                / i128::from(self.tile_frames),
        )
        .map_err(|_| PcmError::Budget)
    }
    pub fn validate_ref(&self, reference: &PcmTileRef) -> Result<()> {
        let (first, frames) = self.tile_window(reference.index)?;
        let expected = u64::from(frames) * self.asset.layout.len() as u64 * 4;
        if &reference.source != self.identity()
            || reference.first_sample.get() != first
            || reference.frames != frames
            || reference.byte_length.get() != expected
        {
            return Err(PcmError::Invalid("PCM tile reference binding"));
        }
        Ok(())
    }
    /// Actual frame PTS must fall exactly on the declared source lattice. No
    /// duration-based accumulation, timestamp repair or implicit rate change.
    pub fn normalize(
        &self,
        frame: &mo_media::DecodedFrame,
        order: ByteOrder,
        generation: u64,
        limits: PcmLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<PcmBlock> {
        if generation == 0 || frame.info.generation != generation {
            return Err(PcmError::Invalid("decoder generation"));
        }
        if frame.info.sample_rate != Some(self.asset.sample_rate)
            || mo_video_audio::ChannelLayout::from_codec_name(&frame.info.channel_layout)?
                != self.asset.layout
        {
            return Err(PcmError::Invalid("decoded PCM binding"));
        }
        let base = frame.info.time_base;
        if base.numerator == 0 || base.denominator == 0 {
            return Err(PcmError::Invalid("decoded PCM timebase"));
        }
        let time = R::integer(
            frame
                .info
                .pts_ticks
                .ok_or(PcmError::Invalid("missing PCM PTS"))?,
        )
        .mul(&R::integer(i64::from(base.numerator)), self.bits)?
        .div(&R::integer(i64::from(base.denominator)), self.bits)?;
        let position = time
            .sub(&self.asset.sample_zero, self.bits)?
            .mul(&R::integer(i64::from(self.asset.sample_rate)), self.bits)?;
        let first = position.round_integer(Rounding::Floor)?;
        if position.compare(&R::integer(first)).is_ne() {
            return Err(PcmError::Invalid("PCM timestamp off sample lattice"));
        }
        Ok(normalize_pcm(
            frame,
            order,
            self.asset.layout.clone(),
            first,
            limits,
            cancelled,
        )?)
    }
}
