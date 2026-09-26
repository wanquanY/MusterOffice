//! Structural ownership and event dependency DAG. Containers never flatten into
//! synthetic absolute leaf offsets; begin/end events retain their native scopes.
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
    dependencies: Vec<Option<usize>>,
    end_dependencies: Vec<Vec<Option<usize>>>,
    traversal: Vec<usize>,
    listeners: Vec<Vec<(usize, Option<usize>)>>,
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
        conditions = conditions
            .saturating_add(1)
            .saturating_add(entry.ends().len());
        if conditions > limits.max_conditions {
            return Err(TimelineError::Limit("timing condition count"));
        }
        for (position, condition) in std::iter::once(entry.start())
            .chain(entry.ends())
            .enumerate()
        {
            cancel(check)?;
            let delay = match condition {
                TimeCondition::At { offset } => offset,
                TimeCondition::After { delay, .. } => delay,
                TimeCondition::Click { delay, .. } => {
                    interactive = true;
                    delay
                }
            };
            if delay.ticks.get() < 0 {
                return Err(invalid(
                    Some(entry.id()),
                    if position == 0 {
                        "negative start offset"
                    } else {
                        "negative end offset"
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
        dependencies: vec![None; count],
        end_dependencies: vec![vec![]; count],
        traversal: vec![],
        listeners: vec![vec![]; count * 2],
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
        for id in &container.children {
            cancel(check)?;
            h.children[i].push(assign(id, Some(i))?);
        }
        if container.kind == ContainerKind::Sequence {
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
    let mut incoming = vec![0usize; count * 2];
    let mut dependents = vec![vec![]; count * 2];
    for i in 0..count {
        cancel(check)?;
        let entry = Entry::at(t, i);
        let mut begin = vec![];
        if let Some(p) = h.parents[i] {
            begin.push(p * 2);
        }
        if let Some(p) = h.previous[i] {
            begin.push(p * 2 + 1);
        }
        if let StartCondition::After { node, event, .. } = entry.start() {
            let d = *ids
                .get(node)
                .ok_or_else(|| invalid(Some(entry.id()), "missing timing dependency"))?;
            h.dependencies[i] = Some(d);
            let vertex = d * 2 + usize::from(*event == NodeEvent::End);
            begin.push(vertex);
            h.listeners[vertex].push((i, None));
        }
        let mut end = vec![i * 2];
        for (position, condition) in entry.ends().iter().enumerate() {
            cancel(check)?;
            let dependency = if let TimeCondition::After { node, event, .. } = condition {
                let d = *ids
                    .get(node)
                    .ok_or_else(|| invalid(Some(entry.id()), "missing end timing dependency"))?;
                let vertex = d * 2 + usize::from(*event == NodeEvent::End);
                end.push(vertex);
                h.listeners[vertex].push((i, Some(position)));
                Some(d)
            } else {
                None
            };
            h.end_dependencies[i].push(dependency);
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
        for (v, mut deps) in [(i * 2, begin), (i * 2 + 1, end)] {
            deps.sort_unstable();
            deps.dedup();
            incoming[v] = deps.len();
            for d in deps {
                dependents[d].push(v);
            }
        }
    }
    let mut ready: VecDeque<_> = incoming
        .iter()
        .enumerate()
        .filter(|(_, n)| **n == 0)
        .map(|(i, _)| i)
        .collect();
    let mut visited = 0;
    while let Some(v) = ready.pop_front() {
        cancel(check)?;
        visited += 1;
        for &d in &dependents[v] {
            incoming[d] -= 1;
            if incoming[d] == 0 {
                ready.push_back(d);
            }
        }
    }
    if visited != count * 2 {
        return Err(invalid(None, "cyclic timing begin/end dependencies"));
    }
    let domain = if tree.is_some() {
        "musteroffice.timeline-plan/0.2-draft"
    } else {
        "musteroffice.timeline-plan/0.1-draft"
    };
    let digest = mo_common::digest(domain, t)?;
    let clocks = crate::plan::clocks(t, limits.max_exact_bits, check)?;
    cancel(check)?;
    Ok(TimelinePlan {
        timeline: t.clone(),
        interactive,
        digest,
        limits,
        hierarchy: h,
        clocks,
    })
}
