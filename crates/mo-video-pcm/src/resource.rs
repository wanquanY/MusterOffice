use crate::{PcmError, PcmReadLimits, PcmSource, PcmTileReader, PcmTileStore, Result};
use mo_video_audio::{AudioAsset, AudioError, AudioResources, PcmBlock, PcmLimits, PcmRange};
use mo_video_compile::MediaBinding;
use std::sync::Arc;
/// A single authorized resource capability. Hosts compose these into their
/// multi-resource bank and own permission leases, cache storage and scheduling.
pub struct CanonicalAudioResource<S> {
    reader: PcmTileReader,
    store: S,
}
impl<S: PcmTileStore> CanonicalAudioResource<S> {
    pub fn new(source: Arc<PcmSource>, store: S, limits: PcmReadLimits) -> Result<Self> {
        Ok(Self {
            reader: PcmTileReader::new(source, limits)?,
            store,
        })
    }
    pub fn source(&self) -> &PcmSource {
        self.reader.source()
    }
    pub fn store_mut(&mut self) -> &mut S {
        &mut self.store
    }
    pub fn retained_bytes(&self) -> usize {
        self.reader.retained_bytes()
    }
    pub fn clear(&mut self) {
        self.reader.clear();
    }
}
impl<S: PcmTileStore> AudioResources for CanonicalAudioResource<S> {
    fn describe(&mut self, binding: &MediaBinding) -> mo_video_audio::Result<AudioAsset> {
        if binding != self.source().binding() {
            return Err(AudioError::Invalid("changed canonical PCM binding"));
        }
        Ok(self.source().asset().clone())
    }
    fn read(
        &mut self,
        binding: &MediaBinding,
        asset: &AudioAsset,
        range: PcmRange,
        limits: PcmLimits,
        check: &dyn Fn() -> bool,
    ) -> mo_video_audio::Result<PcmBlock> {
        if asset != self.source().asset() {
            return Err(AudioError::Invalid("changed canonical PCM metadata"));
        }
        let frames = usize::try_from(i128::from(range.end) - i128::from(range.first))
            .map_err(|_| AudioError::Invalid("canonical PCM range"))?;
        if frames > limits.max_frames
            || frames
                .checked_mul(asset.layout.len())
                .and_then(|n| n.checked_mul(4))
                .is_none_or(|n| n > limits.max_bytes)
        {
            return Err(AudioError::Budget);
        }
        match self.reader.read(binding, range, &mut self.store, check) {
            Ok(read) => Ok(read.pcm),
            Err(PcmError::Missing { .. }) => Err(AudioError::PendingSource {
                resource: binding.resource.clone(),
                first_sample: range.first,
                end_sample: range.end,
            }),
            Err(PcmError::Audio(e)) => Err(e),
            Err(PcmError::Cancelled) => Err(AudioError::Cancelled),
            Err(PcmError::Budget) => Err(AudioError::Budget),
            Err(PcmError::Invalid(reason)) => Err(AudioError::Invalid(reason)),
            Err(PcmError::Time(e)) => Err(AudioError::Time(e)),
            Err(e) => Err(AudioError::Resource(e.to_string())),
        }
    }
}
