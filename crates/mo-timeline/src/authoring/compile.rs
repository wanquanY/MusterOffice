use super::*;
use crate::{
    ContainerDuration, ContainerKind, FadeTransition, NavigationDirection, NextAction, NodeEvent,
    PresentationPreset, PresentationRole, PresentationTrigger, PreviousAction, RestartMode,
    SequenceNavigation, StartCondition, TimeCondition, Timeline, TimelineError, TimelineLimits,
    TimelinePlan, TimelineVersion, TimingContainer, TimingNode, TimingTree, Visibility, cancel,
    clock::BehaviorClock, exact::Ratio, invalid,
};
use std::collections::BTreeSet;

impl PresentationSequence {
    /// Pure, bounded compilation. Recompile this sequence after changing any
    /// effect duration/delay/repeat; no stale, manually maintained offsets remain.
    /// The result carries native editorial grouping, never hidden host state.
    pub fn compile(
        &self,
        limits: TimelineLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Timeline, TimelineError> {
        cancel(check)?;
        if !(128..=4096).contains(&limits.max_exact_bits) {
            return Err(TimelineError::Limit("exact arithmetic policy"));
        }
        // Account for every generated entry BEFORE allocating the timing tree.
        let mut count = 1usize;
        let mut used = BTreeSet::new();
        for (i, group) in self.groups.iter().enumerate() {
            cancel(check)?;
            if group.batches.is_empty()
                || (i > 0 && group.start == PresentationGroupStart::Automatic)
            {
                return Err(invalid(
                    None,
                    "groups require batches; only the first may be automatic",
                ));
            }
            count = bounded_add(count, 1, limits.max_nodes)?;
            for batch in &group.batches {
                cancel(check)?;
                if batch.effects.is_empty() {
                    return Err(invalid(None, "presentation batches require effects"));
                }
                count = bounded_add(count, 1, limits.max_nodes)?;
                for effect in &batch.effects {
                    cancel(check)?;
                    count = bounded_add(
                        count,
                        if matches!(effect.effect, Effect::Fade { .. }) {
                            3
                        } else {
                            2
                        },
                        limits.max_nodes,
                    )?;
                    if !used.insert(effect.id.clone()) {
                        return Err(invalid(
                            Some(&effect.id),
                            "duplicate presentation effect identity",
                        ));
                    }
                }
            }
        }
        if self.groups.is_empty() {
            return Ok(Timeline {
                format: TimelineVersion::V02,
                nodes: vec![],
                tree: Some(TimingTree {
                    roots: vec![],
                    containers: vec![],
                }),
            });
        }
        let mut ids = Ids { used, next: 0 };
        let main = ids.take();
        let mut containers = Vec::with_capacity(count);
        let mut nodes = Vec::new();
        let mut root = container(main.clone(), at(zero()), vec![]);
        root.kind = ContainerKind::Sequence;
        root.presentation = Some(PresentationRole::MainSequence);
        root.duration = ContainerDuration::Indefinite;
        root.restart = RestartMode::Never;
        root.navigation = Some(SequenceNavigation {
            concurrent: false,
            next_action: NextAction::Seek,
            previous_action: PreviousAction::None,
            next_conditions: vec![navigation(NavigationDirection::Next)],
            previous_conditions: vec![navigation(NavigationDirection::Previous)],
        });
        let bits = limits.max_exact_bits;
        for group in &self.groups {
            cancel(check)?;
            let start = match group.start {
                PresentationGroupStart::Automatic => StartCondition::AnyOf {
                    conditions: vec![
                        TimeCondition::Never {},
                        TimeCondition::After {
                            node: main.clone(),
                            event: NodeEvent::OnBegin,
                            delay: zero(),
                        },
                    ],
                },
                PresentationGroupStart::Next => TimeCondition::Never {}.into(),
            };
            let mut click = container(ids.take(), start, vec![]);
            root.children.push(click.id.clone());
            let mut preceding_end = Some(Ratio::integer(0));
            for (batch_index, batch) in group.batches.iter().enumerate() {
                cancel(check)?;
                nonnegative(batch.delay, None)?;
                let origin = preceding_end
                    .take()
                    .ok_or_else(|| {
                        invalid(
                            None,
                            "a sequential batch cannot follow an unbounded parallel batch",
                        )
                    })?
                    .add(&Ratio::time(batch.delay), bits)?;
                let mut parallel = container(ids.take(), at(wire_time(&origin)?), vec![]);
                click.children.push(parallel.id.clone());
                let mut batch_end = Some(Ratio::integer(0));
                for (effect_index, effect) in batch.effects.iter().enumerate() {
                    cancel(check)?;
                    nonnegative(effect.delay, Some(&effect.id))?;
                    let node = TimingNode {
                        id: effect.id.clone(),
                        restart: RestartMode::Always,
                        start: at(zero()),
                        end_conditions: vec![],
                        duration: effect.duration,
                        repeat_milli: effect.repeat_milli,
                        repeat_duration: effect.repeat_duration,
                        fill: effect.fill,
                        time_transform: effect.time_transform,
                        effect: effect.effect.clone(),
                    };
                    let clock = BehaviorClock::new(&node, bits)?;
                    batch_end = match (batch_end, clock.parent_duration()) {
                        (Some(previous), Some(duration)) => {
                            let end = Ratio::time(effect.delay).add(duration, bits)?;
                            Some(if end.cmp(&previous).is_gt() {
                                end
                            } else {
                                previous
                            })
                        }
                        _ => None,
                    };
                    let trigger = if effect_index > 0 {
                        PresentationTrigger::WithPrevious
                    } else if batch_index > 0 {
                        PresentationTrigger::AfterPrevious
                    } else if group.start == PresentationGroupStart::Next {
                        PresentationTrigger::Click
                    } else {
                        PresentationTrigger::WithPrevious
                    };
                    let mut preset = container(ids.take(), at(effect.delay), vec![node.id.clone()]);
                    preset.restart = RestartMode::Never;
                    preset.presentation = Some(PresentationRole::Effect {
                        preset: match &node.effect {
                            Effect::Fade {
                                transition: FadeTransition::In,
                                ..
                            } => PresentationPreset::FadeIn,
                            Effect::Fade {
                                transition: FadeTransition::Out,
                                ..
                            } => PresentationPreset::FadeOut,
                            Effect::MotionLine { .. } | Effect::MotionPath { .. } => {
                                PresentationPreset::CustomMotion
                            }
                            Effect::Rotation { .. } => PresentationPreset::Spin,
                            Effect::Scale { .. } => PresentationPreset::GrowShrink,
                            Effect::SetVisibility {
                                value: Visibility::Visible,
                                ..
                            } => PresentationPreset::Appear,
                            Effect::SetVisibility {
                                value: Visibility::Hidden,
                                ..
                            } => PresentationPreset::Disappear,
                        },
                        trigger,
                    });
                    if let Effect::Fade { target, transition } = &node.effect {
                        // Visibility is a separate native behavior. Entrance reveals
                        // immediately; exit hides over the final native millisecond.
                        // Infinite exits cannot schedule their final visibility change.
                        let millisecond =
                            Ratio::time(RationalTime::new(1, 1000).expect("millisecond"));
                        let span = clock
                            .parent_duration()
                            .map(|duration| {
                                if duration.cmp(&millisecond).is_lt() {
                                    duration.clone()
                                } else {
                                    millisecond.clone()
                                }
                            })
                            .unwrap_or_else(|| millisecond.clone());
                        let start = match transition {
                            FadeTransition::In => at(zero()),
                            FadeTransition::Out => match clock.parent_duration() {
                                Some(duration) => at(wire_time(&duration.sub(&span, bits)?)?),
                                None => TimeCondition::Never {}.into(),
                            },
                        };
                        let helper = TimingNode {
                            id: ids.take(),
                            restart: RestartMode::Always,
                            start,
                            end_conditions: vec![],
                            duration: wire_time(&span)?,
                            repeat_milli: RepeatCount::Finite(1000),
                            repeat_duration: None,
                            fill: FillMode::Hold,
                            time_transform: None,
                            effect: Effect::SetVisibility {
                                target: target.clone(),
                                value: match transition {
                                    FadeTransition::In => Visibility::Visible,
                                    FadeTransition::Out => Visibility::Hidden,
                                },
                            },
                        };
                        match transition {
                            FadeTransition::In => preset.children.insert(0, helper.id.clone()),
                            FadeTransition::Out => preset.children.push(helper.id.clone()),
                        }
                        nodes.push(helper);
                    }
                    parallel.children.push(preset.id.clone());
                    containers.push(preset);
                    nodes.push(node);
                }
                preceding_end = batch_end.map(|end| origin.add(&end, bits)).transpose()?;
                containers.push(parallel);
            }
            containers.push(click);
        }
        containers.push(root);
        let timeline = Timeline {
            format: TimelineVersion::V02,
            nodes,
            tree: Some(TimingTree {
                roots: vec![main],
                containers,
            }),
        };
        // One validator owns effect constraints, tree semantics and all budgets.
        TimelinePlan::compile(&timeline, limits, check)?;
        Ok(timeline)
    }
}

fn bounded_add(count: usize, add: usize, limit: usize) -> Result<usize, TimelineError> {
    count
        .checked_add(add)
        .filter(|n| *n <= limit)
        .ok_or(TimelineError::Limit("presentation authoring node count"))
}
fn zero() -> RationalTime {
    RationalTime::new(0, 1).expect("zero")
}
fn at(offset: RationalTime) -> StartCondition {
    TimeCondition::At { offset }.into()
}
fn nonnegative(t: RationalTime, id: Option<&TimingNodeId>) -> Result<(), TimelineError> {
    if t.ticks.get() < 0 {
        Err(invalid(id, "negative presentation delay"))
    } else {
        Ok(())
    }
}
fn wire_time(r: &Ratio) -> Result<RationalTime, TimelineError> {
    let ticks =
        i64::try_from(&r.n).map_err(|_| TimelineError::Limit("presentation time numerator"))?;
    let scale =
        u32::try_from(&r.d).map_err(|_| TimelineError::Limit("presentation time denominator"))?;
    RationalTime::new(ticks, scale).map_err(|_| TimelineError::Limit("presentation time"))
}
fn navigation(direction: NavigationDirection) -> TimeCondition {
    TimeCondition::Navigation {
        direction,
        target: None,
        delay: zero(),
    }
}
fn container(
    id: TimingNodeId,
    start: StartCondition,
    children: Vec<TimingNodeId>,
) -> TimingContainer {
    TimingContainer {
        time_transform: None,
        id,
        presentation: None,
        restart: RestartMode::Always,
        kind: ContainerKind::Parallel,
        navigation: None,
        start,
        end_conditions: vec![],
        duration: ContainerDuration::Automatic,
        fill: FillMode::Hold,
        children,
    }
}
struct Ids {
    used: BTreeSet<TimingNodeId>,
    next: usize,
}
impl Ids {
    fn take(&mut self) -> TimingNodeId {
        loop {
            let id = TimingNodeId::new(format!("mo.presentation.{}", self.next))
                .expect("bounded generated ID");
            self.next += 1;
            if self.used.insert(id.clone()) {
                return id;
            }
        }
    }
}
