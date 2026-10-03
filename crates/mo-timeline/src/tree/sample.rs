//! Exact scoped clocks and subtree fill. Event schedule and presentation scope
//! are separate: a held child does not prolong its active end or its successor.
use super::*;
use crate::{events::Inputs, exact::Ratio};
use mo_common::RationalTime;
mod clocks;
mod intervals;
mod properties;
mod schedule;
pub(crate) use intervals::Intervals;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Moment {
    time: Ratio,
    sequence: u32,
    /// Exact causal position during an instantaneous, scoped seek. Host time
    /// and input sequence remain unchanged while child clocks advance.
    order: Ratio,
}
impl Moment {
    fn at(time: Ratio, sequence: u32) -> Self {
        Self {
            time,
            sequence,
            order: Ratio::integer(0),
        }
    }
    fn compare(&self, other: &Self) -> std::cmp::Ordering {
        self.time
            .cmp(&other.time)
            .then_with(|| self.sequence.cmp(&other.sequence))
            .then_with(|| self.order.cmp(&other.order))
    }
    fn precedes(&self, other: &Self) -> bool {
        self.compare(other).is_lt()
    }
}
#[derive(Clone)]
struct Bound {
    moment: Moment,
    /// A deadline belongs to the receiving activation's clock. Its coordinate
    /// is stable even when a later seek changes its projected host time.
    local: Ratio,
}
#[derive(Clone, Default)]
pub(crate) struct Interval {
    gate: Option<Moment>,
    /// Last actual event already dispatched at this lifecycle reset. Wall time
    /// alone cannot distinguish an earlier event at the same exact timestamp.
    reset: Option<usize>,
    /// A navigation reset also invalidates earlier records under the same parent.
    reset_at: Option<Moment>,
    position: Option<usize>,
    position_at: Option<Moment>,
    last_navigation: Option<(NavigationDirection, Moment)>,
    start: Option<Moment>,
    /// Current container leg gate. Turning restarts descendants while leaving
    /// this container's activation, origin and external conditions intact.
    child_start: Option<Moment>,
    turned: bool,
    origin: Option<Ratio>,
    jumps: Vec<clocks::Jump>,
    natural: Option<Bound>,
    end: Option<Bound>,
}
impl Interval {
    fn clipped(&self) -> bool {
        match (&self.natural, &self.end) {
            (Some(n), Some(e)) => e.local.cmp(&n.local).is_lt(),
            (None, Some(_)) => true,
            _ => false,
        }
    }
}
#[derive(Clone)]
struct Scope {
    visible: bool,
    holding: bool,
    retired: bool,
    at: Ratio,
    cutoff: Option<Moment>,
    /// Simple time after ancestor filters. Admission proves equal origins and
    /// endpoints throughout this subtree, including unfiltered intermediate nodes.
    filtered_elapsed: Option<Ratio>,
    backwards: bool,
}
pub(crate) fn schedule(
    plan: &TimelinePlan,
    inputs: &Inputs,
    at: RationalTime,
    check: &dyn Fn() -> bool,
) -> Result<Intervals, TimelineError> {
    schedule::run(plan, &plan.hierarchy, inputs, at, check)
}
pub(crate) fn sample(
    plan: &TimelinePlan,
    intervals: &Intervals,
    binding: &PlaybackBinding,
    at: RationalTime,
    event_cursor: u32,
    check: &dyn Fn() -> bool,
) -> Result<EvaluatedFrame, TimelineError> {
    let presentation_step = intervals.presentation_step.clone();
    let h = &plan.hierarchy;
    let selected = intervals.select(plan, at, check)?;
    let intervals = &selected;
    let bits = plan.limits.max_exact_bits;
    let root = Scope {
        visible: true,
        holding: false,
        retired: false,
        at: Ratio::time(at),
        cutoff: None,
        filtered_elapsed: None,
        backwards: false,
    };
    let mut scopes = vec![root.clone(); intervals.len()];
    let mut frames = vec![None; intervals.len()];
    let mut properties = properties::Properties::default();
    let mut sequences = vec![];
    for (rank, &i) in h.traversal.iter().enumerate() {
        cancel(check)?;
        let entry = Entry::at(&plan.timeline, i);
        let interval = &intervals[i];
        if let Some(position) = interval.position {
            sequences.push(SequenceFrame {
                node: entry.id().clone(),
                position: u32::try_from(position)
                    .map_err(|_| TimelineError::Limit("sequence position"))?,
                current: h.children[i]
                    .get(position)
                    .map(|&c| Entry::at(&plan.timeline, c).id().clone()),
            });
        }
        let parent = h.parents[i]
            .map(|p| scopes[p].clone())
            .unwrap_or_else(|| root.clone());
        let mut scope = parent.clone();
        scope.holding = false;
        scope.visible = false;
        scope.retired |= parent.holding && interval.start.is_none();
        let mut frame = NodeFrame {
            node: entry.id().clone(),
            phase: if scope.retired {
                NodePhase::Suppressed
            } else {
                NodePhase::Waiting
            },
            start: interval.start.as_ref().map(|s| s.time.wire()),
            end: interval.end.as_ref().map(|e| e.moment.time.wire()),
            iteration: None,
            progress: None,
        };
        if let Some(start) = &interval.start {
            if parent
                .cutoff
                .as_ref()
                .map_or_else(|| parent.at.cmp(&start.time).is_lt(), |p| p.precedes(start))
            {
                frame.phase = NodePhase::Scheduled;
            } else if !parent.visible {
                frame.phase = NodePhase::Suppressed;
                scope.retired = true;
            } else {
                let ended = interval.end.as_ref().is_some_and(|e| {
                    parent.cutoff.as_ref().map_or_else(
                        || parent.at.cmp(&e.moment.time).is_ge(),
                        |cutoff| !cutoff.precedes(&e.moment),
                    )
                });
                let clipped = interval.clipped();
                let overridden = ended && clipped && parent.holding;
                let released = entry.fill() == FillMode::Freeze
                    && h.next[i]
                        .and_then(|n| intervals[n].start.as_ref())
                        .is_some_and(|n| parent.at.cmp(&n.time).is_ge());
                let holds = overridden || (entry.fill() != FillMode::Remove && !released);
                if ended && !holds {
                    frame.phase = NodePhase::Finished;
                    scope.retired = true;
                } else {
                    scope.visible = true;
                    scope.holding = ended;
                    scope.retired = false;
                    frame.phase = if ended {
                        NodePhase::Frozen
                    } else {
                        NodePhase::Active
                    };
                    if ended {
                        let end = interval.end.as_ref().expect("ended interval");
                        scope.at = end.moment.time.clone();
                        scope.cutoff = Some(end.moment.clone());
                    }
                    if matches!(entry, Entry::Leaf(_))
                        || plan.container_clocks[i].is_some()
                        || parent.filtered_elapsed.is_some()
                    {
                        let local = if ended {
                            interval.end.as_ref().expect("ended interval").local.clone()
                        } else {
                            selected.local_time(i, &scope.at, scope.cutoff.as_ref(), bits, check)?
                        };
                        let origin = interval.origin.as_ref().expect("active clock origin");
                        let elapsed = scope
                            .filtered_elapsed
                            .clone()
                            .unwrap_or(local.sub(origin, bits)?);
                        if let Entry::Container(c) = entry {
                            let elapsed = elapsed.mul(
                                &super::container_clock::rate(
                                    c.time_transform.unwrap_or_default().speed_milli_percent,
                                    bits,
                                )?,
                                bits,
                            )?;
                            scope.filtered_elapsed = Some(match &plan.container_clocks[i] {
                                Some(clock) => {
                                    let (elapsed, backwards) =
                                        clock.sample(&elapsed, parent.backwards, ended, bits)?;
                                    scope.backwards = backwards;
                                    elapsed
                                }
                                None => elapsed,
                            });
                        }
                        if let Entry::Leaf(n) = entry {
                            let clock = &plan.clocks[i];
                            let cutoff = interval
                                .natural
                                .as_ref()
                                .or(interval.end.as_ref())
                                .filter(|_| clock.needs_endpoint(!n.end_conditions.is_empty()))
                                .map(|e| e.local.sub(origin, bits))
                                .transpose()?;
                            let (iteration, progress) = clock.sample(
                                &elapsed,
                                ended,
                                parent.backwards,
                                local.cmp(origin).is_eq(),
                                cutoff.as_ref(),
                                bits,
                            )?;
                            frame.iteration = Some(iteration);
                            frame.progress = Some(progress.wire());
                            properties.sample(
                                &n.effect,
                                &progress,
                                start,
                                rank,
                                bits,
                                plan.motion_paths[i].as_ref(),
                            )?;
                        }
                    }
                }
            }
        }
        scopes[i] = scope;
        frames[i] = Some(frame);
    }
    let mut containers = frames.split_off(plan.timeline.nodes.len());
    let profile = if plan.motion_paths.iter().any(Option::is_some) {
        crate::PACED_MOTION_FRAME_PROFILE
    } else if plan.timeline.nodes.iter().any(|n| matches!(n.effect, Effect::Rotation { composition, .. } if composition != crate::RotationComposition::Absolute)) {
        "musteroffice.composed-property-frame/0.1-draft"
    } else if plan
        .timeline
        .nodes
        .iter()
        .any(|n| matches!(n.effect, Effect::MotionLine { .. }))
    {
        crate::MOTION_FRAME_PROFILE
    } else if plan
        .timeline
        .nodes
        .iter()
        .any(|n| matches!(n.effect, Effect::SetVisibility { .. } | Effect::Fade { .. }))
    {
        crate::PROPERTY_FRAME_PROFILE
    } else if plan
        .timeline
        .nodes
        .iter()
        .any(|n| matches!(n.effect, Effect::Scale { .. }))
    {
        crate::TRANSFORM_FRAME_PROFILE
    } else if plan.timeline.tree.is_some() {
        "musteroffice.rotation-tree-frame/0.2-draft"
    } else {
        "musteroffice.rotation-frame/0.1-draft"
    };
    let properties::Values {
        rotations,
        scales,
        mut visibility,
        opacity,
        motion,
    } = properties.finish(bits, check)?;
    for (target, value) in &plan.initial_visibility {
        cancel(check)?;
        visibility.entry(target.clone()).or_insert(*value);
    }
    let state = FrameState {
        profile: profile.into(),
        binding: binding.clone(),
        timeline_sha256: plan.digest.clone(),
        time: at.normalized(),
        event_cursor,
        nodes: frames
            .into_iter()
            .map(|f| f.expect("tree covers leaves"))
            .collect(),
        containers: containers
            .drain(..)
            .map(|f| f.expect("tree covers containers"))
            .collect(),
        rotations,
        scales,
        visibility,
        opacity,
        motion,
        sequences,
        presentation_step,
    };
    cancel(check)?;
    let sha256 = mo_common::digest(profile, &state)?;
    Ok(EvaluatedFrame { state, sha256 })
}
