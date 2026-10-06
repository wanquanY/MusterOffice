use crate::{AudioError, ChannelLayout, ChannelMatrix, Result, cancel};
use mo_media::{DecodedFrame, MediaKind};

#[derive(Debug, Clone, Copy)]
pub struct PcmLimits {
    pub max_frames: usize,
    pub max_bytes: usize,
}
impl Default for PcmLimits {
    fn default() -> Self {
        Self {
            max_frames: 2_000_000,
            max_bytes: 256 << 20,
        }
    }
}
#[derive(Debug)]
pub struct PcmBlock {
    pub(crate) sample_rate: u32,
    pub(crate) layout: ChannelLayout,
    pub(crate) first_frame: i64,
    pub(crate) samples: Vec<f32>,
}
impl PcmBlock {
    pub fn new(
        sample_rate: u32,
        layout: ChannelLayout,
        first_frame: i64,
        samples: Vec<f32>,
        limits: PcmLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self> {
        let count = admit(sample_rate, &layout, samples.len() / layout.len(), limits)?;
        if samples.len() != count {
            return Err(AudioError::Invalid("PCM sample length"));
        }
        first_frame
            .checked_add((samples.len() / layout.len()) as i64)
            .ok_or(AudioError::Budget)?;
        for chunk in samples.chunks(4096) {
            cancel(check)?;
            if chunk.iter().any(|v| !v.is_finite()) {
                return Err(AudioError::Invalid("non-finite PCM"));
            }
        }
        cancel(check)?;
        Ok(Self {
            sample_rate,
            layout,
            first_frame,
            samples,
        })
    }
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }
    pub fn frames(&self) -> usize {
        self.samples.len() / self.layout.len()
    }
    pub fn first_frame(&self) -> i64 {
        self.first_frame
    }
    pub fn end_frame(&self) -> i64 {
        self.first_frame + self.frames() as i64
    }
    pub fn layout(&self) -> &ChannelLayout {
        &self.layout
    }
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    pub fn bytes(&self) -> usize {
        self.samples.len() * 4
    }
    pub fn map_channels(
        &self,
        matrix: &ChannelMatrix,
        limits: PcmLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self> {
        if self.layout != matrix.input {
            return Err(AudioError::Invalid("channel matrix input binding"));
        }
        let count = admit(self.sample_rate, &matrix.output, self.frames(), limits)?;
        let mut samples = allocate(count)?;
        for (index, frame) in self.samples.chunks_exact(self.layout.len()).enumerate() {
            if index % 128 == 0 {
                cancel(check)?;
            }
            for (out, gains) in matrix.gains.chunks_exact(self.layout.len()).enumerate() {
                let value = frame
                    .iter()
                    .zip(gains)
                    .map(|(v, g)| f64::from(*v) * g)
                    .sum::<f64>() as f32;
                if !value.is_finite() {
                    return Err(AudioError::Invalid("channel mix overflow"));
                }
                samples[index * matrix.output.len() + out] = value;
            }
        }
        Self::new(
            self.sample_rate,
            matrix.output.clone(),
            self.first_frame,
            samples,
            limits,
            check,
        )
    }
}
pub(crate) fn admit(
    rate: u32,
    layout: &ChannelLayout,
    frames: usize,
    limits: PcmLimits,
) -> Result<usize> {
    if !(8000..=384000).contains(&rate) {
        return Err(AudioError::Invalid("PCM sample rate"));
    }
    let count = frames.checked_mul(layout.len()).ok_or(AudioError::Budget)?;
    if frames > limits.max_frames
        || count
            .checked_mul(4)
            .is_none_or(|v| v > limits.max_bytes || v > 1 << 30)
    {
        return Err(AudioError::Budget);
    }
    Ok(count)
}
pub(crate) fn allocate(count: usize) -> Result<Vec<f32>> {
    let mut v = Vec::new();
    v.try_reserve_exact(count).map_err(|_| AudioError::Budget)?;
    v.resize(count, 0.);
    Ok(v)
}
#[derive(Debug, Clone, Copy)]
pub enum ByteOrder {
    Little,
    Big,
}
/// Native AVSampleFormat names carry no byte-order marker. The adapter must
/// provide the actual decoder's byte order and an explicit channel layout.
pub fn normalize_pcm(
    frame: &DecodedFrame,
    order: ByteOrder,
    layout: ChannelLayout,
    first_frame: i64,
    limits: PcmLimits,
    check: &dyn Fn() -> bool,
) -> Result<PcmBlock> {
    cancel(check)?;
    let info = &frame.info;
    if info.kind != MediaKind::Audio || info.channels.map(|n| n as usize) != Some(layout.len()) {
        return Err(AudioError::Invalid("decoded audio channels/kind"));
    }
    let rate = info
        .sample_rate
        .ok_or(AudioError::Invalid("audio sample rate"))?;
    let frames = info
        .samples
        .ok_or(AudioError::Invalid("audio frame count"))? as usize;
    if frames == 0 {
        return Err(AudioError::Invalid("empty decoded audio"));
    }
    let count = admit(rate, &layout, frames, limits)?;
    let (kind, planar) = info
        .format
        .strip_suffix('p')
        .map_or((info.format.as_str(), false), |s| (s, true));
    let size = match kind {
        "u8" => 1,
        "s16" => 2,
        "s32" | "flt" => 4,
        "s64" | "dbl" => 8,
        _ => return Err(AudioError::Unsupported("audio sample format")),
    };
    let planes = if planar { layout.len() } else { 1 };
    let bytes = frames
        .checked_mul(size)
        .and_then(|n| n.checked_mul(if planar { 1 } else { layout.len() }))
        .ok_or(AudioError::Budget)?;
    if frame.planes.len() != planes
        || frame
            .planes
            .iter()
            .any(|p| p.rows != 1 || p.row_bytes != bytes || p.bytes.len() != bytes)
    {
        return Err(AudioError::Invalid("PCM plane layout"));
    }
    let mut samples = allocate(count)?;
    for n in 0..frames {
        if n % 128 == 0 {
            cancel(check)?;
        }
        for ch in 0..layout.len() {
            let (plane, offset) = if planar {
                (ch, n * size)
            } else {
                (0, (n * layout.len() + ch) * size)
            };
            let input = &frame.planes[plane].bytes[offset..offset + size];
            let mut word = [0u8; 8];
            match order {
                ByteOrder::Little => word[..size].copy_from_slice(input),
                ByteOrder::Big => {
                    for (i, b) in input.iter().rev().enumerate() {
                        word[i] = *b;
                    }
                }
            }
            let value = match kind {
                "u8" => (f64::from(word[0]) - 128.) / 128.,
                "s16" => f64::from(i16::from_le_bytes(word[..2].try_into().unwrap())) / 32768.,
                "s32" => f64::from(i32::from_le_bytes(word[..4].try_into().unwrap())) / 2147483648.,
                "s64" => i64::from_le_bytes(word) as f64 / 9223372036854775808.,
                "flt" => f64::from(f32::from_le_bytes(word[..4].try_into().unwrap())),
                "dbl" => f64::from_le_bytes(word),
                _ => unreachable!(),
            } as f32;
            if !value.is_finite() {
                return Err(AudioError::Invalid("non-finite decoded PCM"));
            }
            samples[n * layout.len() + ch] = value;
        }
    }
    PcmBlock::new(rate, layout, first_frame, samples, limits, check)
}
