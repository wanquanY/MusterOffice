mod time_transform;
mod tree;
use crate::{P, PptxError};
use mo_common::{ObjectId, RationalTime, TimingNodeId};
use mo_timeline::{
    Effect, FillMode, NodeEvent, RepeatCount, RepeatDuration, StartCondition, Timeline,
    TimelineLimits, TimelinePlan, TimelineVersion, TimingNode,
};
use mo_xml::{Element, XmlEvent, XmlLimits};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeTimeline {
    pub root_id: u32,
    pub timeline: Timeline,
    pub node_bindings: BTreeMap<TimingNodeId, u32>,
    pub object_bindings: BTreeMap<ObjectId, u32>,
}
struct Node {
    element: Element,
    children: Vec<usize>,
    text: String,
}
fn unsupported(message: impl Into<String>) -> PptxError {
    PptxError::Unsupported(format!("native timing: {}", message.into()))
}
fn integer(node: &Node, name: &str) -> Result<u32, PptxError> {
    node.element
        .attribute(name)
        .ok_or_else(|| unsupported(format!("missing {name}")))?
        .trim()
        .parse()
        .map_err(|_| crate::value("timing", format!("invalid {name}")))
}
fn attrs(node: &Node, allowed: &[&str]) -> Result<(), PptxError> {
    if node
        .element
        .attributes
        .iter()
        .any(|a| !a.name.namespace.is_empty() || !allowed.contains(&a.name.local.as_str()))
    {
        return Err(unsupported(format!(
            "unmapped attributes on {}",
            node.element.name.local
        )));
    }
    Ok(())
}
fn value(node: &Node, name: &str, expected: &str) -> Result<(), PptxError> {
    if node.element.attribute(name) != Some(expected) {
        return Err(unsupported(format!(
            "{} {name} must be {expected}",
            node.element.name.local
        )));
    }
    Ok(())
}
fn children<'a>(tree: &'a [Node], node: &Node, names: &[&str]) -> Result<Vec<&'a Node>, PptxError> {
    if !node.text.trim().is_empty() || node.children.len() != names.len() {
        return Err(unsupported(format!(
            "unmapped content in {}",
            node.element.name.local
        )));
    }
    node.children
        .iter()
        .zip(names)
        .map(|(&i, name)| {
            let child = &tree[i];
            if !child.element.name.is(P, name) {
                Err(unsupported(format!("expected {name}")))
            } else {
                Ok(child)
            }
        })
        .collect()
}
fn single<'a>(tree: &'a [Node], node: &Node, name: &str) -> Result<&'a Node, PptxError> {
    Ok(children(tree, node, &[name])?[0])
}
fn empty(tree: &[Node], node: &Node) -> Result<(), PptxError> {
    children(tree, node, &[]).map(|_| ())
}
fn n_id(n: u32) -> TimingNodeId {
    TimingNodeId::new(format!("tn.{n}")).expect("integer timing identity")
}
fn o_id(n: u32) -> ObjectId {
    ObjectId::new(format!("sp.{n}")).expect("integer shape identity")
}
fn ms(n: u32) -> RationalTime {
    RationalTime::new(i64::from(n), 1000).expect("native timebase")
}
fn object(
    tree: &[Node],
    target: &Node,
    known: &BTreeSet<u32>,
    bindings: &mut BTreeMap<ObjectId, u32>,
) -> Result<ObjectId, PptxError> {
    attrs(target, &[])?;
    let shape = single(tree, target, "spTgt")?;
    attrs(shape, &["spid"])?;
    empty(tree, shape)?;
    let id = integer(shape, "spid")?;
    if !known.contains(&id) {
        return Err(crate::value(
            "timing/target",
            format!("native object {id} does not exist on this slide"),
        ));
    }
    let key = o_id(id);
    bindings.insert(key.clone(), id);
    Ok(key)
}

