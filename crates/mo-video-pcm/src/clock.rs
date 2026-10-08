use crate::{PcmError, Result, check};
use mo_media::{DecodedFrame, FrameInfo, MediaKind};
use mo_time::{ExactRatio as R, Rounding};
use mo_video_audio::{AudioAsset, ByteOrder, ChannelLayout, PcmBlock, PcmLimits, normalize_pcm};

/// Exact clock of one decoded PCM lattice. This does not interpret container
/// edits, codec delay or padding; those must retain their separate evidence.
#[derive(Debug, Clone)]
pub(crate) struct DecodedClock {
    pub sample_rate: u32,
    pub layout: ChannelLayout,
    pub sample_zero: R,
    pub bits: u64,
}
impl DecodedClock {
    pub fn from_first(info: &FrameInfo, bits: u64) -> Result<Self> {
        Self::from_first_at(info, timestamp(info, bits)?, bits)
    }
    pub fn from_first_at(info: &FrameInfo, sample_zero: R, bits: u64) -> Result<Self> {
        sample_zero.check_budget(bits)?;
        if info.kind != MediaKind::Audio {
            return Err(PcmError::Invalid("decoded PCM media kind"));
        }
        let sample_rate = info
            .sample_rate
            .filter(|v| (8000..=384000).contains(v))
            .ok_or(PcmError::Invalid("decoded PCM sample rate"))?;
        Ok(Self {
            sample_rate,
            layout: ChannelLayout::from_codec_name(&info.channel_layout)?,
            sample_zero,
            bits,
        })
    }
    pub fn from_asset(asset: &AudioAsset, bits: u64) -> Self {
        Self {
            sample_rate: asset.sample_rate,
            layout: asset.layout.clone(),
            sample_zero: asset.sample_zero.clone(),
            bits,
        }
    }
    pub fn normalize(
        &self,
        frame: &DecodedFrame,
        order: ByteOrder,
        generation: u64,
        limits: PcmLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<PcmBlock> {
        self.normalize_at(
            frame,
            order,
            generation,
            limits,
            timestamp(&frame.info, self.bits)?,
            cancelled,
        )
    }
    pub fn normalize_at(
        &self,
        frame: &DecodedFrame,
        order: ByteOrder,
        generation: u64,
        limits: PcmLimits,
        at: R,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<PcmBlock> {
        check(cancelled)?;
        if generation == 0 || frame.info.generation != generation {
            return Err(PcmError::Invalid("decoder generation"));
        }
        if frame.info.sample_rate != Some(self.sample_rate)
            || ChannelLayout::from_codec_name(&frame.info.channel_layout)? != self.layout
        {
            return Err(PcmError::Invalid("decoded PCM binding"));
        }
        let position = at
            .sub(&self.sample_zero, self.bits)?
            .mul(&R::integer(i64::from(self.sample_rate)), self.bits)?;
        let first = position.round_integer(Rounding::Floor)?;
        if position.compare(&R::integer(first)).is_ne() {
            return Err(PcmError::Invalid("PCM timestamp off sample lattice"));
        }
        Ok(normalize_pcm(
            frame,
            order,
            self.layout.clone(),
            first,
            limits,
            cancelled,
        )?)
    }
}
fn timestamp(info: &FrameInfo, bits: u64) -> Result<R> {
    if info.kind != MediaKind::Audio {
        return Err(PcmError::Invalid("decoded PCM media kind"));
    }
    let base = info.time_base;
    if base.numerator == 0 || base.denominator == 0 {
        return Err(PcmError::Invalid("decoded PCM timebase"));
    }
    Ok(
        R::integer(info.pts_ticks.ok_or(PcmError::Invalid("missing PCM PTS"))?)
            .mul(&R::integer(i64::from(base.numerator)), bits)?
            .div(&R::integer(i64::from(base.denominator)), bits)?,
    )
}
