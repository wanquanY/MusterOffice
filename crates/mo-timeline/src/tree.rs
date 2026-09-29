//! Structural ownership and causal condition graph. Containers never flatten into
//! synthetic absolute leaf offsets; begin/end events retain their native scopes.
mod causality;
pub(crate) mod container_clock;
mod presentation;
mod sample;
use crate::*;
pub(crate) use sample::{Intervals, sample, schedule};
use std::collections::{BTreeMap, VecDeque};

#[derive(Debug, Clone)]
pub(crate) struct Hierarchy {
    parents: Vec<Option<usize>>,
    previous: Vec<Option<usize>>,
    next: Vec<Option<usize>>,
    children: Vec<Vec<usize>>,
    dependencies: Vec<BTreeMap<ConditionIndex, usize>>,
    positions: Vec<usize>,
    traversal: Vec<usize>,
    listeners: Vec<Vec<(usize, ConditionIndex)>>,
    input_listeners: BTreeMap<InputEvent, Vec<(usize, ConditionIndex)>>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ConditionIndex {
    Start(usize),
    End(usize),
    Next(usize),
    Previous(usize),
}
#[derive(Clone, Copy)]
enum Entry<'a> {
    Leaf(&'a TimingNode),
    Container(&'a TimingContainer),
}
impl<'a> Entry<'a> {
    fn at(t: &'a Timeline, i: usize) -> Self {
        if i < t.nodes.len() {
            Self::Leaf(&t.nodes[i])
        } else {
            Self::Container(&t.tree.as_ref().expect("tree version").containers[i - t.nodes.len()])
        }
    }
    fn id(self) -> &'a TimingNodeId {
        match self {
            Self::Leaf(n) => &n.id,
            Self::Container(n) => &n.id,
        }
    }
    fn start(self) -> &'a StartCondition {
        match self {
            Self::Leaf(n) => &n.start,
            Self::Container(n) => &n.start,
        }
    }
    fn fill(self) -> FillMode {
        match self {
            Self::Leaf(n) => n.fill,
            Self::Container(n) => n.fill,
        }
    }
    fn restart(self) -> RestartMode {
        match self {
            Self::Leaf(n) => n.restart,
            Self::Container(n) => n.restart,
        }
    }
    fn navigation(self) -> Option<&'a SequenceNavigation> {
        match self {
            Self::Leaf(_) => None,
            Self::Container(c) => c.navigation.as_ref(),
        }
    }
    fn conditions(self) -> impl Iterator<Item = (ConditionIndex, &'a TimeCondition)> {
        self.start()
            .conditions()
            .iter()
            .enumerate()
            .map(|(k, c)| (ConditionIndex::Start(k), c))
            .chain(
                self.ends()
                    .iter()
                    .enumerate()
                    .map(|(k, c)| (ConditionIndex::End(k), c)),
            )
            .chain(self.navigation().into_iter().flat_map(|n| {
                n.next_conditions
                    .iter()
                    .enumerate()
                    .map(|(k, c)| (ConditionIndex::Next(k), c))
            }))
            .chain(self.navigation().into_iter().flat_map(|n| {
                n.previous_conditions
                    .iter()
                    .enumerate()
                    .map(|(k, c)| (ConditionIndex::Previous(k), c))
            }))
    }
    fn condition(self, index: ConditionIndex) -> &'a TimeCondition {
        match index {
            ConditionIndex::Start(k) => &self.start().conditions()[k],
            ConditionIndex::End(k) => &self.ends()[k],
            ConditionIndex::Next(k) => &self.navigation().expect("sequence").next_conditions[k],
            ConditionIndex::Previous(k) => {
                &self.navigation().expect("sequence").previous_conditions[k]
            }
        }
    }
    fn ends(self) -> &'a [TimeCondition] {
        match self {
            Self::Leaf(n) => &n.end_conditions,
            Self::Container(n) => &n.end_conditions,
        }
    }
}
pub(crate) fn compile(
    t: &Timeline,
    limits: TimelineLimits,
    check: &dyn Fn() -> bool,
) -> Result<TimelinePlan, TimelineError> {
    let tree = t.tree.as_ref();
    let count = t.node_count();
    if tree.is_some() && (limits.max_depth == 0 || limits.max_depth > 1024) {
        return Err(TimelineError::Limit("timing tree depth policy"));
    }
    let mut ids = BTreeMap::new();
    let mut interactive = false;
    let mut conditions = 0usize;
    for i in 0..count {
        cancel(check)?;
        let entry = Entry::at(t, i);
        if ids.insert(entry.id(), i).is_some() {
            return Err(invalid(Some(entry.id()), "duplicate timing identity"));
        }
        match entry {
            Entry::Leaf(n) => crate::clock::validate(n)?,
            Entry::Container(TimingContainer {
                duration: ContainerDuration::Fixed { duration },
                ..
            }) if duration.ticks.get() < 0 => {
                return Err(invalid(Some(entry.id()), "negative container duration"));
            }
            _ => (),
        }
        let starts = entry.start().conditions();
        if starts.is_empty() {
            return Err(invalid(Some(entry.id()), "empty start condition list"));
        }
        if entry.navigation().is_some()
            && !matches!(
                entry,
                Entry::Container(TimingContainer {
                    kind: ContainerKind::Sequence,
                    ..
                })
            )
        {
            return Err(invalid(Some(entry.id()), "navigation requires a sequence"));
        }
        conditions = conditions.saturating_add(entry.conditions().count());
        if conditions > limits.max_conditions {
            return Err(TimelineError::Limit("timing condition count"));
        }
        for (position, condition) in entry.conditions() {
            cancel(check)?;
            let delay = match condition {
                TimeCondition::Never {} => continue,
                TimeCondition::At { offset } => offset,
                TimeCondition::After { delay, .. } => delay,
                TimeCondition::Click { delay, .. } | TimeCondition::Navigation { delay, .. } => {
                    interactive = true;
                    delay
                }
            };
            if delay.ticks.get() < 0 {
                return Err(invalid(
                    Some(entry.id()),
                    if matches!(position, ConditionIndex::Start(_)) {
                        "negative start offset"
                    } else {
                        "negative condition offset"
                    },
                ));
            }
        }
    }
    let mut h = Hierarchy {
        parents: vec![None; count],
        previous: vec![None; count],
        next: vec![None; count],
        children: vec![vec![]; count],
        dependencies: vec![BTreeMap::new(); count],
        positions: vec![0; count],
        traversal: vec![],
        listeners: vec![vec![]; count * 2],
        input_listeners: BTreeMap::new(),
    };
    let mut owned = vec![false; count];
    let mut references = tree.map_or(t.nodes.len(), |tree| tree.roots.len());
    for c in tree.into_iter().flat_map(|tree| &tree.containers) {
        references = references.saturating_add(c.children.len());
    }
    if references != count {
        return Err(invalid(
            None,
            "each timing entry requires exactly one structural owner",
        ));
    }
    let mut assign = |id: &TimingNodeId, parent: Option<usize>| -> Result<usize, TimelineError> {
        let i = *ids
            .get(id)
            .ok_or_else(|| invalid(Some(id), "missing timing tree entry"))?;
        if std::mem::replace(&mut owned[i], true) {
            return Err(invalid(Some(id), "multiple timing tree owners"));
        }
        h.parents[i] = parent;
        Ok(i)
    };
    let mut roots = vec![];
    for id in tree
        .map(|tree| tree.roots.iter().collect::<Vec<_>>())
        .unwrap_or_else(|| t.nodes.iter().map(|n| &n.id).collect())
    {
        cancel(check)?;
        roots.push(assign(id, None)?);
    }
    for (c, container) in tree
        .into_iter()
        .flat_map(|tree| &tree.containers)
        .enumerate()
    {
        let i = t.nodes.len() + c;
        for (position, id) in container.children.iter().enumerate() {
            cancel(check)?;
            let child = assign(id, Some(i))?;
            h.positions[child] = position;
            h.children[i].push(child);
        }
        if container.kind == ContainerKind::Sequence {
            for &child in &h.children[i] {
                if container.navigation.is_none() && !Entry::at(t, child).restart().is_never() {
                    return Err(invalid(
                        Some(Entry::at(t, child).id()),
                        "restarting sequence entries require the navigation event profile",
                    ));
                }
            }
            for pair in h.children[i].windows(2) {
                h.previous[pair[1]] = Some(pair[0]);
                h.next[pair[0]] = Some(pair[1]);
            }
        }
    }
    let mut stack: Vec<_> = roots.into_iter().rev().map(|i| (i, 1)).collect();
    while let Some((i, depth)) = stack.pop() {
        cancel(check)?;
        if tree.is_some() && depth > limits.max_depth {
            return Err(TimelineError::Limit("timing tree nesting"));
        }
        h.traversal.push(i);
        stack.extend(h.children[i].iter().rev().map(|&c| (c, depth + 1)));
    }
    if h.traversal.len() != count || owned.iter().any(|v| !v) {
        return Err(invalid(None, "cyclic or unowned timing tree"));
    }
    // Validate causal declarations without ancestor clipping edges. Runtime
    // termination stops descendants; it is not a prerequisite for their begin.
    let mut causal = causality::Causality::new(count * 2);
    for i in 0..count {
        cancel(check)?;
        let entry = Entry::at(t, i);
        if let Some(p) = h.parents[i] {
            causal.require(i * 2, p * 2);
        }
        let navigable = h.parents[i].is_some_and(|p| Entry::at(t, p).navigation().is_some());
        if let Some(p) = h.previous[i]
            && !navigable
        {
            causal.require(i * 2, p * 2 + 1);
        }
        let mut alternatives = vec![];
        let mut independent = navigable;
        let mut end = vec![i * 2];
        for (index, condition) in entry.conditions() {
            cancel(check)?;
            if let Some(input) = condition.input() {
                h.input_listeners.entry(input).or_default().push((i, index));
            }
            if let TimeCondition::After { node, event, .. } = condition {
                let d = *ids
                    .get(node)
                    .ok_or_else(|| invalid(Some(entry.id()), "missing timing dependency"))?;
                let vertex = d * 2 + event.edge_index();
                h.dependencies[i].insert(index, d);
                h.listeners[vertex].push((i, index));
                match index {
                    ConditionIndex::Start(_) => alternatives.push(vertex),
                    ConditionIndex::End(_) => end.push(vertex),
                    _ => (), // navigation is an input, not a requirement for the first begin/end
                }
            } else if matches!(index, ConditionIndex::Start(_))
                && !matches!(condition, TimeCondition::Never {})
            {
                independent = true;
            }
        }
        if !independent {
            causal.any(i * 2, &alternatives);
        }
        if matches!(
            entry,
            Entry::Container(TimingContainer {
                duration: ContainerDuration::Automatic,
                ..
            })
        ) {
            end.extend(h.children[i].iter().map(|c| c * 2 + 1));
        }
        end.sort_unstable();
        end.dedup();
        for d in end {
            causal.require(i * 2 + 1, d);
        }
    }
    causal.validate(check)?;
    let domain = if tree.is_some() {
        "musteroffice.timeline-plan/0.2-draft"
    } else {
        "musteroffice.timeline-plan/0.1-draft"
    };
    let digest = mo_common::digest(domain, t)?;
    let initial_visibility = presentation::compile(t, &h, check)?;
    let motion_paths = crate::motion_path::compile(t, limits, check)?;
    let clocks = crate::plan::clocks(t, limits.max_exact_bits, check)?;
    let container_clocks = container_clock::compile(t, &h, &clocks, limits.max_exact_bits, check)?;
    cancel(check)?;
    Ok(TimelinePlan {
        timeline: t.clone(),
        interactive,
        restarting: (0..count).any(|i| {
            let e = Entry::at(t, i);
            !e.restart().is_never() || e.navigation().is_some()
        }),
        digest,
        limits,
        hierarchy: h,
        clocks,
        motion_paths,
        container_clocks,
        initial_visibility,
    })
}