/// Reads the explicitly implemented once-activation rotation graph. Every
/// unmapped timing node/attribute/condition fails the projection; original OPC
/// content remains owned by the source layer. No private author marker required.
pub fn read_slide_timing(
    xml: &[u8],
    known_objects: &BTreeSet<u32>,
    xml_limits: XmlLimits,
    timing_limits: TimelineLimits,
    check: &dyn Fn() -> bool,
) -> Result<Option<NativeTimeline>, PptxError> {
    let mut tree: Vec<Node> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    let mut capture = false;
    let mut unsupported_location = false;
    let mut root_seen = false;
    let capture_limit = timing_limits
        .max_nodes
        .saturating_mul(16)
        .saturating_add(16);
    mo_xml::scan_with_control(xml, xml_limits, check, |event| {
        match event {
            XmlEvent::Start { element, depth, .. } => {
                if depth == 0 {
                    if !element.name.is(P, "sld") {
                        return Err(mo_xml::XmlError::Malformed(
                            "expected slide root for timing".into(),
                        ));
                    }
                    root_seen = true;
                }
                if element.name.is(P, "timing") {
                    if depth != 1 || !tree.is_empty() {
                        unsupported_location = true;
                    } else {
                        capture = true;
                    }
                }
                if capture {
                    if tree.len() >= capture_limit {
                        return Err(mo_xml::XmlError::Limit("captured timing elements"));
                    }
                    let i = tree.len();
                    tree.push(Node {
                        element: element.clone(),
                        children: vec![],
                        text: String::new(),
                    });
                    if let Some(&parent) = stack.last() {
                        tree[parent].children.push(i);
                    }
                    stack.push(i);
                }
            }
            XmlEvent::Text { text, .. } if capture => {
                if let Some(&i) = stack.last() {
                    tree[i].text.push_str(text);
                }
            }
            XmlEvent::End { .. } if capture => {
                stack.pop();
                if stack.is_empty() {
                    capture = false;
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    if !root_seen {
        return Err(crate::value("timing", "missing slide root"));
    }
    if unsupported_location {
        return Err(unsupported(
            "nested, duplicate or compatibility-wrapped timing",
        ));
    }
    if tree.is_empty() {
        return Ok(None);
    }
    let timing = &tree[0];
    attrs(timing, &[])?;
    let list = single(&tree, timing, "tnLst")?;
    attrs(list, &[])?;
    let par = single(&tree, list, "par")?;
    attrs(par, &[])?;
    let root = single(&tree, par, "cTn")?;
    attrs(root, &["id", "dur", "restart", "nodeType"])?;
    value(root, "dur", "indefinite")?;
    value(root, "restart", "never")?;
    value(root, "nodeType", "tmRoot")?;
    let root_id = integer(root, "id")?;
    let list = single(&tree, root, "childTnLst")?;
    attrs(list, &[])?;
    if !list.text.trim().is_empty() || list.children.is_empty() {
        return Err(unsupported("empty or textual child timing list"));
    }
    if list
        .children
        .iter()
        .any(|&i| tree[i].element.name.is(P, "par") || tree[i].element.name.is(P, "seq"))
    {
        return tree::read_tree(&tree, list, known_objects, root_id, timing_limits, check)
            .map(Some);
    }
    let mut nodes = Vec::new();
    let mut node_bindings = BTreeMap::new();
    let mut object_bindings = BTreeMap::new();
    for &index in &list.children {
        crate::cancelled(check)?;
        if nodes.len() >= timing_limits.max_nodes {
            return Err(PptxError::Limit("timing node count"));
        }
        nodes.push(read_rotation(
            &tree,
            &tree[index],
            known_objects,
            root_id,
            &mut node_bindings,
            &mut object_bindings,
            check,
        )?);
    }
    let timeline = Timeline {
        format: TimelineVersion::V01,
        tree: None,
        nodes,
    };
    TimelinePlan::compile(&timeline, timing_limits, check).map_err(|e| match e {
        mo_timeline::TimelineError::Cancelled => PptxError::Cancelled,
        mo_timeline::TimelineError::Limit(reason) => PptxError::Limit(reason),
        other => crate::value("timing", other.to_string()),
    })?;
    Ok(Some(NativeTimeline {
        root_id,
        timeline,
        node_bindings,
        object_bindings,
    }))
}

fn read_rotation(
    tree: &[Node],
    anim: &Node,
    known_objects: &BTreeSet<u32>,
    root_id: u32,
    node_bindings: &mut BTreeMap<TimingNodeId, u32>,
    object_bindings: &mut BTreeMap<ObjectId, u32>,
    check: &dyn Fn() -> bool,
) -> Result<TimingNode, PptxError> {
    if !anim.element.name.is(P, "animRot") {
        return Err(unsupported(format!("behavior {}", anim.element.name.local)));
    }
    attrs(anim, &["from", "to"])?;
    let angle = |name| {
        anim.element
            .attribute(name)
            .ok_or_else(|| unsupported(format!("missing rotation {name}")))?
            .parse::<i32>()
            .map_err(|_| crate::value("timing/rotation", "invalid angle"))
    };
    let from = angle("from")?;
    let to = angle("to")?;
    let behavior = single(tree, anim, "cBhvr")?;
    attrs(behavior, &["additive", "accumulate", "xfrmType"])?;
    value(behavior, "additive", "repl")?;
    value(behavior, "accumulate", "none")?;
    value(behavior, "xfrmType", "pt")?;
    let parts = children(tree, behavior, &["cTn", "tgtEl", "attrNameLst"])?;
    let common = parts[0];
    attrs(
        common,
        &[
            "id",
            "dur",
            "repeatCount",
            "repeatDur",
            "restart",
            "fill",
            "spd",
            "autoRev",
            "accel",
            "decel",
        ],
    )?;
    value(common, "restart", "never")?;
    let native = integer(common, "id")?;
    let id = n_id(native);
    if native == root_id || node_bindings.insert(id.clone(), native).is_some() {
        return Err(crate::value("timing/id", "duplicate timing identity"));
    }
    let duration = ms(integer(common, "dur")?);
    let repeat_milli = match common.element.attribute("repeatCount").map(str::trim) {
        Some("indefinite") => RepeatCount::Indefinite,
        Some(_) => RepeatCount::Finite(integer(common, "repeatCount")?),
        None => RepeatCount::Finite(1000),
    };
    let repeat_duration = match common.element.attribute("repeatDur").map(str::trim) {
        Some("indefinite") => Some(RepeatDuration::Indefinite),
        Some(_) => Some(RepeatDuration::Finite(ms(integer(common, "repeatDur")?))),
        None => None,
    };
    let fill = match common.element.attribute("fill") {
        Some("remove") => FillMode::Remove,
        Some("freeze") => FillMode::Freeze,
        Some("hold") => FillMode::Hold,
        _ => return Err(unsupported("fill mode")),
    };
    let lists = children(
        tree,
        common,
        if common.children.len() == 2 {
            &["stCondLst", "endCondLst"]
        } else {
            &["stCondLst"]
        },
    )?;
    let start_list = lists[0];
    let start = read_start(tree, start_list, known_objects, object_bindings)?;
    let end_conditions = lists
        .get(1)
        .map(|list| read_ends(tree, list, known_objects, object_bindings, check))
        .transpose()?
        .unwrap_or_default();
    let target = object(tree, parts[1], known_objects, object_bindings)?;
    attrs(parts[2], &[])?;
    let property = single(tree, parts[2], "attrName")?;
    attrs(property, &[])?;
    if !property.children.is_empty() || property.text.trim() != "r" {
        return Err(unsupported("rotation property name"));
    }
    Ok(TimingNode {
        id,
        start,
        duration,
        repeat_milli,
        end_conditions,
        repeat_duration,
        fill,
        time_transform: time_transform::read(common)?,
        effect: Effect::Rotation { target, from, to },
    })
}

fn read_start(
    tree: &[Node],
    start_list: &Node,
    known_objects: &BTreeSet<u32>,
    object_bindings: &mut BTreeMap<ObjectId, u32>,
) -> Result<StartCondition, PptxError> {
    attrs(start_list, &[])?;
    let condition = single(tree, start_list, "cond")?;
    read_condition(tree, condition, known_objects, object_bindings)
}
fn read_ends(
    tree: &[Node],
    list: &Node,
    known_objects: &BTreeSet<u32>,
    object_bindings: &mut BTreeMap<ObjectId, u32>,
    check: &dyn Fn() -> bool,
) -> Result<Vec<mo_timeline::TimeCondition>, PptxError> {
    attrs(list, &[])?;
    if list.children.is_empty() || !list.text.trim().is_empty() {
        return Err(unsupported("empty or textual end condition list"));
    }
    list.children
        .iter()
        .map(|&i| {
            crate::cancelled(check)?;
            if !tree[i].element.name.is(P, "cond") {
                return Err(unsupported("end condition child"));
            }
            read_condition(tree, &tree[i], known_objects, object_bindings)
        })
        .collect()
}

fn read_condition(
    tree: &[Node],
    condition: &Node,
    known_objects: &BTreeSet<u32>,
    object_bindings: &mut BTreeMap<ObjectId, u32>,
) -> Result<mo_timeline::TimeCondition, PptxError> {
    attrs(condition, &["evt", "delay"])?;
    let delay = ms(integer(condition, "delay")?);
    Ok(match condition.element.attribute("evt") {
        None => {
            empty(tree, condition)?;
            StartCondition::At { offset: delay }
        }
        Some("onBegin" | "onEnd") => {
            let target = single(tree, condition, "tn")?;
            attrs(target, &["val"])?;
            empty(tree, target)?;
            StartCondition::After {
                node: n_id(integer(target, "val")?),
                event: if condition.element.attribute("evt") == Some("onBegin") {
                    NodeEvent::Begin
                } else {
                    NodeEvent::End
                },
                delay,
            }
        }
        Some("onClick") => {
            let target = single(tree, condition, "tgtEl")?;
            attrs(target, &[])?;
            let slide_target =
                target.children.len() == 1 && tree[target.children[0]].element.name.is(P, "sldTgt");
            let target = if slide_target {
                let s = single(tree, target, "sldTgt")?;
                attrs(s, &[])?;
                empty(tree, s)?;
                None
            } else {
                Some(object(tree, target, known_objects, object_bindings)?)
            };
            StartCondition::Click { target, delay }
        }
        Some(_) => return Err(unsupported("start event")),
    })
}
