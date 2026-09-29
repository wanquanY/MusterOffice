//! Native timing mapping. Unsupported trees stay in their original source package.
mod write_flat;
mod write_motion;
mod write_tree;
use crate::{PptxError, xml::Xml};
use mo_common::{ObjectId, RationalTime, TimingNodeId};
pub use mo_presentation_source::timing::*;
use mo_timeline::{
    Effect, FillMode, NodeEvent, RepeatCount, RepeatDuration, StartCondition, TimeCondition,
    Timeline,
};
use std::collections::BTreeMap;

fn write_time_transform(
    x: &mut Xml,
    transform: Option<mo_timeline::TimeTransform>,
) -> Result<(), PptxError> {
    if let Some(t) = transform {
        x.attr("spd", t.speed_milli_percent)?;
        x.attr("autoRev", if t.auto_reverse { "1" } else { "0" })?;
        x.attr("accel", t.acceleration_milli_percent)?;
        x.attr("decel", t.deceleration_milli_percent)?;
    }
    Ok(())
}

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
    document: &mo_presentation_model::Document,
    check: &dyn Fn() -> bool,
) -> Result<(), PptxError> {
    if timeline.node_count() == 0 {
        return Ok(());
    }
    if timeline.tree.is_some() {
        return write_tree::write(x, timeline, objects, document, check);
    }
    write_flat::write(x, timeline, objects, document, check)
}

fn write_behavior(
    x: &mut Xml,
    node: &mo_timeline::TimingNode,
    ids: &BTreeMap<&TimingNodeId, u32>,
    objects: &BTreeMap<ObjectId, u32>,
    document: &mo_presentation_model::Document,
) -> Result<(), PptxError> {
    match &node.effect {
        Effect::Rotation {
            from,
            to,
            composition,
            target,
        } => {
            x.raw("<p:animRot")?;
            if *composition == mo_timeline::RotationComposition::Add {
                if *from != 0 {
                    return Err(PptxError::Unsupported(
                        "native additive rotation requires a zero starting offset".into(),
                    ));
                }
                x.attr("by", to)?;
            } else {
                // Native endpoints are offsets from the original local angle.
                // Resolve the author's absolute domain while streaming this
                // node; no timeline clone or mutation of the document is needed.
                let base = if *composition == mo_timeline::RotationComposition::Absolute {
                    document
                        .objects
                        .get(target)
                        .and_then(|o| o.transform.as_ref())
                        .ok_or_else(|| {
                            PptxError::Unsupported(
                                "rotation requires a local object transform".into(),
                            )
                        })?
                        .normalized_rotation()
                } else {
                    0
                };
                let endpoint = |value: i32| {
                    i32::try_from(i64::from(value) - i64::from(base)).map_err(|_| {
                        PptxError::Unsupported(
                            "absolute rotation exceeds native endpoint range".into(),
                        )
                    })
                };
                x.attr("from", endpoint(*from)?)?;
                x.attr("to", endpoint(*to)?)?;
            }
        }
        Effect::Fade { transition, .. } => {
            x.raw("<p:animEffect filter=\"fade\"")?;
            x.attr(
                "transition",
                match transition {
                    mo_timeline::FadeTransition::In => "in",
                    mo_timeline::FadeTransition::Out => "out",
                },
            )?;
        }
        Effect::MotionPath { path, .. } => {
            x.raw("<p:animMotion origin=\"layout\" pathEditMode=\"relative\"")?;
            x.attr("path", write_motion::path(path))?;
        }
        Effect::MotionLine { from, to, .. } => {
            x.raw("<p:animMotion origin=\"layout\" pathEditMode=\"relative\"")?;
            x.attr(
                "path",
                format!(
                    "M {} {} L {} {} E",
                    from.x.lexical(),
                    from.y.lexical(),
                    to.x.lexical(),
                    to.y.lexical()
                ),
            )?;
        }
        Effect::Scale { .. } => x.raw("<p:animScale")?,
        Effect::SetVisibility { .. } => x.raw("<p:set")?,
    }
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
    x.attr("restart", restart(node.restart))?;
    x.attr(
        "fill",
        match node.fill {
            FillMode::Remove => "remove",
            FillMode::Freeze => "freeze",
            FillMode::Hold => "hold",
        },
    )?;
    write_time_transform(x, node.time_transform)?;
    write_start(x, &node.start, ids, objects)?;
    write_ends(x, &node.end_conditions, ids, objects)?;
    x.raw("</p:cTn><p:tgtEl><p:spTgt")?;
    x.attr("spid", objects[node.target()])?;
    x.raw("/></p:tgtEl>")?;
    if matches!(node.effect, Effect::Fade { .. }) {
        return x.raw("</p:cBhvr></p:animEffect>");
    }
    x.raw("<p:attrNameLst>")?;
    match &node.effect {
        Effect::Fade { .. } => unreachable!("fade closed above"),
        Effect::MotionLine { .. } | Effect::MotionPath { .. } => {
            x.raw("<p:attrName>ppt_x</p:attrName><p:attrName>ppt_y</p:attrName></p:attrNameLst></p:cBhvr></p:animMotion>")?;
        }
        Effect::SetVisibility { value, .. } => {
            x.raw("<p:attrName>style.visibility</p:attrName></p:attrNameLst></p:cBhvr><p:to><p:strVal")?;
            x.attr(
                "val",
                match value {
                    mo_timeline::Visibility::Visible => "visible",
                    mo_timeline::Visibility::Hidden => "hidden",
                },
            )?;
            x.raw("/></p:to></p:set>")?;
        }
        Effect::Rotation { .. } => {
            x.raw("<p:attrName>r</p:attrName></p:attrNameLst></p:cBhvr></p:animRot>")?
        }
        Effect::Scale { from, to, .. } => {
            x.raw("<p:attrName>ScaleX</p:attrName><p:attrName>ScaleY</p:attrName></p:attrNameLst></p:cBhvr>")?;
            // The native Grow/Shrink editor reads its size from `by`. In
            // by-only form the implicit start is exactly (100%, 100%); do not
            // also emit from/to, which would make the editor's by value inert.
            if from.x == 100_000 && from.y == 100_000 {
                x.raw("<p:by")?;
            } else {
                x.raw("<p:from")?;
                x.attr("x", from.x)?;
                x.attr("y", from.y)?;
                x.raw("/><p:to")?;
            }
            x.attr("x", to.x)?;
            x.attr("y", to.y)?;
            x.raw("/></p:animScale>")?;
        }
    }
    Ok(())
}

