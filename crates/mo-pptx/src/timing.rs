//! Native timing mapping. Unsupported trees stay in their original source package.
mod read;
mod source;
mod write_tree;
use crate::{PptxError, xml::Xml};
use mo_common::{ObjectId, RationalTime, TimingNodeId};
use mo_timeline::{
    Effect, FillMode, NodeEvent, RepeatCount, RepeatDuration, StartCondition, Timeline,
};
pub use read::*;
pub use source::*;
use std::collections::BTreeMap;

fn milliseconds(time: RationalTime) -> Result<u32, PptxError> {
    let n = i128::from(time.ticks.get()) * 1000;
    let d = i128::from(time.timescale.get());
    if n < 0 || n % d != 0 {
        return Err(PptxError::Unsupported(
            "timing value is not exactly representable as native milliseconds".into(),
        ));
    }
    u32::try_from(n / d).map_err(|_| {
        PptxError::Unsupported("timing value exceeds native unsigned milliseconds".into())
    })
}
pub(crate) fn write(
    x: &mut Xml,
    timeline: &Timeline,
    objects: &BTreeMap<ObjectId, u32>,
    check: &dyn Fn() -> bool,
) -> Result<(), PptxError> {
    if timeline.node_count() == 0 {
        return Ok(());
    }
    if timeline.tree.is_some() {
        return write_tree::write(x, timeline, objects, check);
    }
    let ids: BTreeMap<&TimingNodeId, u32> = timeline
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            Ok((
                &n.id,
                u32::try_from(i + 2).map_err(|_| PptxError::Limit("native timing IDs"))?,
            ))
        })
        .collect::<Result<_, PptxError>>()?;
    x.raw("<p:timing><p:tnLst><p:par><p:cTn id=\"1\" dur=\"indefinite\" restart=\"never\" nodeType=\"tmRoot\"><p:childTnLst>")?;
    for node in &timeline.nodes {
        crate::cancelled(check)?;
        write_behavior(x, node, &ids, objects)?;
    }
    x.raw("</p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>")
}

fn write_behavior(
    x: &mut Xml,
    node: &mo_timeline::TimingNode,
    ids: &BTreeMap<&TimingNodeId, u32>,
    objects: &BTreeMap<ObjectId, u32>,
) -> Result<(), PptxError> {
    let Effect::Rotation { target, from, to } = &node.effect;
    x.raw("<p:animRot")?;
    x.attr("from", from)?;
    x.attr("to", to)?;
    x.raw("><p:cBhvr additive=\"repl\" accumulate=\"none\" xfrmType=\"pt\"><p:cTn")?;
    x.attr("id", ids[&node.id])?;
    x.attr("dur", milliseconds(node.duration)?)?;
    match node.repeat_milli {
        RepeatCount::Finite(count) => x.attr("repeatCount", count)?,
        RepeatCount::Indefinite => x.attr("repeatCount", "indefinite")?,
    }
    match node.repeat_duration {
        Some(RepeatDuration::Finite(duration)) => x.attr("repeatDur", milliseconds(duration)?)?,
        Some(RepeatDuration::Indefinite) => x.attr("repeatDur", "indefinite")?,
        None => (),
    }
    x.attr("restart", "never")?;
    x.attr(
        "fill",
        match node.fill {
            FillMode::Remove => "remove",
            FillMode::Freeze => "freeze",
            FillMode::Hold => "hold",
        },
    )?;
    if let Some(t) = node.time_transform {
        x.attr("spd", t.speed_milli_percent)?;
        x.attr("autoRev", if t.auto_reverse { "1" } else { "0" })?;
        x.attr("accel", t.acceleration_milli_percent)?;
        x.attr("decel", t.deceleration_milli_percent)?;
    }
    write_start(x, &node.start, ids, objects)?;
    write_ends(x, &node.end_conditions, ids, objects)?;
    x.raw("</p:cTn><p:tgtEl><p:spTgt")?;
    x.attr("spid", objects[target])?;
    x.raw("/></p:tgtEl><p:attrNameLst><p:attrName>r</p:attrName></p:attrNameLst></p:cBhvr></p:animRot>")?;
    Ok(())
}

fn write_start(
    x: &mut Xml,
    start: &StartCondition,
    ids: &BTreeMap<&TimingNodeId, u32>,
    objects: &BTreeMap<ObjectId, u32>,
) -> Result<(), PptxError> {
    x.raw("><p:stCondLst>")?;
    write_condition(x, start, ids, objects)?;
    x.raw("</p:stCondLst>")
}
fn write_ends(
    x: &mut Xml,
    conditions: &[mo_timeline::TimeCondition],
    ids: &BTreeMap<&TimingNodeId, u32>,
    objects: &BTreeMap<ObjectId, u32>,
) -> Result<(), PptxError> {
    if !conditions.is_empty() {
        x.raw("<p:endCondLst>")?;
        for condition in conditions {
            write_condition(x, condition, ids, objects)?;
        }
        x.raw("</p:endCondLst>")?;
    }
    Ok(())
}

fn write_condition(
    x: &mut Xml,
    start: &mo_timeline::TimeCondition,
    ids: &BTreeMap<&TimingNodeId, u32>,
    objects: &BTreeMap<ObjectId, u32>,
) -> Result<(), PptxError> {
    x.raw("<p:cond")?;
    match start {
        StartCondition::At { offset } => {
            x.attr("delay", milliseconds(*offset)?)?;
            x.raw("/>")?;
        }
        StartCondition::After { node, event, delay } => {
            x.attr(
                "evt",
                match event {
                    NodeEvent::Begin => "onBegin",
                    NodeEvent::End => "onEnd",
                },
            )?;
            x.attr("delay", milliseconds(*delay)?)?;
            x.raw("><p:tn")?;
            x.attr("val", ids[node])?;
            x.raw("/></p:cond>")?;
        }
        StartCondition::Click { target, delay } => {
            x.attr("evt", "onClick")?;
            x.attr("delay", milliseconds(*delay)?)?;
            x.raw("><p:tgtEl>")?;
            if let Some(target) = target {
                x.raw("<p:spTgt")?;
                x.attr("spid", objects[target])?;
                x.raw("/>")?;
            } else {
                x.raw("<p:sldTgt/>")?;
            }
            x.raw("</p:tgtEl></p:cond>")?;
        }
    }
    Ok(())
}
