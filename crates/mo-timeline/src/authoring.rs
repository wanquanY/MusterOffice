//! Ordered presentation authoring, lowered to the same explicit playback tree.
//! Click groups contain sequential batches; effects within a batch run in
//! parallel. A batch waits for ALL effects in its predecessor, including delay,
//! repeats and clock transforms. This is distinct from an arbitrary node edge.
mod compile;
use crate::{Effect, FillMode, RepeatCount, RepeatDuration, TimeTransform};
use mo_common::{RationalTime, TimingNodeId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PresentationSequence {
    pub groups: Vec<PresentationGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PresentationGroup {
    /// Automatic is valid only for the first group; Next waits for navigation.
    pub start: PresentationGroupStart,
    pub batches: Vec<PresentationBatch>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum PresentationGroupStart {
    Automatic,
    Next,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PresentationBatch {
    /// Additional delay after the preceding batch ends (or group activation).
    pub delay: RationalTime,
    pub effects: Vec<PresentationEffect>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PresentationEffect {
    /// Stable behavior identity. Container identities are allocated separately.
    pub id: TimingNodeId,
    /// Offset from this batch's activation, not from the preceding effect.
    pub delay: RationalTime,
    pub duration: RationalTime,
    pub repeat_milli: RepeatCount,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat_duration: Option<RepeatDuration>,
    pub fill: FillMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_transform: Option<TimeTransform>,
    pub effect: Effect,
}
