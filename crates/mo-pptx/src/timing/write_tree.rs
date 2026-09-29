//! Write explicit native par/seq nesting, with checked exact millisecond values.
use super::*;
use mo_timeline::{
    ContainerDuration, ContainerKind, NextAction, PresentationRole, PresentationTrigger,
    PreviousAction, TimingContainer,
};
pub(super) fn write(
    x: &mut Xml,
    t: &Timeline,
    objects: &BTreeMap<ObjectId, u32>,
    document: &mo_presentation_model::Document,
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
        Close(&'a TimingContainer),
    }
    let mut stack: Vec<_> = tree.roots.iter().rev().map(Task::Open).collect();
    x.raw("<p:timing><p:tnLst><p:par><p:cTn id=\"1\" dur=\"indefinite\" restart=\"never\" nodeType=\"tmRoot\"><p:childTnLst>")?;
    while let Some(task) = stack.pop() {
        crate::cancelled(check)?;
        match task {
            Task::Close(c) => {
                if !c.children.is_empty() {
                    x.raw("</p:childTnLst>")?;
                }
                x.raw("</p:cTn>")?;
                if let Some(nav) = &c.navigation {
                    write_conditions(x, "prevCondLst", &nav.previous_conditions, &ids, objects)?;
                    write_conditions(x, "nextCondLst", &nav.next_conditions, &ids, objects)?;
                }
                x.raw(match c.kind {
                    ContainerKind::Parallel => "</p:par>",
                    ContainerKind::Sequence => "</p:seq>",
                })?;
            }
            Task::Open(id) => {
                if let Some(n) = leaves.get(id) {
                    write_behavior(x, n, &ids, objects, document)?;
                    continue;
                }
                let c = containers[id];
                match c.kind {
                    ContainerKind::Parallel => x.raw("<p:par><p:cTn")?,
                    ContainerKind::Sequence => {
                        x.raw("<p:seq")?;
                        if let Some(nav) = &c.navigation {
                            x.attr("concurrent", if nav.concurrent { "1" } else { "0" })?;
                            x.attr(
                                "nextAc",
                                match nav.next_action {
                                    NextAction::None => "none",
                                    NextAction::Seek => "seek",
                                },
                            )?;
                            x.attr(
                                "prevAc",
                                match nav.previous_action {
                                    PreviousAction::None => "none",
                                    PreviousAction::SkipTimed => "skipTimed",
                                },
                            )?;
                        }
                        x.raw("><p:cTn")?;
                    }
                }
                x.attr("id", ids[id])?;
                match c.presentation {
                    None => (),
                    Some(PresentationRole::MainSequence) => x.attr("nodeType", "mainSeq")?,
                    Some(PresentationRole::Effect { preset, trigger }) => {
                        let (id, class) = mo_presentation_source::timing::native_preset(preset);
                        x.attr("presetID", id)?;
                        x.attr("presetClass", class)?;
                        x.attr("presetSubtype", "0")?;
                        x.attr(
                            "nodeType",
                            match trigger {
                                PresentationTrigger::Click => "clickEffect",
                                PresentationTrigger::WithPrevious => "withEffect",
                                PresentationTrigger::AfterPrevious => "afterEffect",
                            },
                        )?;
                    }
                }
                match c.duration {
                    ContainerDuration::Fixed { duration } => {
                        x.attr("dur", milliseconds(duration)?)?
                    }
                    _ => x.attr("dur", "indefinite")?,
                }
                x.attr("restart", restart(c.restart))?;
                write_time_transform(x, c.time_transform)?;
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
                stack.push(Task::Close(c));
                stack.extend(c.children.iter().rev().map(Task::Open));
            }
        }
    }
    x.raw("</p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>")
}
