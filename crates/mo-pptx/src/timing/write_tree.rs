//! Write explicit native par/seq nesting, with checked exact millisecond values.
use super::*;
use mo_timeline::{ContainerDuration, ContainerKind};
pub(super) fn write(
    x: &mut Xml,
    t: &Timeline,
    objects: &BTreeMap<ObjectId, u32>,
    check: &dyn Fn() -> bool,
) -> Result<(), PptxError> {
    let tree = t.tree.as_ref().expect("tree writer");
    let ids: BTreeMap<_, _> = t
        .nodes
        .iter()
        .map(|n| &n.id)
        .chain(tree.containers.iter().map(|c| &c.id))
        .enumerate()
        .map(|(i, id)| {
            Ok((
                id,
                u32::try_from(i + 2).map_err(|_| PptxError::Limit("native timing IDs"))?,
            ))
        })
        .collect::<Result<_, PptxError>>()?;
    let leaves: BTreeMap<_, _> = t.nodes.iter().map(|n| (&n.id, n)).collect();
    let containers: BTreeMap<_, _> = tree.containers.iter().map(|c| (&c.id, c)).collect();
    enum Task<'a> {
        Open(&'a TimingNodeId),
        Close(ContainerKind, bool),
    }
    let mut stack: Vec<_> = tree.roots.iter().rev().map(Task::Open).collect();
    x.raw("<p:timing><p:tnLst><p:par><p:cTn id=\"1\" dur=\"indefinite\" restart=\"never\" nodeType=\"tmRoot\"><p:childTnLst>")?;
    while let Some(task) = stack.pop() {
        crate::cancelled(check)?;
        match task {
            Task::Close(kind, children) => {
                if children {
                    x.raw("</p:childTnLst>")?;
                }
                x.raw(match kind {
                    ContainerKind::Parallel => "</p:cTn></p:par>",
                    ContainerKind::Sequence => "</p:cTn></p:seq>",
                })?;
            }
            Task::Open(id) => {
                if let Some(n) = leaves.get(id) {
                    write_behavior(x, n, &ids, objects)?;
                    continue;
                }
                let c = containers[id];
                x.raw(match c.kind {
                    ContainerKind::Parallel => "<p:par><p:cTn",
                    ContainerKind::Sequence => "<p:seq><p:cTn",
                })?;
                x.attr("id", ids[id])?;
                match c.duration {
                    ContainerDuration::Fixed { duration } => {
                        x.attr("dur", milliseconds(duration)?)?
                    }
                    _ => x.attr("dur", "indefinite")?,
                }
                x.attr("restart", "never")?;
                x.attr(
                    "fill",
                    match c.fill {
                        FillMode::Remove => "remove",
                        FillMode::Freeze => "freeze",
                        FillMode::Hold => "hold",
                    },
                )?;
                write_start(x, &c.start, &ids, objects)?;
                write_ends(x, &c.end_conditions, &ids, objects)?;
                if c.duration == ContainerDuration::Automatic {
                    x.raw("<p:endSync evt=\"end\" delay=\"0\"><p:rtn val=\"all\"/></p:endSync>")?;
                }
                if !c.children.is_empty() {
                    x.raw("<p:childTnLst>")?;
                }
                stack.push(Task::Close(c.kind, !c.children.is_empty()));
                stack.extend(c.children.iter().rev().map(Task::Open));
            }
        }
    }
    x.raw("</p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>")
}
