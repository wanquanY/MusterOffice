use crate::{Timeline, TimelineError, cancel, invalid};
use mo_common::Digest;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy)]
pub struct TimelineLimits {
    pub max_nodes: usize,
    pub max_events: usize,
    pub max_conditions: usize,
    pub max_schedule_steps: usize,
    pub max_exact_bits: u64,
    pub max_depth: usize,
}
impl Default for TimelineLimits {
    fn default() -> Self {
        Self {
            max_nodes: 10000,
            max_events: 65536,
            max_conditions: 65536,
            max_schedule_steps: 1_000_000,
            max_exact_bits: 1024,
            max_depth: 128,
        }
    }
}
/// Immutable reusable computation plan. Revision/generation are supplied by the host.
#[derive(Debug, Clone)]
pub struct TimelinePlan {
    pub(crate) timeline: Timeline,
    pub(crate) interactive: bool,
    pub(crate) digest: Digest,
    pub(crate) limits: TimelineLimits,
    pub(crate) hierarchy: crate::tree::Hierarchy,
    pub(crate) clocks: Vec<crate::clock::BehaviorClock>,
}
impl TimelinePlan {
    pub fn compile(
        timeline: &Timeline,
        limits: TimelineLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, TimelineError> {
        cancel(check)?;
        if timeline.node_count() > limits.max_nodes {
            return Err(TimelineError::Limit("timing node count"));
        }
        if !(128..=4096).contains(&limits.max_exact_bits) {
            return Err(TimelineError::Limit("exact arithmetic policy"));
        }
        match timeline.format {
            crate::TimelineVersion::V01 if timeline.tree.is_some() => {
                return Err(invalid(None, "version 0.1 does not carry a timing tree"));
            }
            crate::TimelineVersion::V02 if timeline.tree.is_none() => {
                return Err(invalid(None, "version 0.2 requires a timing tree"));
            }
            _ => (),
        }
        crate::tree::compile(timeline, limits, check)
    }

    pub fn digest(&self) -> &Digest {
        &self.digest
    }
    pub fn targets(&self) -> BTreeSet<&mo_common::ObjectId> {
        let mut targets: BTreeSet<_> = self
            .timeline
            .nodes
            .iter()
            .flat_map(|n| {
                std::iter::once(n.target())
                    .chain(n.conditions().filter_map(crate::TimeCondition::target))
            })
            .collect();
        if let Some(tree) = &self.timeline.tree {
            targets.extend(
                tree.containers
                    .iter()
                    .flat_map(|c| c.conditions().filter_map(crate::TimeCondition::target)),
            );
        }
        targets
    }
}
pub(crate) fn clocks(
    timeline: &Timeline,
    bits: u64,
    check: &dyn Fn() -> bool,
) -> Result<Vec<crate::clock::BehaviorClock>, TimelineError> {
    timeline
        .nodes
        .iter()
        .map(|node| {
            cancel(check)?;
            crate::clock::BehaviorClock::new(node, bits)
        })
        .collect()
}
