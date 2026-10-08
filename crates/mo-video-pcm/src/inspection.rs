use crate::{PcmError, Result, check, clock::DecodedClock};
use mo_common::RationalTime;
use mo_media::DecodedFrame;
use mo_time::ExactRatio as R;
use mo_video_audio::{AudioAsset, ByteOrder, PcmBlock, PcmLimits, PcmRange};
use mo_video_model::TimeRange;

#[derive(Debug, Clone, Copy)]
pub struct PcmInspectLimits {
    /// Total frames returned by the decoder, not compressed packet count.
    pub max_decode_frames: u64,
    /// Total sample frames per channel; independent of sample rate.
    pub max_sample_frames: u64,
    pub exact_bits: u64,
    pub frame: PcmLimits,
}
impl Default for PcmInspectLimits {
    fn default() -> Self {
        Self {
            max_decode_frames: 10_000_000,
            max_sample_frames: 384000 * 60 * 60 * 24,
            exact_bits: 256,
            frame: PcmLimits::default(),
        }
    }
}
impl PcmInspectLimits {
    pub(crate) fn validate(&self) -> Result<()> {
        if self.max_decode_frames == 0
            || self.max_decode_frames > 100_000_000
            || self.max_sample_frames == 0
            || self.max_sample_frames > i64::MAX as u64
            || !(128..=4096).contains(&self.exact_bits)
        {
            return Err(PcmError::Invalid("PCM inspection limits"));
        }
        Ok(())
    }
}
/// Bounded validation of an actual continuous decode from its fresh start to
/// EOF. Retains no PCM. A seek, hole, overlap, changed format, invalid payload,
/// cancellation or exceeded budget poisons the scan, so a prefix cannot later
/// be mistaken for a complete source. The caller owns the decoder and read I/O.
pub struct PcmInspection {
    generation: u64,
    limits: PcmInspectLimits,
    clock: Option<DecodedClock>,
    frames: u64,
    samples: i64,
    failed: bool,
}
/// This attests decoded coverage, not audible/movie coverage. In particular,
/// raw AAC priming/padding may still be inside `asset.available`. Container
/// edits must be applied separately before creating a document resource.
#[derive(Debug, Clone)]
pub struct InspectedPcm {
    asset: AudioAsset,
    decode_frames: u64,
}
impl PcmInspection {
    pub fn new(generation: u64, limits: PcmInspectLimits) -> Result<Self> {
        if generation == 0 {
            return Err(PcmError::Invalid("decoder generation"));
        }
        limits.validate()?;
        Ok(Self {
            generation,
            limits,
            clock: None,
            frames: 0,
            samples: 0,
            failed: false,
        })
    }
    pub fn observe(
        &mut self,
        frame: &DecodedFrame,
        order: ByteOrder,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<()> {
        self.observe_pcm(frame, order, cancelled).map(drop)
    }
    /// Validate the next decoded frame and transfer its canonical PCM to the
    /// host in the same pass. No second normalization or retained sample copy.
    /// The block belongs to a provisional prefix, not a complete asset. Hosts
    /// must discard unpublished storage on decode/storage failure and publish
    /// completeness only after successful EOF and their own atomic commit.
    pub fn observe_pcm(
        &mut self,
        frame: &DecodedFrame,
        order: ByteOrder,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<PcmBlock> {
        self.observe_bounded(frame, order, self.limits.frame.max_bytes, cancelled)
    }
    /// The streaming assembler also charges its existing carry before allowing
    /// a normalized frame allocation. This can only tighten configured limits.
    pub(crate) fn observe_bounded(
        &mut self,
        frame: &DecodedFrame,
        order: ByteOrder,
        max_bytes: usize,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<PcmBlock> {
        self.observe_bounded_at(frame, order, max_bytes, None, cancelled)
    }
    pub(crate) fn observe_bounded_at(
        &mut self,
        frame: &DecodedFrame,
        order: ByteOrder,
        max_bytes: usize,
        at: Option<R>,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<PcmBlock> {
        if self.failed {
            return Err(PcmError::Invalid("PCM inspection failed"));
        }
        // Commit only after all metadata and owned sample bytes have passed.
        // A callback panic also leaves this object failed if a host catches it.
        self.failed = true;
        check(cancelled)?;
        if self.frames >= self.limits.max_decode_frames {
            return Err(PcmError::Budget);
        }
        let clock = match &self.clock {
            Some(clock) => clock.clone(),
            None => match &at {
                Some(at) => {
                    DecodedClock::from_first_at(&frame.info, at.clone(), self.limits.exact_bits)?
                }
                None => DecodedClock::from_first(&frame.info, self.limits.exact_bits)?,
            },
        };
        let limits = PcmLimits {
            max_bytes: self.limits.frame.max_bytes.min(max_bytes),
            ..self.limits.frame
        };
        let pcm = match at {
            Some(at) => clock.normalize_at(frame, order, self.generation, limits, at, cancelled)?,
            None => clock.normalize(frame, order, self.generation, limits, cancelled)?,
        };
        if pcm.first_frame() != self.samples {
            return Err(PcmError::Invalid("discontinuous PCM decode"));
        }
        if pcm.end_frame() as u64 > self.limits.max_sample_frames {
            return Err(PcmError::Budget);
        }
        check(cancelled)?;
        self.samples = pcm.end_frame();
        self.frames += 1;
        self.clock = Some(clock);
        self.failed = false;
        Ok(pcm)
    }
    /// Call only after the same decoder reports successful EOF. A transport
    /// error, task cancellation or host early stop is not EOF. Consuming `self`
    /// prevents observations after finalization or a second completion receipt.
    pub fn finish_at_eof(self, cancelled: &dyn Fn() -> bool) -> Result<InspectedPcm> {
        check(cancelled)?;
        if self.failed {
            return Err(PcmError::Invalid("PCM inspection failed"));
        }
        let clock = self.clock.ok_or(PcmError::Invalid("empty PCM decode"))?;
        Ok(InspectedPcm {
            asset: AudioAsset {
                sample_rate: clock.sample_rate,
                layout: clock.layout,
                sample_zero: clock.sample_zero,
                available: PcmRange::new(0, self.samples)?,
            },
            decode_frames: self.frames,
        })
    }
}
impl InspectedPcm {
    /// Private reconstruction path for an authenticated host completion record;
    /// this validates structural coverage, not provenance of the host's bytes.
    pub(crate) fn restore(
        asset: AudioAsset,
        decode_frames: u64,
        limits: PcmInspectLimits,
    ) -> Result<Self> {
        limits.validate()?;
        if !(8000..=384000).contains(&asset.sample_rate)
            || asset.available.first != 0
            || asset.available.end <= 0
            || asset.available.end as u64 > limits.max_sample_frames
            || decode_frames == 0
            || decode_frames > limits.max_decode_frames
            || decode_frames > asset.available.end as u64
        {
            return Err(PcmError::Invalid("PCM completion record extent"));
        }
        asset.sample_zero.check_budget(limits.exact_bits)?;
        let value = Self {
            asset,
            decode_frames,
        };
        value.decoded_range(limits.exact_bits)?;
        Ok(value)
    }
    pub fn asset(&self) -> &AudioAsset {
        &self.asset
    }
    pub fn decode_frames(&self) -> u64 {
        self.decode_frames
    }
    /// Exact raw decoded coverage, with no accumulated float duration or
    /// nominal packet clock. Failure to fit document time is never rounded.
    pub fn decoded_range(&self, bits: u64) -> Result<TimeRange> {
        let duration = R::integer(self.asset.available.end)
            .div(&R::integer(i64::from(self.asset.sample_rate)), bits)?;
        Ok(TimeRange {
            start: self.asset.sample_zero.to_time_exact()?,
            end: self
                .asset
                .sample_zero
                .add(&duration, bits)?
                .to_time_exact()?,
        })
    }
    /// Binds only an explicitly requested continuous subsection of the decoded
    /// lattice. This does not choose container edit semantics on the host's
    /// behalf. It never fabricates missing samples or silence outside coverage.
    pub fn continuous_asset(&self, range: TimeRange, bits: u64) -> Result<AudioAsset> {
        use mo_time::Rounding;
        if !range.valid() {
            return Err(PcmError::Invalid("PCM inspected binding range"));
        }
        let index = |time: RationalTime| -> Result<i64> {
            Ok(R::from_time(time)
                .sub(&self.asset.sample_zero, bits)?
                .mul(&R::integer(i64::from(self.asset.sample_rate)), bits)?
                .round_integer(Rounding::Ceil)?)
        };
        let decoded_end = self.asset.sample_zero.add(
            &R::integer(self.asset.available.end)
                .div(&R::integer(i64::from(self.asset.sample_rate)), bits)?,
            bits,
        )?;
        if R::from_time(range.start)
            .compare(&self.asset.sample_zero)
            .is_lt()
            || R::from_time(range.end).compare(&decoded_end).is_gt()
        {
            return Err(PcmError::Invalid("PCM binding outside decoded coverage"));
        }
        let first = index(range.start)?;
        let end = index(range.end)?;
        if first < 0 || end > self.asset.available.end || first >= end {
            return Err(PcmError::Invalid("PCM binding outside decoded coverage"));
        }
        Ok(AudioAsset {
            available: PcmRange::new(first, end)?,
            ..self.asset.clone()
        })
    }
}
