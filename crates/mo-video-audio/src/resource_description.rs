//! Exact resource metadata across a computation boundary. This is a description,
//! never authorization: the host must grant the underlying immutable resource.
use crate::{AudioAsset, AudioError, Channel, ChannelLayout, PcmRange, Result};
use mo_common::Ticks;
use mo_time::ExactRatio;
use mo_video_compile::MediaBinding;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioResourceDescription {
    pub binding: MediaBinding,
    pub sample_rate: u32,
    pub layout: Vec<Channel>,
    pub sample_zero: ExactRatio,
    pub first_sample: Ticks,
    pub end_sample: Ticks,
}
impl AudioResourceDescription {
    pub fn new(binding: MediaBinding, asset: &AudioAsset, bits: u64) -> Result<Self> {
        asset.sample_zero.check_budget(bits)?;
        asset.validate_binding(&binding, bits)?;
        Ok(Self {
            binding,
            sample_rate: asset.sample_rate,
            layout: asset.layout.channels().to_vec(),
            sample_zero: asset.sample_zero.clone(),
            first_sample: Ticks::new(asset.available.first),
            end_sample: Ticks::new(asset.available.end),
        })
    }
    pub fn asset(&self, expected: &MediaBinding, bits: u64) -> Result<AudioAsset> {
        if &self.binding != expected {
            return Err(AudioError::Invalid("audio description binding"));
        }
        self.sample_zero.check_budget(bits)?;
        let asset = AudioAsset {
            sample_rate: self.sample_rate,
            layout: ChannelLayout::new(self.layout.clone())?,
            sample_zero: self.sample_zero.clone(),
            available: PcmRange::new(self.first_sample.get(), self.end_sample.get())?,
        };
        asset.validate_binding(expected, bits)?;
        Ok(asset)
    }
}
