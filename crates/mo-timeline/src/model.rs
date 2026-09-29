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
    /// Native presentation identity. It participates in initial playback state
    /// and editable export; it never changes the container's declared clock.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presentation: Option<PresentationRole>,
    #[serde(default, skip_serializing_if = "RestartMode::is_never")]
    pub restart: RestartMode,
    pub kind: ContainerKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub navigation: Option<SequenceNavigation>,
    pub start: StartCondition,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub end_conditions: Vec<TimeCondition>,
    pub duration: ContainerDuration,
    /// Filter the container's simple time before its descendants consume it.
    /// Compilation validates the supported clock domain; this is never copied
    /// into the leaves or interpreted as an independent per-effect easing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_transform: Option<TimeTransform>,
    pub fill: FillMode,
    pub children: Vec<TimingNodeId>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum PresentationRole {
    MainSequence,
    Effect {
        preset: PresentationPreset,
        trigger: PresentationTrigger,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum PresentationPreset {
    Appear,
    Disappear,
    Spin,
    GrowShrink,
    CustomMotion,
    FadeIn,
    FadeOut,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum PresentationTrigger {
    Click,
    WithPrevious,
    AfterPrevious,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ContainerKind {
    Parallel,
    Sequence,
}
/// Sequence controls are document computation, independent of host buttons or
/// keyboard bindings. Conditions are disjunctions, like begin/end conditions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SequenceNavigation {
    pub concurrent: bool,
    pub next_action: NextAction,
    pub previous_action: PreviousAction,
    pub next_conditions: Vec<TimeCondition>,
    pub previous_conditions: Vec<TimeCondition>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum NextAction {
    None,
    Seek,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum PreviousAction {
    None,
    SkipTimed,
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum NavigationDirection {
    Next,
    Previous,
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
    #[serde(default, skip_serializing_if = "RestartMode::is_never")]
    pub restart: RestartMode,
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
    pub effect: Effect,
}
/// Admission of new begin instances within one parent activation. Ancestor
/// reactivation resets this policy, including `Never`. Omission preserves the
/// existing draft's once-per-parent behavior, independently of native defaults.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RestartMode {
    #[default]
    Never,
    Always,
    WhenNotActive,
}
impl RestartMode {
    pub fn is_never(&self) -> bool {
        *self == Self::Never
    }
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
    /// An explicitly indefinite condition, never a fabricated zero-time event.
    Never {},
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
    Navigation {
        direction: NavigationDirection,
        target: Option<ObjectId>,
        delay: RationalTime,
    },
}
/// A flat disjunction of native begin conditions. The single-condition wire
/// representation remains unchanged; alternatives cannot recursively nest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum StartCondition {
    AnyOf {
        #[schemars(length(min = 1))]
        conditions: Vec<TimeCondition>,
    },
    #[serde(untagged)]
    Single(TimeCondition),
}
impl From<TimeCondition> for StartCondition {
    fn from(value: TimeCondition) -> Self {
        Self::Single(value)
    }
}
impl StartCondition {
    pub fn conditions(&self) -> &[TimeCondition] {
        match self {
            Self::Single(condition) => std::slice::from_ref(condition),
            Self::AnyOf { conditions } => conditions,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum NodeEvent {
    /// A dependency on the referenced node's interval boundary.
    Begin,
    End,
    /// A lifecycle notification delivered by the referenced timing node.
    /// This is not an event on a shape or another external event target.
    /// Preserve the native distinction: presentation editors use onBegin to
    /// recognize an automatic group attached to a main sequence.
    OnBegin,
    OnEnd,
}
impl NodeEvent {
    /// In the owned timing-node domain, notifications are emitted at the same
    /// boundaries as interval dependencies. Native event identity stays in the
    /// document; only the scheduler's listener slot is shared.
    pub(crate) const fn edge_index(self) -> usize {
        match self {
            Self::Begin | Self::OnBegin => 0,
            Self::End | Self::OnEnd => 1,
        }
    }
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
        #[serde(default, skip_serializing_if = "RotationComposition::is_absolute")]
        composition: RotationComposition,
    },
    /// Object-local scale factors in thousandths of a percent. 100000 is
    /// identity, zero collapses an axis. Independent of the rotation channel.
    Scale {
        target: ObjectId,
        from: ScaleValue,
        to: ScaleValue,
    },
    /// Fixed property replacement while active or held. Before activation and
    /// after removal, the underlying document/animation value is unchanged.
    /// This does not imply the initial-hidden state of an entrance preset.
    SetVisibility { target: ObjectId, value: Visibility },
    /// Straight native motion path, in slide fractions relative to the original
    /// layout center. Translation is in slide axes, independent of local scale
    /// and rotation. This is not a multi-segment/curve path approximation.
    MotionLine {
        target: ObjectId,
        from: crate::MotionPoint,
        to: crate::MotionPoint,
    },
    /// Connected lines and cubic Beziers, paced by normalized path length.
    /// The original control points are retained for native editable export.
    MotionPath {
        target: ObjectId,
        path: crate::MotionPath,
    },
    /// Native whole-object fade filter. Progress is supplied by the behavior
    /// clock; visibility remains an independent channel, including preset helpers.
    Fade {
        target: ObjectId,
        transition: FadeTransition,
    },
}
/// Rotation is composed before object/group placement. Layout replaces earlier
/// animation offsets while preserving the document's local orientation. Add
/// sums the sampled offset with the lower-priority visible rotation stack.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RotationComposition {
    #[default]
    Absolute,
    Layout,
    Add,
}
impl RotationComposition {
    pub fn is_absolute(&self) -> bool {
        *self == Self::Absolute
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum FadeTransition {
    In,
    Out,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Visibility {
    Visible,
    Hidden,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScaleValue {
    pub x: u32,
    pub y: u32,
}
pub const MAX_SCALE_MILLI_PERCENT: u32 = 2_147_483_625;
impl TimingNode {
    pub fn target(&self) -> &ObjectId {
        match &self.effect {
            Effect::Rotation { target, .. }
            | Effect::Scale { target, .. }
            | Effect::MotionLine { target, .. }
            | Effect::MotionPath { target, .. }
            | Effect::Fade { target, .. }
            | Effect::SetVisibility { target, .. } => target,
        }
    }
    pub fn references_object(&self, object: &ObjectId) -> bool {
        self.target() == object || self.conditions().any(|c| c.target() == Some(object))
    }
    pub fn conditions(&self) -> impl Iterator<Item = &TimeCondition> {
        self.start.conditions().iter().chain(&self.end_conditions)
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
            Self::Click { target, .. } | Self::Navigation { target, .. } => target.as_ref(),
            _ => None,
        }
    }
}

impl TimingContainer {
    pub fn conditions(&self) -> impl Iterator<Item = &TimeCondition> {
        self.start
            .conditions()
            .iter()
            .chain(&self.end_conditions)
            .chain(
                self.navigation
                    .iter()
                    .flat_map(|n| n.next_conditions.iter().chain(&n.previous_conditions)),
            )
    }
}
impl TimeCondition {
    pub(crate) fn input(&self) -> Option<crate::InputEvent> {
        match self {
            Self::Click { target, .. } => Some(crate::InputEvent::Click {
                target: target.clone(),
            }),
            Self::Navigation {
                direction, target, ..
            } => Some(crate::InputEvent::Navigation {
                direction: *direction,
                target: target.clone(),
            }),
            _ => None,
        }
    }
}
