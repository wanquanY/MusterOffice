use super::{
    frame::{Child, Node},
    *,
};
use crate::{
    A,
    source::{
        effects::{Children, PendingEffect, Slots},
        malformed,
    },
};
use mo_xml::{Element, ExpandedName, XmlError};

pub(super) fn new(e: &Element) -> Result<(Node, &'static [&'static str]), XmlError> {
    if e.name.namespace != A {
        return Err(malformed("unknown effect namespace"));
    }
    let name = if e.name.local == "effectDag" {
        "cont"
    } else {
        e.name.local.as_str()
    };
    let (pending, attrs) = PendingEffect::read(name, e)?;
    Ok((
        Node::EffectDraft {
            pending,
            slots: Slots::default(),
        },
        attrs,
    ))
}
pub(super) fn child(
    value: &Node,
    name: &ExpandedName,
) -> Result<Option<(u8, bool, Child)>, XmlError> {
    let local = name.local.as_str();
    let color = name.namespace == A
        && [
            "srgbClr",
            "scrgbClr",
            "hslClr",
            "sysClr",
            "schemeClr",
            "prstClr",
        ]
        .contains(&local);
    let choice = match value {
        Node::EffectList(_) => {
            if name.namespace != A {
                return Err(malformed("unknown effect list namespace"));
            }
            let rank = [
                "blur",
                "fillOverlay",
                "glow",
                "innerShdw",
                "outerShdw",
                "prstShdw",
                "reflection",
                "softEdge",
            ]
            .iter()
            .position(|v| *v == local)
            .ok_or_else(|| malformed("invalid effect list child"))?;
            (rank as u8 + 1, false, Child::Effect)
        }
        Node::EffectStyle { .. } => {
            if name.namespace != A {
                return Err(malformed("unknown effect style namespace"));
            }
            match local {
                "effectLst" => (1, false, Child::Ordinary),
                "effectDag" => (1, false, Child::Effect),
                "scene3d" => (2, false, Child::Opaque),
                "sp3d" => (3, false, Child::Opaque),
                _ => return Err(malformed("invalid effect style child")),
            }
        }
        Node::EffectDraft { pending, slots } => match pending.children() {
            Children::OptionalColor | Children::Color if color => (1, false, Child::Ordinary),
            Children::TwoColors if color && slots.colors.len() < 2 => (1, true, Child::Ordinary),
            Children::ColorChange if name.is(A, "clrFrom") => (1, false, Child::Ordinary),
            Children::ColorChange if name.is(A, "clrTo") => (2, false, Child::Ordinary),
            Children::Fill if super::super::fill::is_fill(name) => (1, false, Child::Ordinary),
            Children::Container if name.is(A, "cont") => (1, false, Child::Effect),
            Children::Effects if name.namespace == A => (1, true, Child::Effect),
            _ => return Err(malformed("unexpected effect child or namespace")),
        },
        _ => return Ok(None),
    };
    Ok(Some(choice))
}
pub(super) fn finish(
    value: Node,
    ordinal: u32,
    name: &str,
    retained: Vec<u32>,
    nodes: &mut BTreeMap<u32, SourceEffectNode>,
) -> Result<Node, XmlError> {
    match value {
        Node::EffectDraft { pending, slots } => {
            let node = SourceEffectNode {
                source_ordinal: ordinal,
                definition: pending.finish(slots)?,
                retained_ordinals: retained,
            };
            if nodes.insert(ordinal, node).is_some() {
                return Err(malformed("duplicate effect source binding"));
            }
            Ok(if name == "effectDag" {
                Node::Effects(SourceEffectProperties {
                    source_ordinal: ordinal,
                    definition: SourceEffectPropertiesDefinition::Dag { root: ordinal },
                    retained_ordinals: Vec::new(),
                })
            } else {
                Node::EffectLink(ordinal)
            })
        }
        Node::EffectList(ids) => Ok(Node::Effects(SourceEffectProperties {
            source_ordinal: ordinal,
            definition: SourceEffectPropertiesDefinition::List { nodes: ids },
            retained_ordinals: retained,
        })),
        Node::EffectStyle { effects } => Ok(Node::FinishedEffectStyle(SourceEffectStyle {
            source_ordinal: ordinal,
            effects: effects.ok_or_else(|| malformed("effect style missing properties"))?,
            retained_ordinals: retained,
        })),
        _ => Err(malformed("invalid effect completion")),
    }
}
