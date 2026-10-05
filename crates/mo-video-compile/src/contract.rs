use mo_common::{Digest, ResourceId, Ticks};
use mo_time::ExactRatio;
use mo_video_model::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VideoFramePlan {
    pub document_digest: Digest,
    pub frame_index: Ticks,
    pub sample: TimeSample,
    pub canvas: Canvas,
    /// Bottom-to-top compositing order. Nested groups must be composited before group effects.
    pub layers: Vec<VisualNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualNode {
    pub track: TrackId,
    pub clip: ClipId,
    pub local: TimeSample,
    /// Ordered effects; group opacity is NOT distributive over child layers.
    pub effects: Vec<VisualEffect>,
    pub source: VisualSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum VisualSource {
    Media {
        binding: MediaBinding,
        mapping: MappedTime,
    },
    Group {
        sequence: SequenceId,
        mapping: MappedTime,
        layers: Vec<VisualNode>,
    },
    Solid {
        rgba: [u8; 4],
    },
    Text {
        text: String,
        font: ResourceId,
        #[serde(rename = "fontDigest")]
        font_digest: Digest,
        #[serde(rename = "faceIndex")]
        face_index: u32,
        #[serde(rename = "fontSizeMilliPixels")]
        font_size_milli_pixels: u32,
        rgba: [u8; 4],
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum VisualEffect {
    Transform {
        #[serde(rename = "xMilliPixels")]
        x_milli_pixels: i32,
        #[serde(rename = "yMilliPixels")]
        y_milli_pixels: i32,
        #[serde(rename = "scaleXPpm")]
        scale_x_ppm: i32,
        #[serde(rename = "scaleYPpm")]
        scale_y_ppm: i32,
        #[serde(rename = "rotationMilliDegrees")]
        rotation_milli_degrees: i32,
        #[serde(rename = "opacityPpm")]
        opacity_ppm: u32,
    },
    Opacity {
        factor: ExactRatio,
    },
}

/// Point inspection/oracle, not a PCM block-processing API. Production DSP must schedule spans.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioSamplePlan {
    pub document_digest: Digest,
    pub sample_index: Ticks,
    pub sample_rate: u32,
    pub sample: TimeSample,
    pub voices: Vec<AudioNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioNode {
    pub track: TrackId,
    pub clip: ClipId,
    pub local: TimeSample,
    /// Each hierarchy stage retains its own rate/pitch policy and gain; do not flatten DSP stages.
    pub mapping: MappedTime,
    pub policy: AudioTimePolicy,
    pub effects: Vec<AudioEffect>,
    pub source: AudioSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum AudioSource {
    Media {
        binding: MediaBinding,
    },
    Group {
        sequence: SequenceId,
        voices: Vec<AudioNode>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum AudioEffect {
    Gain {
        #[serde(rename = "milliDecibels")]
        milli_decibels: i32,
    },
    Amplitude {
        factor: ExactRatio,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MediaBinding {
    pub resource: ResourceId,
    pub digest: Digest,
    pub stream: StreamId,
    pub descriptor: MediaStream,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct QueryStats {
    pub visited: usize,
    /// Selected clip occurrences, including audio subsequently silenced by its explicit policy.
    pub selected: usize,
}

#[derive(Debug)]
pub struct Computed<T> {
    pub value: T,
    pub stats: QueryStats,
}
