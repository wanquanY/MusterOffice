mod behavior;
mod condition;
mod envelope;
mod presentation;
mod sequence;
mod time_transform;
mod tree;
use crate::{P, PptxError};
use mo_common::{ObjectId, RationalTime, TimingNodeId};
use mo_timeline::{
    FillMode, NodeEvent, RepeatCount, RepeatDuration, StartCondition, TimeCondition, Timeline,
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
/// Optional native children are still ordered, unique, and exhaustive.
fn ordered<'a>(
    tree: &'a [Node],
    node: &Node,
    names: &[(&str, bool)],
) -> Result<Vec<Option<&'a Node>>, PptxError> {
    let mut position = 0;
    let mut result = Vec::with_capacity(names.len());
    for &(name, required) in names {
        let child = node
            .children
            .get(position)
            .map(|&i| &tree[i])
            .filter(|n| n.element.name.is(P, name));
        if child.is_some() {
            position += 1;
        } else if required {
            return Err(unsupported(format!("missing {name}")));
        }
        result.push(child);
    }
    if position != node.children.len() || !node.text.trim().is_empty() {
        return Err(unsupported(format!(
            "unmapped content in {}",
            node.element.name.local
        )));
    }
    Ok(result)
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

/// Reads the implemented transform graph and scoped sequence navigation. Every
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
    let structural_limit = timing_limits
        .max_nodes
        // Bounded XML capture includes native grouping overhead as well as
        // behaviors. Semantic node/condition limits are checked after lifting.
        .saturating_mul(32)
        .saturating_add(32);
    // Conditions cannot consume the structural quota or grant more room to
    // unrelated XML. Allow grouping conditions before neutral envelope lifting;
    // semantic compilation enforces the exact public condition count afterward.
    let condition_limit = timing_limits
        .max_conditions
        .saturating_mul(3)
        .saturating_add(structural_limit);
    let mut structure_count = 0usize;
    let mut condition_count = 0usize;
    let mut condition_depth = None;
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
                    let (count, limit) = if condition_depth.is_some() {
                        (&mut condition_count, condition_limit)
                    } else {
                        (&mut structure_count, structural_limit)
                    };
                    if *count >= limit {
                        return Err(mo_xml::XmlError::Limit("captured timing elements"));
                    }
                    *count += 1;
                    if condition_depth.is_none()
                        && (element.name.is(P, "stCondLst") || element.name.is(P, "endCondLst"))
                    {
                        condition_depth = Some(depth);
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
            XmlEvent::End { depth, .. } if capture => {
                if condition_depth == Some(depth) {
                    condition_depth = None;
                }
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
    let envelope = envelope::lift(&tree, list, root_id, timing_limits, check)?;
    if envelope.is_none()
        && list
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
    let indices = envelope.as_ref().map_or(&list.children, |e| &e.behaviors);
    for &index in indices {
        crate::cancelled(check)?;
        if nodes.len() >= timing_limits.max_nodes {
            return Err(PptxError::Limit("timing node count"));
        }
        let node = read_behavior(
            &tree,
            &tree[index],
            known_objects,
            root_id,
            &mut node_bindings,
            &mut object_bindings,
            check,
        )?;
        if envelope
            .as_ref()
            .is_some_and(|e| e.ids.contains(&node_bindings[&node.id]))
        {
            return Err(crate::value("timing/id", "duplicate timing identity"));
        }
        nodes.push(node);
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

fn read_behavior(
    tree: &[Node],
    anim: &Node,
    known_objects: &BTreeSet<u32>,
    root_id: u32,
    node_bindings: &mut BTreeMap<TimingNodeId, u32>,
    object_bindings: &mut BTreeMap<ObjectId, u32>,
    check: &dyn Fn() -> bool,
) -> Result<TimingNode, PptxError> {
    let (payload, behavior) = behavior::read(tree, anim)?;
    payload.common_attributes(behavior)?;
    let parts = ordered(
        tree,
        behavior,
        &[("cTn", true), ("tgtEl", true), ("attrNameLst", false)],
    )?;
    let common = parts[0].expect("required common behavior");
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
    let restart = read_restart(common)?;
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
        // Native fade filters have a finite duration and ordinary producer
        // output omits fill: the filter is removed, independent of Set helpers.
        None if matches!(payload, behavior::Payload::Fade(_)) => FillMode::Remove,
        _ => return Err(unsupported("fill mode")),
    };
    let lists = ordered(tree, common, &[("stCondLst", false), ("endCondLst", false)])?;
    let start = lists[0]
        .map(|list| read_start(tree, list, known_objects, object_bindings, check))
        .transpose()?
        .unwrap_or_else(|| TimeCondition::At { offset: ms(0) }.into());
    let end_conditions = lists[1]
        .map(|list| read_conditions(tree, list, known_objects, object_bindings, check))
        .transpose()?
        .unwrap_or_default();
    let target = object(
        tree,
        parts[1].expect("required target"),
        known_objects,
        object_bindings,
    )?;
    if let Some(properties) = parts[2] {
        payload.properties(tree, properties)?;
    } else if matches!(payload, behavior::Payload::Visibility(_)) {
        return Err(unsupported("set requires an explicit visibility property"));
    }
    Ok(TimingNode {
        id,
        restart,
        start,
        duration,
        repeat_milli,
        end_conditions,
        repeat_duration,
        fill,
        time_transform: time_transform::read(common)?,
        effect: payload.effect(target),
    })
}

fn read_restart(common: &Node) -> Result<mo_timeline::RestartMode, PptxError> {
    match common.element.attribute("restart") {
        Some("never") => Ok(mo_timeline::RestartMode::Never),
        // Native missing restart is always; the author API default is separate.
        None | Some("always") => Ok(mo_timeline::RestartMode::Always),
        Some("whenNotActive") => Ok(mo_timeline::RestartMode::WhenNotActive),
        _ => Err(unsupported(
            "restart mode requires an explicit supported value",
        )),
    }
}
fn read_start(
    tree: &[Node],
    start_list: &Node,
    known_objects: &BTreeSet<u32>,
    object_bindings: &mut BTreeMap<ObjectId, u32>,
    check: &dyn Fn() -> bool,
) -> Result<StartCondition, PptxError> {
    let mut conditions = read_conditions(tree, start_list, known_objects, object_bindings, check)?;
    Ok(if conditions.len() == 1 {
        conditions.pop().expect("one condition").into()
    } else {
        StartCondition::AnyOf { conditions }
    })
}
fn read_conditions(
    tree: &[Node],
    list: &Node,
    known_objects: &BTreeSet<u32>,
    object_bindings: &mut BTreeMap<ObjectId, u32>,
    check: &dyn Fn() -> bool,
) -> Result<Vec<mo_timeline::TimeCondition>, PptxError> {
    attrs(list, &[])?;
    if list.children.is_empty() || !list.text.trim().is_empty() {
        return Err(unsupported("empty or textual condition list"));
    }
    list.children
        .iter()
        .map(|&i| {
            crate::cancelled(check)?;
            if !tree[i].element.name.is(P, "cond") {
                return Err(unsupported("condition list child"));
            }
            condition::read(tree, &tree[i], known_objects, object_bindings)
        })
        .collect()
}
