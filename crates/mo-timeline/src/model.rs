use mo_common::{ObjectId, RationalTime, TimingNodeId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum TimelineVersion {
    #[serde(rename = "musteroffice.timeline/0.1-draft")]
    V01,
    #[serde(rename = "musteroffice.timeline/0.2-draft")]
    V02,
}
/// Behaviors plus an optional explicit timing forest. The legacy graph retains
/// its byte representation; version 0.2 owns every behavior through tree roots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Timeline {
    pub format: TimelineVersion,
    pub nodes: Vec<TimingNode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tree: Option<TimingTree>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimingTree {
    pub roots: Vec<TimingNodeId>,
    pub containers: Vec<TimingContainer>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimingContainer {
    pub id: TimingNodeId,
    pub kind: ContainerKind,
    pub start: StartCondition,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub end_conditions: Vec<TimeCondition>,
    pub duration: ContainerDuration,
    pub fill: FillMode,
    pub children: Vec<TimingNodeId>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ContainerKind {
    Parallel,
    Sequence,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ContainerDuration {
    /// End when every child has ended; an untriggered child keeps it unresolved.
    Automatic,
    Fixed {
        duration: RationalTime,
    },
    Indefinite,
}
impl Timeline {
    pub fn node_count(&self) -> usize {
        self.nodes
            .len()
            .saturating_add(self.tree.as_ref().map_or(0, |t| t.containers.len()))
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimingNode {
    pub id: TimingNodeId,
    pub start: StartCondition,
    /// Earliest resolved eligible end; absent conditions add no end constraint.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub end_conditions: Vec<TimeCondition>,
    pub duration: RationalTime,
    /// Native count in thousandths, or explicit indefinite repetition.
    pub repeat_milli: RepeatCount,
    /// Additional bound in local active time, before speed scaling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat_duration: Option<RepeatDuration>,
    pub fill: FillMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_transform: Option<TimeTransform>,
    /// The current graph activates each node once (native restart="never").
    pub effect: Effect,
}
/// Finite values retain the existing integer wire form. Infinity is a named
/// alternative, never a sentinel count or a pre-expanded list of iterations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RepeatCount {
    Indefinite,
    #[serde(untagged)]
    Finite(u32),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RepeatDuration {
    Indefinite,
    #[serde(untagged)]
    Finite(RationalTime),
}
impl From<u32> for RepeatCount {
    fn from(value: u32) -> Self {
        Self::Finite(value)
    }
}
/// Local behavior clock. Percentages use native thousandths of one percent;
/// 100000 speed is normal playback. The clock is independent of effect values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimeTransform {
    pub speed_milli_percent: i32,
    pub auto_reverse: bool,
    pub acceleration_milli_percent: u32,
    pub deceleration_milli_percent: u32,
}
impl Default for TimeTransform {
    fn default() -> Self {
        Self {
            speed_milli_percent: 100_000,
            auto_reverse: false,
            acceleration_milli_percent: 0,
            deceleration_milli_percent: 0,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum TimeCondition {
    At {
        offset: RationalTime,
    },
    After {
        node: TimingNodeId,
        event: NodeEvent,
        delay: RationalTime,
    },
    Click {
        target: Option<ObjectId>,
        delay: RationalTime,
    },
}
/// Retained Rust source name for the single begin condition.
pub type StartCondition = TimeCondition;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum NodeEvent {
    Begin,
    End,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum FillMode {
    Remove,
    Freeze,
    /// Unlike freeze, persists when the next sequence sibling starts.
    Hold,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Effect {
    /// Absolute local rotation in native 1/60000 degree units, including multi-turn.
    /// Replacement behavior; later activation wins, author order breaks ties.
    Rotation {
        target: ObjectId,
        from: i32,
        to: i32,
    },
}
impl TimingNode {
    pub fn target(&self) -> &ObjectId {
        match &self.effect {
            Effect::Rotation { target, .. } => target,
        }
    }
    pub fn references_object(&self, object: &ObjectId) -> bool {
        self.target() == object || self.conditions().any(|c| c.target() == Some(object))
    }
    pub fn dependency(&self) -> Option<&TimingNodeId> {
        self.start.dependency()
    }
    pub fn conditions(&self) -> impl Iterator<Item = &TimeCondition> {
        std::iter::once(&self.start).chain(&self.end_conditions)
    }
}
impl TimeCondition {
    pub fn dependency(&self) -> Option<&TimingNodeId> {
        match self {
            Self::After { node, .. } => Some(node),
            _ => None,
        }
    }
    pub fn target(&self) -> Option<&ObjectId> {
        match self {
            Self::Click { target, .. } => target.as_ref(),
            _ => None,
        }
    }
}

impl TimingContainer {
    pub fn conditions(&self) -> impl Iterator<Item = &TimeCondition> {
        std::iter::once(&self.start).chain(&self.end_conditions)
    }
}