fn restart(mode: mo_timeline::RestartMode) -> &'static str {
    match mode {
        mo_timeline::RestartMode::Never => "never",
        mo_timeline::RestartMode::Always => "always",
        mo_timeline::RestartMode::WhenNotActive => "whenNotActive",
    }
}
fn write_start(
    x: &mut Xml,
    start: &StartCondition,
    ids: &BTreeMap<&TimingNodeId, u32>,
    objects: &BTreeMap<ObjectId, u32>,
) -> Result<(), PptxError> {
    x.raw("><p:stCondLst>")?;
    for condition in start.conditions() {
        write_condition(x, condition, ids, objects)?;
    }
    x.raw("</p:stCondLst>")
}
fn write_ends(
    x: &mut Xml,
    conditions: &[mo_timeline::TimeCondition],
    ids: &BTreeMap<&TimingNodeId, u32>,
    objects: &BTreeMap<ObjectId, u32>,
) -> Result<(), PptxError> {
    write_conditions(x, "endCondLst", conditions, ids, objects)
}

fn write_conditions(
    x: &mut Xml,
    name: &str,
    conditions: &[TimeCondition],
    ids: &BTreeMap<&TimingNodeId, u32>,
    objects: &BTreeMap<ObjectId, u32>,
) -> Result<(), PptxError> {
    if !conditions.is_empty() {
        x.raw(&format!("<p:{name}>"))?;
        for condition in conditions {
            write_condition(x, condition, ids, objects)?;
        }
        x.raw(&format!("</p:{name}>"))?;
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
        TimeCondition::Never {} => x.raw(" delay=\"indefinite\"/>")?,
        TimeCondition::At { offset } => {
            x.attr("delay", milliseconds(*offset)?)?;
            x.raw("/>")?;
        }
        TimeCondition::After { node, event, delay } => {
            // MS-OI29500 2.1.1196 distinguishes node lifecycle edges from
            // onBegin/onEnd events delivered to an event target.
            // Keep both identities; do not normalize a source notification or
            // an automatic presentation-group trigger into an interval edge.
            x.attr(
                "evt",
                match event {
                    NodeEvent::Begin => "begin",
                    NodeEvent::End => "end",
                    NodeEvent::OnBegin => "onBegin",
                    NodeEvent::OnEnd => "onEnd",
                },
            )?;
            x.attr("delay", milliseconds(*delay)?)?;
            x.raw("><p:tn")?;
            x.attr("val", ids[node])?;
            x.raw("/></p:cond>")?;
        }
        TimeCondition::Click { target, delay }
        | TimeCondition::Navigation { target, delay, .. } => {
            x.attr(
                "evt",
                match start {
                    TimeCondition::Navigation {
                        direction: mo_timeline::NavigationDirection::Next,
                        ..
                    } => "onNext",
                    TimeCondition::Navigation { .. } => "onPrev",
                    _ => "onClick",
                },
            )?;
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
