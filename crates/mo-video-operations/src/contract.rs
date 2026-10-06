use mo_common::{Digest, DocumentId, ResourceId, Ticks};
use mo_time::ExactRatio;
use mo_video_edit::{Invalidation, Inverse, ObjectKey, ObjectVersion};
use mo_video_model::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum SnapshotFormat {
    #[serde(rename = "musteroffice.video.snapshot/1")]
    V1,
}
/// Portable authoring snapshot. No host revision, permissions, paths or jobs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VideoSnapshot {
    pub format: SnapshotFormat,
    pub document_digest: Digest,
    pub document: VideoDocument,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateProject {
    pub document_id: DocumentId,
    pub title: String,
    pub root_sequence: SequenceId,
    pub canvas: Canvas,
    pub frame_rate: FrameRate,
    pub audio_sample_rate: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectSummary {
    pub document_id: DocumentId,
    pub document_digest: Digest,
    pub title: String,
    pub canvas: Canvas,
    pub frame_rate: FrameRate,
    pub audio_sample_rate: u32,
    pub root_sequence: SequenceId,
    pub duration: ExactRatio,
    pub frame_count: Ticks,
    pub audio_sample_count: Ticks,
    pub sequence_count: u32,
    pub track_count: u32,
    pub clip_count: u32,
    pub resource_count: u32,
    pub link_group_count: u32,
}
/// This is a computation candidate, never a durable host commit receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MutationCandidate {
    pub base_digest: Digest,
    pub request_digest: Digest,
    pub snapshot: VideoSnapshot,
    pub read_set: Vec<ObjectVersion>,
    pub write_set: Vec<ObjectKey>,
    pub inverse: Inverse,
    pub invalidations: Vec<Invalidation>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ListQuery {
    Sequences,
    Tracks {
        sequence: SequenceId,
    },
    Clips {
        track: TrackId,
        range: Option<TimeRange>,
    },
    Resources,
    LinkGroups {
        clip: Option<ClipId>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PageCursor {
    pub document_digest: Digest,
    pub query_digest: Digest,
    pub offset: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PageRequest {
    pub query: ListQuery,
    pub cursor: Option<PageCursor>,
    pub limit: u16,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ProjectItem {
    Sequence {
        id: SequenceId,
        #[serde(rename = "trackCount")]
        track_count: u32,
    },
    Track {
        id: TrackId,
        name: String,
        #[serde(rename = "trackKind")]
        track_kind: TrackKind,
        locked: bool,
        muted: bool,
        solo: bool,
        #[serde(rename = "clipCount")]
        clip_count: u32,
    },
    Clip {
        clip: Clip,
    },
    Resource {
        resource: Resource,
    },
    LinkGroup {
        group: LinkGroup,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectPage {
    pub document_digest: Digest,
    pub items: Vec<ProjectItem>,
    pub next: Option<PageCursor>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrameRequest {
    pub expected_digest: Digest,
    pub frame: Ticks,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InverseRequest {
    pub expected_digest: Digest,
    pub inverse: Inverse,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceClosure {
    pub document_digest: Digest,
    pub resources: Vec<ResourceRequirement>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceRequirement {
    pub resource_id: ResourceId,
    pub digest: Digest,
    pub byte_length: mo_common::ByteLength,
}
