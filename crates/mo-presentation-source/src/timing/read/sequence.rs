//! Sequence policy and condition lists retain the owning native container.
use super::*;
use mo_timeline::{NextAction, PreviousAction, SequenceNavigation};

pub(super) fn read<'a>(
    tree: &'a [Node],
    node: &Node,
    known: &BTreeSet<u32>,
    bindings: &mut BTreeMap<ObjectId, u32>,
    check: &dyn Fn() -> bool,
) -> Result<(&'a Node, Option<SequenceNavigation>), PptxError> {
    attrs(node, &["concurrent", "nextAc", "prevAc"])?;
    let parts = ordered(
        tree,
        node,
        &[
            ("cTn", true),
            ("prevCondLst", false),
            ("nextCondLst", false),
        ],
    )?;
    let common = parts[0].expect("required cTn");
    if node.element.attributes.is_empty() && parts[1..].iter().all(Option::is_none) {
        return Ok((common, None));
    }
    let concurrent = match node.element.attribute("concurrent") {
        None | Some("0" | "false") => false,
        Some("1" | "true") => true,
        _ => return Err(unsupported("sequence concurrent value")),
    };
    let next_action = match node.element.attribute("nextAc") {
        None | Some("none") => NextAction::None,
        Some("seek") => NextAction::Seek,
        _ => return Err(unsupported("sequence next action")),
    };
    let previous_action = match node.element.attribute("prevAc") {
        None | Some("none") => PreviousAction::None,
        Some("skipTimed") => PreviousAction::SkipTimed,
        _ => return Err(unsupported("sequence previous action")),
    };
    let previous_conditions = parts[1]
        .map(|n| read_conditions(tree, n, known, bindings, check))
        .transpose()?
        .unwrap_or_default();
    let next_conditions = parts[2]
        .map(|n| read_conditions(tree, n, known, bindings, check))
        .transpose()?
        .unwrap_or_default();
    Ok((
        common,
        Some(SequenceNavigation {
            concurrent,
            next_action,
            previous_action,
            next_conditions,
            previous_conditions,
        }),
    ))
}
