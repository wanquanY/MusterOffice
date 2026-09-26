//! Exact scoped clocks and subtree fill. Event schedule and presentation scope
//! are separate: a held child does not prolong its active end or its successor.
use super::*;
use crate::{events::Clicks, exact::Ratio};
use mo_common::{ObjectId, RationalTime};
mod schedule;

#[derive(Clone, PartialEq, Eq)]
struct Moment {
    time: Ratio,
    sequence: u32,
}
#[derive(Clone)]
struct Interval {
    gate: Option<Moment>,
    start: Option<Moment>,
    natural: Option<Ratio>,
    end: Option<Ratio>,
    end_sequence: u32,
    natural_sequence: u32,
}
impl Interval {
    fn clipped(&self) -> bool {
        match (&self.natural, &self.end) {
            (Some(n), Some(e)) => e.cmp(n).is_lt(),
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
}
/// Constructed only by the shared scheduler, retained only with its owning plan.
pub(crate) struct Intervals(Vec<Interval>);
impl Intervals {
    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }
}
pub(crate) fn schedule(
    plan: &TimelinePlan,
    clicks: &Clicks,
    check: &dyn Fn() -> bool,
) -> Result<Intervals, TimelineError> {
    schedule::run(plan, &plan.hierarchy, clicks, check).map(Intervals)
}
pub(crate) fn sample(
    plan: &TimelinePlan,
    intervals: &Intervals,
    binding: &PlaybackBinding,
    at: RationalTime,
    event_cursor: u32,
    check: &dyn Fn() -> bool,
) -> Result<EvaluatedFrame, TimelineError> {
    let h = &plan.hierarchy;
    let intervals = &intervals.0;
    let bits = plan.limits.max_exact_bits;
    let root = Scope {
        visible: true,
        holding: false,
        retired: false,
        at: Ratio::time(at),
    };
    let mut scopes = vec![root.clone(); intervals.len()];
    let mut frames = vec![None; intervals.len()];
    let mut rotations: BTreeMap<ObjectId, (Ratio, usize, ExactValue)> = BTreeMap::new();
    for (rank, &i) in h.traversal.iter().enumerate() {
        cancel(check)?;
        let entry = Entry::at(&plan.timeline, i);
        let interval = &intervals[i];
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
            end: interval.end.as_ref().map(Ratio::wire),
            iteration: None,
            progress: None,
        };
        if let Some(start) = &interval.start {
            if parent.at.cmp(&start.time).is_lt() {
                frame.phase = NodePhase::Scheduled;
            } else if !parent.visible {
                frame.phase = NodePhase::Suppressed;
                scope.retired = true;
            } else {
                let ended = interval
                    .end
                    .as_ref()
                    .is_some_and(|e| parent.at.cmp(e).is_ge());
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
                        scope.at = interval.end.clone().expect("ended interval");
                    }
                    if let Entry::Leaf(n) = entry {
                        let elapsed = scope.at.sub(&start.time, bits)?;
                        let clock = &plan.clocks[i];
                        let cutoff = interval
                            .natural
                            .as_ref()
                            .or(interval.end.as_ref())
                            .filter(|_| clock.needs_endpoint(!n.end_conditions.is_empty()))
                            .map(|e| e.sub(&start.time, bits))
                            .transpose()?;
                        let (iteration, progress) =
                            clock.sample(&elapsed, ended, cutoff.as_ref(), bits)?;
                        frame.iteration = Some(iteration);
                        frame.progress = Some(progress.wire());
                        let Effect::Rotation { target, from, to } = &n.effect;
                        let value = Ratio::integer(i64::from(*from))
                            .add(
                                &Ratio::integer(i64::from(*to) - i64::from(*from))
                                    .mul(&progress, bits)?,
                                bits,
                            )?
                            .wire();
                        let replace = rotations.get(target).is_none_or(|(previous, order, _)| {
                            start.time.cmp(previous).is_gt()
                                || (start.time.cmp(previous).is_eq() && rank > *order)
                        });
                        if replace {
                            rotations.insert(target.clone(), (start.time.clone(), rank, value));
                        }
                    }
                }
            }
        }
        scopes[i] = scope;
        frames[i] = Some(frame);
    }
    let mut containers = frames.split_off(plan.timeline.nodes.len());
    let profile = if plan.timeline.tree.is_some() {
        "musteroffice.rotation-tree-frame/0.2-draft"
    } else {
        "musteroffice.rotation-frame/0.1-draft"
    };
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
        rotations: rotations
            .into_iter()
            .map(|(id, (_, _, value))| (id, value))
            .collect(),
    };
    cancel(check)?;
    let sha256 = mo_common::digest(profile, &state)?;
    Ok(EvaluatedFrame { state, sha256 })
}
