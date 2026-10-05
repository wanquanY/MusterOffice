use mo_common::{Digest, RationalTime, ResourceId};
use mo_video_model::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum EditContract {
    #[serde(rename = "musteroffice.video.edit/1")]
    V1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Transaction {
    pub contract: EditContract,
    pub expected_digest: Digest,
    pub operations: Vec<Operation>,
}

/// Require every member of an intersecting link group in the explicit selection, or deliberately
/// edit this instance independently. No implicit expansion or provider/Agent override.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum LinkedPolicy {
    RequireAll,
    Independent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Edge {
    Start,
    End,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SplitTarget {
    pub clip: ClipId,
    pub right: ClipId,
}

/// Ripple never guesses a track set. An empty scope leaves the other clips in place.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum Operation {
    SetTitle {
        title: String,
    },
    AddSequence {
        sequence: Sequence,
    },
    AddTrack {
        sequence: SequenceId,
        track: Track,
        before: Option<TrackId>,
    },
    RemoveTrack {
        track: TrackId,
    },
    SetTrack {
        track: TrackId,
        name: String,
        locked: bool,
        muted: bool,
        solo: bool,
    },
    AddResource {
        resource: Resource,
    },
    RemoveResource {
        resource: ResourceId,
    },
    Insert {
        track: TrackId,
        clip: Clip,
        #[serde(rename = "rippleTracks")]
        ripple_tracks: Vec<TrackId>,
    },
    Delete {
        clips: Vec<ClipId>,
        #[serde(rename = "linkedPolicy")]
        linked_policy: LinkedPolicy,
    },
    Move {
        clips: Vec<ClipId>,
        delta: RationalTime,
        #[serde(rename = "targetTrack")]
        target_track: Option<TrackId>,
        #[serde(rename = "linkedPolicy")]
        linked_policy: LinkedPolicy,
    },
    Split {
        targets: Vec<SplitTarget>,
        at: RationalTime,
        #[serde(rename = "rightGroups")]
        right_groups: Vec<LinkGroup>,
        #[serde(rename = "linkedPolicy")]
        linked_policy: LinkedPolicy,
    },
    Trim {
        clips: Vec<ClipId>,
        edge: Edge,
        delta: RationalTime,
        #[serde(rename = "rippleTracks")]
        ripple_tracks: Vec<TrackId>,
        #[serde(rename = "linkedPolicy")]
        linked_policy: LinkedPolicy,
    },
    Slip {
        clips: Vec<ClipId>,
        delta: RationalTime,
        #[serde(rename = "linkedPolicy")]
        linked_policy: LinkedPolicy,
    },
    Roll {
        left: ClipId,
        right: ClipId,
        delta: RationalTime,
        #[serde(rename = "linkedPolicy")]
        linked_policy: LinkedPolicy,
    },
    Slide {
        left: ClipId,
        clip: ClipId,
        right: ClipId,
        delta: RationalTime,
        #[serde(rename = "linkedPolicy")]
        linked_policy: LinkedPolicy,
    },
    SetEffects {
        clip: ClipId,
        effects: Vec<Effect>,
    },
    ReplaceSource {
        clip: ClipId,
        source: ClipSource,
    },
    Link {
        group: LinkGroup,
    },
    Unlink {
        group: LinkGroupId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    content = "id",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum ObjectKey {
    Header,
    Sequence(SequenceId),
    Track(TrackId),
    Clip(ClipId),
    Resource(ResourceId),
    LinkGroup(LinkGroupId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ObjectVersion {
    pub object: ObjectKey,
    pub digest: Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum RestoreObject {
    Title {
        title: String,
    },
    Sequence {
        id: SequenceId,
        value: Option<Sequence>,
    },
    Track {
        id: TrackId,
        value: Option<Track>,
    },
    Clip {
        id: ClipId,
        value: Option<Clip>,
    },
    Resource {
        id: ResourceId,
        value: Option<Resource>,
    },
    LinkGroup {
        id: LinkGroupId,
        value: Option<LinkGroup>,
    },
}
impl RestoreObject {
    pub fn key(&self) -> ObjectKey {
        match self {
            Self::Title { .. } => ObjectKey::Header,
            Self::Sequence { id, .. } => ObjectKey::Sequence(id.clone()),
            Self::Track { id, .. } => ObjectKey::Track(id.clone()),
            Self::Clip { id, .. } => ObjectKey::Clip(id.clone()),
            Self::Resource { id, .. } => ObjectKey::Resource(id.clone()),
            Self::LinkGroup { id, .. } => ObjectKey::LinkGroup(id.clone()),
        }
    }
}

/// A conditional inverse, not a whole-project rewind. The host binds this to its authorized
/// history record; the core neither grants permission nor makes a durable commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Inverse {
    pub conditions: Vec<ObjectVersion>,
    pub restore: Vec<RestoreObject>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Invalidation {
    pub track: TrackId,
    pub ranges: Vec<TimeRange>,
}
