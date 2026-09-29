//! Lossless typed hierarchy projection for the implemented timing semantics.
use super::*;
use mo_timeline::{ContainerDuration, ContainerKind, TimingContainer, TimingTree};

fn fill(common: &Node) -> Result<FillMode, PptxError> {
    match common.element.attribute("fill") {
        Some("remove") => Ok(FillMode::Remove),
        Some("freeze") => Ok(FillMode::Freeze),
        Some("hold") => Ok(FillMode::Hold),
        _ => Err(unsupported("container fill mode")),
    }
}
pub(super) fn read_tree(
    tree: &[Node],
    list: &Node,
    known: &BTreeSet<u32>,
    root_id: u32,
    limits: TimelineLimits,
    check: &dyn Fn() -> bool,
) -> Result<NativeTimeline, PptxError> {
    let mut nodes = vec![];
    let mut containers: Vec<TimingContainer> = vec![];
    let mut roots = vec![];
    let mut node_bindings = BTreeMap::new();
    let mut object_bindings = BTreeMap::new();
    let mut stack: Vec<_> = list
        .children
        .iter()
        .rev()
        .map(|&i| (i, None, 1usize))
        .collect();
    while let Some((index, parent, depth)) = stack.pop() {
        crate::cancelled(check)?;
        if node_bindings.len() >= limits.max_nodes {
            return Err(PptxError::Limit("timing node count"));
        }
        if depth > limits.max_depth {
            return Err(PptxError::Limit("timing tree nesting"));
        }
        let node = &tree[index];
        let id = if node.element.name.is(P, "animRot")
            || node.element.name.is(P, "animScale")
            || node.element.name.is(P, "set")
            || node.element.name.is(P, "animEffect")
            || node.element.name.is(P, "animMotion")
        {
            let n = read_behavior(
                tree,
                node,
                known,
                root_id,
                &mut node_bindings,
                &mut object_bindings,
                check,
            )?;
            let id = n.id.clone();
            nodes.push(n);
            id
        } else if node.element.name.is(P, "par") || node.element.name.is(P, "seq") {
            let kind = if node.element.name.is(P, "seq") {
                ContainerKind::Sequence
            } else {
                ContainerKind::Parallel
            };
            let (common, navigation) = if kind == ContainerKind::Sequence {
                sequence::read(tree, node, known, &mut object_bindings, check)?
            } else {
                attrs(node, &[])?;
                (single(tree, node, "cTn")?, None)
            };
            attrs(
                common,
                &[
                    "id",
                    "dur",
                    "restart",
                    "fill",
                    "nodeType",
                    "presetID",
                    "presetClass",
                    "presetSubtype",
                    "spd",
                    "autoRev",
                    "accel",
                    "decel",
                ],
            )?;
            let restart = read_restart(common)?;
            let native = integer(common, "id")?;
            let id = n_id(native);
            if native == root_id || node_bindings.insert(id.clone(), native).is_some() {
                return Err(crate::value("timing/id", "duplicate timing identity"));
            }
            let mut position = 0;
            let mut take = |name: &str| -> Option<&Node> {
                let child = common.children.get(position).map(|&i| &tree[i])?;
                if child.element.name.is(P, name) {
                    position += 1;
                    Some(child)
                } else {
                    None
                }
            };
            let start = take("stCondLst")
                .map(|s| read_start(tree, s, known, &mut object_bindings, check))
                .transpose()?
                .unwrap_or_else(|| TimeCondition::At { offset: ms(0) }.into());
            let end_conditions = take("endCondLst")
                .map(|list| read_conditions(tree, list, known, &mut object_bindings, check))
                .transpose()?
                .unwrap_or_default();
            let sync = take("endSync");
            let children = take("childTnLst");
            if position != common.children.len() || !common.text.trim().is_empty() {
                return Err(unsupported("container common content"));
            }
            let presentation = presentation::validate(tree, common, kind, children)?;
            let duration = match (common.element.attribute("dur"), sync) {
                // A presentation container without an explicit duration owns
                // the active span of its children, including delayed children.
                (None, None) => ContainerDuration::Automatic,
                (Some("indefinite"), Some(sync)) => {
                    attrs(sync, &["evt", "delay"])?;
                    value(sync, "evt", "end")?;
                    value(sync, "delay", "0")?;
                    let runtime = single(tree, sync, "rtn")?;
                    attrs(runtime, &["val"])?;
                    value(runtime, "val", "all")?;
                    empty(tree, runtime)?;
                    ContainerDuration::Automatic
                }
                (Some("indefinite"), None) => ContainerDuration::Indefinite,
                (Some(_), None) => ContainerDuration::Fixed {
                    duration: ms(integer(common, "dur")?),
                },
                _ => return Err(unsupported("container duration/endSync combination")),
            };
            let owner = containers.len();
            // An unending root main sequence has no observable post-end fill.
            // Normalize only that provably equivalent case; omitted fill on a
            // finite or nested container remains unsupported.
            let fill = if common.element.attribute("fill").is_none()
                && presentation == Some(mo_timeline::PresentationRole::MainSequence)
                && parent.is_none()
                && duration == ContainerDuration::Indefinite
                && end_conditions.is_empty()
            {
                FillMode::Remove
            } else {
                fill(common)?
            };
            containers.push(TimingContainer {
                presentation,
                navigation,
                id: id.clone(),
                restart,
                kind,
                start,
                end_conditions,
                duration,
                time_transform: time_transform::read(common)?,
                fill,
                children: vec![],
            });
            if let Some(children) = children {
                attrs(children, &[])?;
                if children.children.is_empty() || !children.text.trim().is_empty() {
                    return Err(unsupported("textual child timing list"));
                }
                stack.extend(
                    children
                        .children
                        .iter()
                        .rev()
                        .map(|&i| (i, Some(owner), depth + 1)),
                );
            }
            id
        } else {
            return Err(unsupported(format!("behavior {}", node.element.name.local)));
        };
        if let Some(parent) = parent {
            containers[parent].children.push(id);
        } else {
            roots.push(id);
        }
    }
    let timeline = Timeline {
        format: TimelineVersion::V02,
        nodes,
        tree: Some(TimingTree { roots, containers }),
    };
    TimelinePlan::compile(&timeline, limits, check).map_err(|e| match e {
        mo_timeline::TimelineError::Cancelled => PptxError::Cancelled,
        mo_timeline::TimelineError::Limit(s) => PptxError::Limit(s),
        e => crate::value("timing", e.to_string()),
    })?;
    Ok(NativeTimeline {
        root_id,
        timeline,
        node_bindings,
        object_bindings,
    })
}
