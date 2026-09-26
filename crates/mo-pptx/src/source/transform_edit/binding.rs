//! Bind attributes against physical XML, including ignored MCE material. Never
//! infer byte positions or accept a caller-supplied ordinal/name/attribute value.
use super::{SourceTransformEdit, SourceTransformValues};
use crate::{A, P, PptxError, source::SourceLimits};
use mo_xml::{AttributeEdit, Element, ExpandedName, XmlError, XmlEvent};
use std::collections::BTreeMap;

fn values(v: &SourceTransformValues, local: &str) -> Vec<(&'static str, Option<String>)> {
    match local {
        "xfrm" => vec![
            ("rot", v.rotation.map(|v| v.to_string())),
            (
                "flipH",
                v.flip_horizontal.map(|v| if v { "1" } else { "0" }.into()),
            ),
            (
                "flipV",
                v.flip_vertical.map(|v| if v { "1" } else { "0" }.into()),
            ),
        ],
        "off" | "chOff" => {
            let p = if local == "off" {
                v.origin
            } else {
                v.child_origin
            };
            vec![
                ("x", p.map(|v| v.x.get().to_string())),
                ("y", p.map(|v| v.y.get().to_string())),
            ]
        }
        "ext" | "chExt" => {
            let s = if local == "ext" { v.size } else { v.child_size };
            vec![
                ("cx", s.map(|v| v.width.get().to_string())),
                ("cy", s.map(|v| v.height.get().to_string())),
            ]
        }
        _ => unreachable!("validated transform child"),
    }
}
fn append(
    e: &Element,
    ordinal: usize,
    edit: &SourceTransformEdit,
    out: &mut Vec<AttributeEdit>,
) -> Result<(), XmlError> {
    let old = values(&edit.expected, &e.name.local);
    let new = values(&edit.replacement, &e.name.local);
    // MCE can remove unknown attributes from the semantic reader. Check the raw
    // node as well; do not silently mutate a selected projection of the source.
    if e.attributes
        .iter()
        .any(|a| !a.name.namespace.is_empty() || !old.iter().any(|(n, _)| *n == a.name.local))
    {
        return Err(XmlError::Compatibility(
            "transform attributes need coordinated editing".into(),
        ));
    }
    for ((name, before), (_, after)) in old.into_iter().zip(new) {
        if before == after {
            continue;
        }
        out.push(AttributeEdit {
            element_ordinal: ordinal,
            expected_element: e.name.clone(),
            attribute: ExpandedName {
                namespace: String::new(),
                local: name.into(),
            },
            expected_value: e.attribute(name).map(str::to_owned),
            value: after,
            insertion_name: None,
        });
    }
    Ok(())
}

pub(super) fn attributes(
    bytes: &[u8],
    edits: &BTreeMap<u32, &SourceTransformEdit>,
    limits: SourceLimits,
    check: &dyn Fn() -> bool,
) -> Result<Vec<AttributeEdit>, PptxError> {
    let mut depth = 0usize;
    let mut ordinal = 0usize;
    let mut active: Option<(usize, &SourceTransformEdit)> = None;
    let mut matched = 0usize;
    let mut result = Vec::new();
    mo_xml::scan_with_control(bytes, limits.package.xml, check, |event| {
        match event {
            XmlEvent::Start { element, .. } => {
                if let Some(edit) = u32::try_from(ordinal).ok().and_then(|n| edits.get(&n)) {
                    if active.is_some()
                        || !(element.name.is(A, "xfrm") || element.name.is(P, "xfrm"))
                    {
                        return Err(XmlError::EditConflict(
                            "bound transform identity changed".into(),
                        ));
                    }
                    active = Some((depth, edit));
                    matched += 1;
                }
                if let Some((root, edit)) = active {
                    if depth != root
                        && (depth != root + 1
                            || element.name.namespace != A
                            || !["off", "ext", "chOff", "chExt"]
                                .contains(&element.name.local.as_str()))
                    {
                        return Err(XmlError::Compatibility(
                            "physical transform children need coordinated editing".into(),
                        ));
                    }
                    append(element, ordinal, edit, &mut result)?;
                }
                depth += 1;
                ordinal += 1;
            }
            XmlEvent::End { .. } => {
                depth -= 1;
                if active.is_some_and(|(root, _)| root == depth) {
                    active = None;
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    if matched != edits.len() {
        return Err(super::conflict("bound transform source missing"));
    }
    Ok(result)
}
