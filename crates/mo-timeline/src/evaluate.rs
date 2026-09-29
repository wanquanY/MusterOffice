use crate::PlaybackGeneration;
use crate::{TimelineError, TimelinePlan};
use mo_common::{Digest, ObjectId, PlaybackSessionId, RationalTime, TimingNodeId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub const TRANSFORM_FRAME_PROFILE: &str = "musteroffice.transform-frame/0.1-draft";
/// Connected-path position error is bounded by 2^-17 in normalized slide
/// coordinates, including arc-length and Q64 subdivision error. This profile
/// does not describe analytic curve coordinates as exact.
pub const PACED_MOTION_FRAME_PROFILE: &str = "musteroffice.paced-motion-frame-q64/0.1-draft";
pub const MOTION_FRAME_PROFILE: &str = "musteroffice.motion-frame/0.1-draft";
pub const PROPERTY_FRAME_PROFILE: &str = "musteroffice.property-frame/0.1-draft";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlaybackBinding {
    pub session: PlaybackSessionId,
    pub revision: Digest,
    pub generation: PlaybackGeneration,
}
/// Output-only exact reduced ratio. Units are defined by each property channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExactValue {
    pub numerator: String,
    pub denominator: String,
}
/// An exact angle with its document dependency still explicit. The layout
/// orientation is resolved only at placement, after source inheritance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExactRotation {
    pub numerator: String,
    pub denominator: String,
    #[serde(default, skip_serializing_if = "RotationBasis::is_absolute")]
    pub basis: RotationBasis,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RotationBasis {
    #[default]
    Absolute,
    Layout,
}
impl RotationBasis {
    pub fn is_absolute(&self) -> bool {
        *self == Self::Absolute
    }
}
impl ExactRotation {
    pub fn value(&self) -> ExactValue {
        ExactValue {
            numerator: self.numerator.clone(),
            denominator: self.denominator.clone(),
        }
    }
}
impl From<ExactValue> for ExactRotation {
    fn from(value: ExactValue) -> Self {
        Self {
            numerator: value.numerator,
            denominator: value.denominator,
            basis: RotationBasis::Absolute,
        }
    }
}
/// Scale values remain thousandths of a percent until the placement boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExactScale {
    pub x: ExactValue,
    pub y: ExactValue,
}
/// Position offsets in fractions of the slide width/height. Ratios encode the
/// computed position exactly; the frame profile states any path approximation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExactMotion {
    pub x: ExactValue,
    pub y: ExactValue,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlaybackEvent {
    pub generation: PlaybackGeneration,
    pub sequence: u32,
    pub at: RationalTime,
    pub event: InputEvent,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum InputEvent {
    Click {
        target: Option<ObjectId>,
    },
    Navigation {
        direction: crate::NavigationDirection,
        target: Option<ObjectId>,
    },
}
/// Complete prefix, explicitly including periods when no event occurred. A host
/// that cannot supply this context must not invent clicks when seeking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EventHistory {
    pub binding: PlaybackBinding,
    pub through: RationalTime,
    pub events: Vec<PlaybackEvent>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum NodePhase {
    Waiting,
    Scheduled,
    Active,
    Frozen,
    Finished,
    Suppressed,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NodeFrame {
    pub node: TimingNodeId,
    pub phase: NodePhase,
    pub start: Option<ExactValue>,
    pub end: Option<ExactValue>,
    pub iteration: Option<String>,
    pub progress: Option<ExactValue>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrameState {
    pub profile: String,
    pub binding: PlaybackBinding,
    pub timeline_sha256: Digest,
    pub time: RationalTime,
    pub event_cursor: u32,
    pub nodes: Vec<NodeFrame>,
    pub rotations: BTreeMap<ObjectId, ExactRotation>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub scales: BTreeMap<ObjectId, ExactScale>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub visibility: BTreeMap<ObjectId, crate::Visibility>,
    /// Exact slide-relative offsets from the original layout center.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub motion: BTreeMap<ObjectId, ExactMotion>,
    /// Exact whole-object opacity in [0, 1], before one render-boundary rounding.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub opacity: BTreeMap<ObjectId, ExactValue>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub containers: Vec<NodeFrame>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sequences: Vec<SequenceFrame>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SequenceFrame {
    pub node: TimingNodeId,
    /// Zero-based cursor. The child count denotes the position after the end.
    pub position: u32,
    pub current: Option<TimingNodeId>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvaluatedFrame {
    pub state: FrameState,
    pub sha256: Digest,
}

impl TimelinePlan {
    pub fn evaluate(
        &self,
        binding: &PlaybackBinding,
        at: RationalTime,
        history: Option<&EventHistory>,
        check: &dyn Fn() -> bool,
    ) -> Result<EvaluatedFrame, TimelineError> {
        let events = self.validate_events(binding, at, history, check)?;
        let intervals = crate::tree::schedule(self, &events.inputs(check)?, at, check)?;
        crate::tree::sample(self, &intervals, binding, at, events.cursor(), check)
    }
}
