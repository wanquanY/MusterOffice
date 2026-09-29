use crate::{DeletePolicy, EditError};
use mo_common::ObjectId;
use mo_presentation_model::Document;
use mo_timeline::{TimeCondition, Timeline};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub(crate) fn delete_references(
    document: &mut Document,
    objects: &BTreeSet<ObjectId>,
    policy: DeletePolicy,
) -> Result<(), EditError> {
    let mut empty = Vec::new();
    for (slide, timeline) in &mut document.timelines {
        let clicked =
            |condition: &TimeCondition| condition.target().is_some_and(|id| objects.contains(id));
        let mut remove: BTreeSet<_> = timeline
            .nodes
            .iter()
            .filter(|node| objects.contains(node.target()) || node.conditions().any(&clicked))
            .map(|n| n.id.clone())
            .collect();
        if let Some(tree) = &timeline.tree {
            remove.extend(
                tree.containers
                    .iter()
                    .filter(|c| c.conditions().any(&clicked))
                    .map(|c| c.id.clone()),
            );
        }
        if remove.is_empty() {
            continue;
        }
        if policy == DeletePolicy::RejectDependencies {
            return Err(EditError::ReferenceConflict(
                "animation depends on deleted object".into(),
            ));
        }
        // Structural descendants and explicit event dependents form one closure.
        // Removing a container must never promote children into a different clock.
        let dependents = dependents(timeline);
        let mut pending: VecDeque<_> = remove.iter().cloned().collect();
        while let Some(id) = pending.pop_front() {
            for next in dependents.get(&id).into_iter().flatten() {
                if remove.insert((*next).clone()) {
                    pending.push_back((*next).clone());
                }
            }
        }
        // Empty containers are meaningful timers (including event dependency
        // targets). Keep their identity, duration and scope; only prune edges to
        // removed entries. Sequential survivors keep their remaining order.
        if let Some(tree) = &mut timeline.tree {
            tree.roots.retain(|id| !remove.contains(id));
            tree.containers.retain(|c| !remove.contains(&c.id));
            for c in &mut tree.containers {
                c.children.retain(|id| !remove.contains(id));
                if c.children.is_empty()
                    && matches!(
                        c.presentation,
                        Some(mo_timeline::PresentationRole::Effect { .. })
                    )
                {
                    // Retain the timing identity for surviving dependencies,
                    // but an emptied effect no longer has an editorial preset.
                    c.presentation = None;
                }
            }
        }
        timeline.nodes.retain(|n| !remove.contains(&n.id));
        if timeline.node_count() == 0 {
            empty.push(slide.clone());
        }
    }
    for slide in empty {
        document.timelines.remove(&slide);
    }
    Ok(())
}

fn dependents(
    timeline: &Timeline,
) -> BTreeMap<&mo_common::TimingNodeId, Vec<&mo_common::TimingNodeId>> {
    let mut out: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for node in &timeline.nodes {
        for parent in node
            .conditions()
            .filter_map(mo_timeline::TimeCondition::dependency)
        {
            out.entry(parent).or_default().push(&node.id);
        }
    }
    if let Some(tree) = &timeline.tree {
        for container in &tree.containers {
            for node in container
                .conditions()
                .filter_map(mo_timeline::TimeCondition::dependency)
            {
                out.entry(node).or_default().push(&container.id);
            }
            out.entry(&container.id)
                .or_default()
                .extend(&container.children);
        }
    }
    out
}
