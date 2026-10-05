use crate::*;
use mo_common::{ByteLength, CanonicalError, Digest, DocumentId, RationalTime, ResourceId, digest};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum VideoFormat {
    #[serde(rename = "musteroffice.video/1")]
    V1,
}

/// Content only: resource bytes, URLs, project head, jobs, permissions and caches live in the host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VideoDocument {
    pub format: VideoFormat,
    pub id: DocumentId,
    pub title: String,
    pub canvas: Canvas,
    pub frame_rate: FrameRate,
    pub audio_sample_rate: u32,
    pub root_sequence: SequenceId,
    pub sequences: ObjectMap<SequenceId, Sequence>,
    pub tracks: ObjectMap<TrackId, Track>,
    pub clips: ObjectMap<ClipId, Clip>,
    pub resources: ObjectMap<ResourceId, Resource>,
    pub link_groups: ObjectMap<LinkGroupId, LinkGroup>,
}
impl VideoDocument {
    pub fn empty(id: DocumentId, root: SequenceId, canvas: Canvas, frame_rate: FrameRate) -> Self {
        Self {
            format: VideoFormat::V1,
            id,
            title: String::new(),
            canvas,
            frame_rate,
            audio_sample_rate: 48_000,
            root_sequence: root.clone(),
            sequences: ObjectMap::from([(
                root.clone(),
                Sequence {
                    id: root,
                    tracks: vec![],
                },
            )]),
            tracks: ObjectMap::new(),
            clips: ObjectMap::new(),
            resources: ObjectMap::new(),
            link_groups: ObjectMap::new(),
        }
    }
    /// Hashing does not validate content or grant any resource permission.
    pub fn content_digest(&self) -> Result<Digest, CanonicalError> {
        digest("musteroffice.video/1", self)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Canvas {
    pub width: u32,
    pub height: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Sequence {
    pub id: SequenceId,
    pub tracks: Vec<TrackId>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum TrackKind {
    Video,
    Audio,
    Graphics,
    Captions,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Track {
    pub id: TrackId,
    pub name: String,
    pub kind: TrackKind,
    pub locked: bool,
    pub muted: bool,
    pub solo: bool,
    pub clips: Vec<ClipId>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Clip {
    pub id: ClipId,
    pub name: String,
    pub timeline: TimeRange,
    pub source: ClipSource,
    pub effects: Vec<Effect>,
    /// Explicitly persisted even for silent visual clips; only an audio-track compiler consumes it.
    #[serde(default)]
    pub audio: AudioTimePolicy,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioTimePolicy {
    pub rate: AudioRatePolicy,
    pub reverse: ReverseAudioPolicy,
    pub hold: HoldAudioPolicy,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum AudioRatePolicy {
    Resample,
    #[default]
    PreservePitch,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ReverseAudioPolicy {
    Reverse,
    #[default]
    Mute,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum HoldAudioPolicy {
    #[default]
    Silence,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ClipSource {
    Media {
        resource: ResourceId,
        stream: StreamId,
        #[serde(rename = "timeMap")]
        time_map: TimeMap,
    },
    /// A video-track instance projects child visuals; an audio-track instance projects child audio.
    /// A host creates linked instances when inserting both buses of a compound composition.
    Sequence {
        sequence: SequenceId,
        #[serde(rename = "timeMap")]
        time_map: TimeMap,
    },
    Solid {
        rgba: [u8; 4],
    },
    Text {
        text: String,
        font: ResourceId,
        #[serde(rename = "fontSizeMilliPixels")]
        font_size_milli_pixels: u32,
        rgba: [u8; 4],
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Effect {
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
    Gain {
        #[serde(rename = "milliDecibels")]
        milli_decibels: i32,
    },
    Fade {
        #[serde(rename = "fadeIn")]
        fade_in: RationalTime,
        #[serde(rename = "fadeOut")]
        fade_out: RationalTime,
        /// Fade phase in clip-local time; splitting shifts this range without restarting it.
        envelope: TimeRange,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Resource {
    pub id: ResourceId,
    pub digest: Digest,
    pub byte_length: ByteLength,
    pub descriptor: ResourceDescriptor,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ResourceDescriptor {
    Media {
        streams: BTreeMap<StreamId, MediaStream>,
    },
    Font {
        #[serde(rename = "faceIndex")]
        face_index: u32,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum MediaStream {
    Video {
        available: TimeRange,
        dimensions: Canvas,
        #[serde(rename = "frameRate")]
        frame_rate: Option<FrameRate>,
    },
    Audio {
        available: TimeRange,
        #[serde(rename = "sampleRate")]
        sample_rate: u32,
        channels: u16,
    },
    Image {
        dimensions: Canvas,
    },
}
impl MediaStream {
    pub fn available(&self) -> Option<TimeRange> {
        match self {
            Self::Video { available, .. } | Self::Audio { available, .. } => Some(*available),
            Self::Image { .. } => None,
        }
    }
    pub fn compatible(&self, track: TrackKind) -> bool {
        match self {
            Self::Video { .. } => track == TrackKind::Video,
            Self::Audio { .. } => track == TrackKind::Audio,
            Self::Image { .. } => matches!(track, TrackKind::Video | TrackKind::Graphics),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LinkGroup {
    pub id: LinkGroupId,
    pub members: Vec<ClipId>,
}
