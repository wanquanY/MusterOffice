use crate::{AudioError, ChannelLayout, ChannelMatrix, PcmBlock, PcmLimits, PcmRange, Result};
use mo_time::{ExactRatio as R, Rounding};
use mo_video_compile::MediaBinding;
use mo_video_model::MediaStream;
/// Describes a continuous, already trimmed PCM lattice. The host resolves
/// codec priming/edit lists before binding it; no inferred timestamps or holes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioAsset {
    pub sample_rate: u32,
    pub layout: ChannelLayout,
    /// Source-document time of PCM sample index zero (can be negative).
    pub sample_zero: R,
    pub available: PcmRange,
}
impl AudioAsset {
    pub fn validate_binding(&self, binding: &MediaBinding, bits: u64) -> Result<()> {
        let MediaStream::Audio {
            sample_rate,
            channels,
            available,
        } = &binding.descriptor
        else {
            return Err(AudioError::Invalid("audio resource descriptor"));
        };
        if !available.valid() || !(8000..=384000).contains(sample_rate) {
            return Err(AudioError::Invalid("audio resource rate/span"));
        }
        if *sample_rate != self.sample_rate || usize::from(*channels) != self.layout.len() {
            return Err(AudioError::Invalid("audio resource metadata binding"));
        }
        PcmRange::new(self.available.first, self.available.end)?;
        let sr = R::integer(i64::from(self.sample_rate));
        let first = R::from_time(available.start)
            .sub(&self.sample_zero, bits)?
            .mul(&sr, bits)?
            .round_integer(Rounding::Ceil)?;
        let end = R::from_time(available.end)
            .sub(&self.sample_zero, bits)?
            .mul(&sr, bits)?
            .round_integer(Rounding::Ceil)?;
        if self.available != PcmRange::new(first, end)? {
            return Err(AudioError::Invalid(
                "audio resource continuous span binding",
            ));
        }
        Ok(())
    }
}
/// Host capabilities, not paths, URLs, device handles or implicit native codecs.
/// Implementations must verify immutable digest/stream binding and honor limits
/// and cancellation. The core validates every returned PCM window as well.
pub trait AudioResources {
    /// Identity of all output-affecting host bindings: decoded PCM profile,
    /// channel matrices and actual DSP numerical build. No filename/version guess.
    fn retime_profile(&mut self) -> Result<mo_common::Digest> {
        Err(AudioError::Unsupported("non-unit preserve-pitch DSP"))
    }
    fn read_retimed(
        &mut self,
        request: &crate::RetimeRequest,
        range: PcmRange,
        _limits: PcmLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<PcmBlock> {
        crate::cancel(check)?;
        Err(AudioError::PendingDerived {
            identity: request.identity().clone(),
            first_sample: range.first,
            end_sample: range.end,
        })
    }
    fn describe(&mut self, binding: &MediaBinding) -> Result<AudioAsset>;
    fn read(
        &mut self,
        binding: &MediaBinding,
        asset: &AudioAsset,
        range: PcmRange,
        limits: PcmLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<PcmBlock>;
    fn channel_matrix(
        &mut self,
        input: &ChannelLayout,
        output: &ChannelLayout,
    ) -> Result<ChannelMatrix> {
        if input == output {
            Ok(ChannelMatrix::identity(input.clone()))
        } else {
            Err(AudioError::Unsupported("explicit channel mapping required"))
        }
    }
}
