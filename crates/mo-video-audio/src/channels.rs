use crate::{AudioError, Result};
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Channel {
    FrontLeft,
    FrontRight,
    FrontCenter,
    LowFrequency,
    BackLeft,
    BackRight,
    SideLeft,
    SideRight,
    BackCenter,
    /// Explicit unnamed channel identity; never infer speaker placement from count.
    Discrete(u8),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelLayout(Vec<Channel>);
impl ChannelLayout {
    pub fn new(channels: Vec<Channel>) -> Result<Self> {
        if channels.is_empty() || channels.len() > 64 {
            return Err(AudioError::Invalid("channel count"));
        }
        let mut unique = channels.clone();
        unique.sort();
        unique.dedup();
        if unique.len() != channels.len() {
            return Err(AudioError::Invalid("duplicate channel"));
        }
        Ok(Self(channels))
    }
    pub fn mono() -> Self {
        Self(vec![Channel::FrontCenter])
    }
    pub fn stereo() -> Self {
        Self(vec![Channel::FrontLeft, Channel::FrontRight])
    }
    pub fn channels(&self) -> &[Channel] {
        &self.0
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn from_codec_name(name: &str) -> Result<Self> {
        use Channel::*;
        let channels = match name {
            "mono" => vec![FrontCenter],
            "stereo" => vec![FrontLeft, FrontRight],
            "2.1" => vec![FrontLeft, FrontRight, LowFrequency],
            "3.0" => vec![FrontLeft, FrontRight, FrontCenter],
            "quad" => vec![FrontLeft, FrontRight, BackLeft, BackRight],
            "5.0" => vec![FrontLeft, FrontRight, FrontCenter, BackLeft, BackRight],
            "5.0(side)" => vec![FrontLeft, FrontRight, FrontCenter, SideLeft, SideRight],
            "5.1" => vec![
                FrontLeft,
                FrontRight,
                FrontCenter,
                LowFrequency,
                BackLeft,
                BackRight,
            ],
            "5.1(side)" => vec![
                FrontLeft,
                FrontRight,
                FrontCenter,
                LowFrequency,
                SideLeft,
                SideRight,
            ],
            "7.1" => vec![
                FrontLeft,
                FrontRight,
                FrontCenter,
                LowFrequency,
                BackLeft,
                BackRight,
                SideLeft,
                SideRight,
            ],
            _ => return Err(AudioError::Unsupported("explicit channel layout required")),
        };
        Self::new(channels)
    }
}
/// Explicit linear channel matrix, row=output, column=input. No auto gain
/// normalization, channel truncation, or silent LFE/surround interpretation.
#[derive(Debug, Clone)]
pub struct ChannelMatrix {
    pub(crate) input: ChannelLayout,
    pub(crate) output: ChannelLayout,
    pub(crate) gains: Vec<f64>,
}
impl ChannelMatrix {
    pub fn new(input: ChannelLayout, output: ChannelLayout, gains: Vec<f64>) -> Result<Self> {
        if gains.len() != input.len() * output.len()
            || gains.iter().any(|v| !v.is_finite() || v.abs() > 16.)
        {
            return Err(AudioError::Invalid("channel matrix"));
        }
        Ok(Self {
            input,
            output,
            gains,
        })
    }
    pub fn identity(layout: ChannelLayout) -> Self {
        let n = layout.len();
        let mut gains = vec![0.; n * n];
        for i in 0..n {
            gains[i * n + i] = 1.;
        }
        Self {
            input: layout.clone(),
            output: layout,
            gains,
        }
    }
    pub fn mono_to_stereo() -> Self {
        Self {
            input: ChannelLayout::mono(),
            output: ChannelLayout::stereo(),
            gains: vec![1., 1.],
        }
    }
    pub fn stereo_to_mono_average() -> Self {
        Self {
            input: ChannelLayout::stereo(),
            output: ChannelLayout::mono(),
            gains: vec![0.5, 0.5],
        }
    }
    pub fn input(&self) -> &ChannelLayout {
        &self.input
    }
    pub fn output(&self) -> &ChannelLayout {
        &self.output
    }
}
