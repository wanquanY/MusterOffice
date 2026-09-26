use crate::PlaybackGeneration;
use crate::{TimelineError, TimelinePlan};
use mo_common::{Digest, ObjectId, PlaybackSessionId, RationalTime, TimingNodeId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlaybackBinding {
    pub session: PlaybackSessionId,
    pub revision: Digest,
    pub generation: PlaybackGeneration,
}
/// Output-only exact reduced ratio. Rotation units remain 1/60000 degree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExactValue {
    pub numerator: String,
    pub denominator: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlaybackEvent {
    pub generation: PlaybackGeneration,
    pub sequence: u32,
    pub at: RationalTime,
    pub event: InputEvent,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum InputEvent {
    Click { target: Option<ObjectId> },
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
    pub rotations: BTreeMap<ObjectId, ExactValue>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub containers: Vec<NodeFrame>,
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
        let intervals = crate::tree::schedule(self, &events.clicks(check)?, check)?;
        crate::tree::sample(self, &intervals, binding, at, events.cursor(), check)
    }
}
