//! Lossy platform timestamps are a transport, never the canonical PCM clock.
use crate::{
    CompletedPcmStream, PcmError, PcmStreamBuilder, PcmStreamLimits, PcmStreamOrigin, Result,
    UnsealedPcmTile, check,
};
use mo_media::{DecodedFrame, MediaKind};
use mo_time::{ExactRatio as R, Rounding};
use mo_video_audio::ByteOrder;

pub const QUANTIZED_PCM_PROFILE: &str = "musteroffice.pcm.microsecond-clock/1-draft";

/// Clock for a fresh continuous WebCodecs decode. The host submits packet PTS
/// relative to the first RAW packet, using `packet_timestamp`. AudioData PTS
/// stays untouched, including any codec-reported priming removal. No offset is
/// inferred from the first output, and no duration is accumulated in floats.
///
/// Up to one microsecond is admitted: one packet timestamp rounding plus one
/// codec trim rounding. At <=384 kHz this still identifies at most one sample.
/// This is an explicit adapter policy, not permission to repair discontinuity.
#[derive(Debug, Clone)]
pub struct MicrosecondPcmClock {
    origin: R,
    rate: u32,
    bits: u64,
}
impl MicrosecondPcmClock {
    pub fn new(raw_packet_origin: R, sample_rate: u32, exact_bits: u64) -> Result<Self> {
        if !(8000..=384000).contains(&sample_rate) || !(128..=4096).contains(&exact_bits) {
            return Err(PcmError::Invalid("platform PCM clock configuration"));
        }
        raw_packet_origin.check_budget(exact_bits)?;
        Ok(Self {
            origin: raw_packet_origin,
            rate: sample_rate,
            bits: exact_bits,
        })
    }
    pub fn sample_rate(&self) -> u32 {
        self.rate
    }
    /// Input packets must describe a sample lattice relative to the same raw
    /// anchor. A coarse/discontinuous container clock needs its own explicit
    /// adapter; silently shifting packet starts would hide a format defect.
    pub fn packet_timestamp(&self, raw_pts: &R) -> Result<i64> {
        let offset = raw_pts.sub(&self.origin, self.bits)?;
        let sample = offset.mul(&R::integer(i64::from(self.rate)), self.bits)?;
        let integer = sample.round_integer(Rounding::NearestTiesEven)?;
        if integer < 0 || sample.compare(&R::integer(integer)).is_ne() {
            return Err(PcmError::Invalid("packet PTS off raw PCM lattice"));
        }
        Ok(offset
            .mul(&R::integer(1_000_000), self.bits)?
            .round_integer(Rounding::NearestTiesEven)?)
    }
    /// Preserve the actual leading sample reported by the decoder. A browser
    /// that already discarded priming produces a later canonical sample zero;
    /// the movie mapper then verifies coverage instead of trimming it twice.
    pub fn decoded_timestamp(&self, microseconds: i64) -> Result<R> {
        if microseconds < 0 {
            return Err(PcmError::Invalid("negative platform PCM timestamp"));
        }
        let at = R::integer(microseconds).div(&R::integer(1_000_000), self.bits)?;
        let sample = at
            .mul(&R::integer(i64::from(self.rate)), self.bits)?
            .round_integer(Rounding::NearestTiesEven)?;
        let exact = R::integer(sample).div(&R::integer(i64::from(self.rate)), self.bits)?;
        let error = at.sub(&exact, self.bits)?;
        let bound = R::integer(1).div(&R::integer(1_000_000), self.bits)?;
        if error.compare(&bound).is_gt()
            || error
                .compare(&R::integer(0).sub(&bound, self.bits)?)
                .is_lt()
        {
            return Err(PcmError::Invalid(
                "platform PCM timestamp off sample lattice",
            ));
        }
        Ok(self.origin.add(&exact, self.bits)?)
    }
}

/// Same EOF sealing and tile identity as native PCM. Only the lossy transport
/// timestamp is replaced, after a checked conversion by the shared Rust core.
/// Any failed push poisons this object, including clock/format validation that
/// precedes the inner stream. A partial decode can never complete afterwards.
pub struct MicrosecondPcmStream {
    clock: MicrosecondPcmClock,
    stream: PcmStreamBuilder,
    failed: bool,
}
impl MicrosecondPcmStream {
    pub fn new(
        origin: PcmStreamOrigin,
        generation: u64,
        clock: MicrosecondPcmClock,
        limits: PcmStreamLimits,
    ) -> Result<Self> {
        if clock.bits != limits.inspection.exact_bits {
            return Err(PcmError::Invalid("platform PCM precision binding"));
        }
        Ok(Self {
            clock,
            stream: PcmStreamBuilder::new(origin, generation, limits)?,
            failed: false,
        })
    }
    pub fn retained_bytes(&self) -> usize {
        self.stream.retained_bytes()
    }
    pub fn push(
        &mut self,
        frame: &DecodedFrame,
        order: ByteOrder,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Vec<UnsealedPcmTile>> {
        if self.failed {
            return Err(PcmError::Invalid("platform PCM stream failed"));
        }
        self.failed = true;
        check(cancelled)?;
        let info = &frame.info;
        if info.kind != MediaKind::Audio
            || info.sample_rate != Some(self.clock.rate)
            || info.time_base.numerator != 1
            || info.time_base.denominator != 1_000_000
        {
            return Err(PcmError::Invalid("platform PCM output clock"));
        }
        let at = self.clock.decoded_timestamp(
            info.pts_ticks
                .ok_or(PcmError::Invalid("missing platform PCM PTS"))?,
        )?;
        let tiles = self.stream.push_at(frame, order, Some(at), cancelled)?;
        self.failed = false;
        Ok(tiles)
    }
    /// Host calls only after all raw packets and a successful decoder flush,
    /// with every output consumed and no earlier error/cancellation. Host I/O
    /// and atomic publication remain independent requirements.
    pub fn finish_at_eof(self, cancelled: &dyn Fn() -> bool) -> Result<CompletedPcmStream> {
        if self.failed {
            return Err(PcmError::Invalid("platform PCM stream failed"));
        }
        self.stream.finish_at_eof(cancelled)
    }
}
