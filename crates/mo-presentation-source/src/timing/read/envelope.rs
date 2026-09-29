//! Lift only a timing-neutral presentation envelope. No navigation, implicit
//! duration, finite ancestor, repeat, event gate or multi-behavior effect is
//! discarded. The source package retains its original editorial declarations.
use super::*;
use crate::timing::TransformPreset;

pub(super) struct Envelope {
    pub behaviors: Vec<usize>,
    pub ids: BTreeSet<u32>,
}

pub(super) fn lift(
    tree: &[Node],
    list: &Node,
    root_id: u32,
    limits: TimelineLimits,
    check: &dyn Fn() -> bool,
) -> Result<Option<Envelope>, PptxError> {
    match neutral(tree, list, root_id, limits, check) {
        // A non-neutral presentation envelope belongs to the full tree reader.
        // That reader must validate every declaration; nothing is discarded.
        Err(PptxError::Unsupported(_)) => Ok(None),
        result => result,
    }
}

fn neutral(
    tree: &[Node],
    list: &Node,
    root_id: u32,
    limits: TimelineLimits,
    check: &dyn Fn() -> bool,
) -> Result<Option<Envelope>, PptxError> {
    // A general typed tree without mainSeq remains owned by the tree reader.
    if !list.children.iter().any(|&i| {
        let node = &tree[i];
        node.element.name.is(P, "seq")
            && node.children.iter().any(|&j| {
                tree[j].element.name.is(P, "cTn")
                    && tree[j].element.attribute("nodeType") == Some("mainSeq")
            })
    }) {
        return Ok(None);
    }
    let seq = single(tree, list, "seq")?;
    attrs(seq, &["concurrent", "nextAc", "prevAc"])?;
    if seq
        .element
        .attribute("concurrent")
        .is_some_and(|v| !matches!(v, "0" | "false"))
        || seq.element.attribute("nextAc").is_some_and(|v| v != "none")
        || seq.element.attribute("prevAc").is_some_and(|v| v != "none")
    {
        return Err(unsupported(
            "sequence navigation/concurrency requires its event profile",
        ));
    }
    let mut envelope = Envelope {
        behaviors: vec![],
        ids: BTreeSet::from([root_id]),
    };
    let main = single(tree, seq, "cTn")?;
    let outer = common(tree, main, Some("mainSeq"), None, &mut envelope.ids)?;
    let par = single(tree, outer, "par")?;
    attrs(par, &[])?;
    let inner = common(
        tree,
        single(tree, par, "cTn")?,
        None,
        None,
        &mut envelope.ids,
    )?;
    let par = single(tree, inner, "par")?;
    attrs(par, &[])?;
    let effects = common(
        tree,
        single(tree, par, "cTn")?,
        None,
        None,
        &mut envelope.ids,
    )?;
    for &index in &effects.children {
        crate::cancelled(check)?;
        if envelope.behaviors.len() >= limits.max_nodes {
            return Err(PptxError::Limit("timing node count"));
        }
        let par = &tree[index];
        if !par.element.name.is(P, "par") {
            return Err(unsupported("effect container must be parallel"));
        }
        attrs(par, &[])?;
        let effect = single(tree, par, "cTn")?;
        // Locate the single payload before validating its matching preset.
        let parts = children(tree, effect, &["stCondLst", "childTnLst"])?;
        let list = parts[1];
        if list.children.len() != 1 {
            return Err(unsupported("effect requires one transform behavior"));
        }
        let behavior = &tree[list.children[0]];
        let preset = if behavior.element.name.is(P, "set") {
            None
        } else {
            Some(
                TransformPreset::for_behavior(&behavior.element.name.local)
                    .filter(|_| behavior.element.name.namespace == P)
                    .ok_or_else(|| unsupported("effect transform behavior"))?,
            )
        };
        common(
            tree,
            effect,
            preset.map(|_| "withEffect"),
            preset,
            &mut envelope.ids,
        )?;
        envelope.behaviors.push(list.children[0]);
    }
    // Referenced containers require real identities in the retained hierarchy.
    // Never rebind an effect/container event to its leaf behavior.
    for node in tree {
        crate::cancelled(check)?;
        if node.element.name.is(P, "tn") && envelope.ids.contains(&integer(node, "val")?) {
            return Ok(None);
        }
    }
    Ok(Some(envelope))
}

fn common<'a>(
    tree: &'a [Node],
    node: &Node,
    node_type: Option<&str>,
    preset: Option<TransformPreset>,
    ids: &mut BTreeSet<u32>,
) -> Result<&'a Node, PptxError> {
    attrs(
        node,
        &[
            "id",
            "dur",
            "restart",
            "fill",
            "nodeType",
            "presetID",
            "presetClass",
            "presetSubtype",
        ],
    )?;
    value(node, "dur", "indefinite")?;
    value(node, "restart", "never")?;
    value(node, "fill", "hold")?;
    if node.element.attribute("nodeType") != node_type {
        return Err(unsupported("presentation envelope node type"));
    }
    if let Some(preset) = preset {
        if integer(node, "presetID")? != preset.id() {
            return Err(unsupported(
                "effect preset does not match transform behavior",
            ));
        }
        value(node, "presetClass", preset.class())?;
        value(node, "presetSubtype", "0")?;
    } else if ["presetID", "presetClass", "presetSubtype"]
        .iter()
        .any(|key| node.element.attribute(key).is_some())
    {
        return Err(unsupported("preset on grouping container"));
    }
    if !ids.insert(integer(node, "id")?) {
        return Err(crate::value("timing/id", "duplicate timing identity"));
    }
    let parts = children(tree, node, &["stCondLst", "childTnLst"])?;
    attrs(parts[0], &[])?;
    let start = single(tree, parts[0], "cond")?;
    attrs(start, &["delay"])?;
    empty(tree, start)?;
    if condition::delay(start)? != ms(0) {
        return Err(unsupported("presentation envelope must start at zero"));
    }
    let list = parts[1];
    attrs(list, &[])?;
    if list.children.is_empty() || !list.text.trim().is_empty() {
        return Err(unsupported("empty or textual presentation envelope"));
    }
    Ok(list)
}
