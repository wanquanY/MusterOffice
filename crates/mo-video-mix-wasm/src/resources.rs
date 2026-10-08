use js_sys::{Function, Uint8Array};
use mo_common::Ticks;
use mo_video_audio::{
    AudioAsset, AudioError, AudioResourceDescription, AudioResources, ChannelLayout, ChannelMatrix,
    PcmBlock, PcmLimits, PcmRange, Result,
};
use mo_video_compile::MediaBinding;
use wasm_bindgen::{JsCast, JsValue};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Pending {
    binding: MediaBinding,
    first_sample: Option<Ticks>,
    end_sample: Option<Ticks>,
}
pub(crate) struct Resources {
    describe: Function,
    read: Function,
    pub pending: Option<Pending>,
}
impl Resources {
    pub fn new(describe: Function, read: Function) -> Self {
        Self {
            describe,
            read,
            pending: None,
        }
    }
    fn missing(&mut self, binding: &MediaBinding, range: Option<PcmRange>) -> AudioError {
        self.pending = Some(Pending {
            binding: binding.clone(),
            first_sample: range.map(|r| Ticks::new(r.first)),
            end_sample: range.map(|r| Ticks::new(r.end)),
        });
        AudioError::PendingSource {
            resource: binding.resource.clone(),
            first_sample: range.map_or(0, |r| r.first),
            end_sample: range.map_or(0, |r| r.end),
        }
    }
}
impl AudioResources for Resources {
    fn describe(&mut self, binding: &MediaBinding) -> Result<AudioAsset> {
        let text =
            serde_json::to_string(binding).map_err(|_| AudioError::Invalid("audio binding"))?;
        let value = self
            .describe
            .call1(&JsValue::UNDEFINED, &JsValue::from_str(&text))
            .map_err(|_| AudioError::Resource("host describe failed".into()))?;
        if value.is_null() || value.is_undefined() {
            return Err(self.missing(binding, None));
        }
        let text = value
            .as_string()
            .ok_or(AudioError::Invalid("audio description type"))?;
        if text.len() > 16384 {
            return Err(AudioError::Budget);
        }
        let description: AudioResourceDescription = mo_common::from_json_str_with_path(&text)
            .map_err(|_| AudioError::Invalid("audio description"))?;
        description.asset(binding, 256)
    }
    fn read(
        &mut self,
        binding: &MediaBinding,
        asset: &AudioAsset,
        range: PcmRange,
        limits: PcmLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<PcmBlock> {
        if check() {
            return Err(AudioError::Cancelled);
        }
        let frames = usize::try_from(
            range
                .end
                .checked_sub(range.first)
                .ok_or(AudioError::Budget)?,
        )
        .map_err(|_| AudioError::Budget)?;
        let bytes = frames
            .checked_mul(asset.layout.len())
            .and_then(|n| n.checked_mul(4))
            .ok_or(AudioError::Budget)?;
        if frames > limits.max_frames || bytes > limits.max_bytes {
            return Err(AudioError::Budget);
        }
        let text = serde_json::json!({ "binding": binding, "firstSample": Ticks::new(range.first),
            "endSample": Ticks::new(range.end), "frames": frames, "maxBytes": bytes })
        .to_string();
        let value = self
            .read
            .call1(&JsValue::UNDEFINED, &JsValue::from_str(&text))
            .map_err(|_| AudioError::Resource("host audio read failed".into()))?;
        if value.is_null() || value.is_undefined() {
            return Err(self.missing(binding, Some(range)));
        }
        let array = value
            .dyn_into::<Uint8Array>()
            .map_err(|_| AudioError::Invalid("PCM window type"))?;
        // Inspect the actual transport length before copying any host bytes.
        if array.length() as usize != bytes {
            return Err(AudioError::Invalid("PCM window length"));
        }
        let samples = array
            .to_vec()
            .chunks_exact(4)
            .map(|v| f32::from_le_bytes([v[0], v[1], v[2], v[3]]))
            .collect();
        PcmBlock::new(
            asset.sample_rate,
            asset.layout.clone(),
            range.first,
            samples,
            limits,
            check,
        )
    }
    fn channel_matrix(
        &mut self,
        input: &ChannelLayout,
        output: &ChannelLayout,
    ) -> Result<ChannelMatrix> {
        if input == output {
            Ok(ChannelMatrix::identity(input.clone()))
        } else if input == &ChannelLayout::mono() && output == &ChannelLayout::stereo() {
            Ok(ChannelMatrix::mono_to_stereo())
        } else {
            Err(AudioError::Unsupported("explicit channel mapping required"))
        }
    }
}
