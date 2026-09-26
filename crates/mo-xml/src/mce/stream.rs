use super::{
    Branch, Event, NAMESPACE, Profile, Selection, Summary,
    rules::{self, Rules},
};
use crate::{Element, XmlError, XmlEvent, XmlLimits, names};
use std::rc::Rc;

struct Alternate {
    selection: usize,
    choices: usize,
    fallback: bool,
    selected: bool,
}
enum Kind {
    Normal,
    Extension,
    Unwrapped,
    Skipped,
    Alternate(Alternate),
    Branch(bool),
}
struct Frame {
    kind: Kind,
    rules: Rc<Rules>,
    forwarded: bool,
    branch_added: bool,
}
impl Frame {
    fn suppressed(&self) -> bool {
        matches!(self.kind, Kind::Skipped | Kind::Branch(false))
    }
}

pub fn scan(
    input: &[u8],
    limits: XmlLimits,
    profile: &Profile,
    cancelled: &dyn Fn() -> bool,
    mut visitor: impl FnMut(Event<'_>) -> Result<(), XmlError>,
) -> Result<Summary, XmlError> {
    if profile
        .extension_elements
        .iter()
        .any(|n| n.namespace == NAMESPACE)
    {
        return Err(rules::invalid(
            "MCE elements cannot be application extensions",
        ));
    }
    let initial = Rc::new(Rules::default());
    let mut frames: Vec<Frame> = Vec::new();
    let mut branches = Vec::new();
    let mut selections: Vec<Selection> = Vec::new();
    let mut ordinal = 0;
    let mut output_depth = 0;
    let mut ignored_elements = 0;
    let mut ignored_attributes = 0;
    let mut unwrapped_elements = 0;
    let xml = crate::scan_with_control(input, limits, cancelled, |event| {
        match event {
            XmlEvent::Start {
                element,
                namespaces,
                span,
                ..
            } => {
                let source_ordinal = ordinal;
                ordinal += 1;
                visitor(Event::SourceElement {
                    element,
                    source_ordinal,
                })?;
                let parent_rules = frames
                    .last()
                    .map(|f| f.rules.clone())
                    .unwrap_or_else(|| initial.clone());
                let suppressed = frames.last().is_some_and(Frame::suppressed);
                let opaque = frames
                    .last()
                    .is_some_and(|f| matches!(f.kind, Kind::Extension))
                    || profile.extension_elements.contains(&element.name);
                if opaque
                    && frames
                        .last()
                        .is_some_and(|f| matches!(f.kind, Kind::Alternate(_)))
                {
                    return Err(XmlError::Compatibility(
                        "application extension directly inside AlternateContent is not discarded"
                            .into(),
                    ));
                }
                let mut branch_added = false;
                let mut filtered: Option<Element> = None;
                let (kind, scope, forwarded) = if suppressed {
                    (Kind::Skipped, parent_rules, false)
                } else if opaque {
                    // Part 3 §8 and §9.4: even MCE attributes on this element and
                    // its descendants survive unchanged in the logical projection.
                    (Kind::Extension, parent_rules, true)
                } else {
                    let attrs = rules::attributes(element, namespaces, &parent_rules)?;
                    let ignored = attrs.rules.ignorable(&element.name.namespace)
                        && !profile
                            .understood_namespaces
                            .contains(&element.name.namespace);
                    if let Some(Frame {
                        kind: Kind::Alternate(alt),
                        ..
                    }) = frames.last_mut()
                    {
                        if element.name.is(NAMESPACE, "Choice")
                            || element.name.is(NAMESPACE, "Fallback")
                        {
                            let fallback = element.name.local == "Fallback";
                            if alt.fallback || (fallback && alt.choices == 0) {
                                return Err(rules::invalid("Choice/Fallback order or cardinality"));
                            }
                            if fallback {
                                alt.fallback = true;
                            } else {
                                alt.choices += 1;
                            }
                            let selected = !alt.selected
                                && (fallback
                                    || attrs
                                        .requires
                                        .iter()
                                        .all(|ns| profile.understood_namespaces.contains(ns)));
                            if selected {
                                rules::require_understood(&attrs.must_understand, profile)?;
                                alt.selected = true;
                                branches.push(selections[alt.selection].source_ordinal);
                                branch_added = true;
                            }
                            selections[alt.selection].branches.push(Branch {
                                source_ordinal,
                                requires: attrs.requires,
                                fallback,
                                selected,
                            });
                            (Kind::Branch(selected), attrs.rules, false)
                        } else if ignored && !attrs.rules.process(&element.name) {
                            ignored_elements += 1;
                            (Kind::Skipped, attrs.rules, false)
                        } else {
                            return Err(XmlError::Compatibility(
                                "non-discarded non-branch child of AlternateContent".into(),
                            ));
                        }
                    } else if ignored {
                        if attrs.rules.process(&element.name) {
                            if element.attributes.iter().any(|a| {
                                a.name.namespace == names::XML_NS
                                    && ["base", "lang", "space"].contains(&a.name.local.as_str())
                            }) {
                                return Err(rules::invalid(
                                    "unwrapped element cannot carry xml:base/lang/space",
                                ));
                            }
                            rules::require_understood(&attrs.must_understand, profile)?;
                            unwrapped_elements += 1;
                            (Kind::Unwrapped, attrs.rules, false)
                        } else {
                            ignored_elements += 1;
                            (Kind::Skipped, attrs.rules, false)
                        }
                    } else if element.name.namespace == NAMESPACE {
                        if !element.name.is(NAMESPACE, "AlternateContent") {
                            return Err(rules::invalid(
                                "Choice/Fallback outside AlternateContent or unknown MCE element",
                            ));
                        }
                        rules::require_understood(&attrs.must_understand, profile)?;
                        let selection = selections.len();
                        selections.push(Selection {
                            source_ordinal,
                            branches: Vec::new(),
                        });
                        (
                            Kind::Alternate(Alternate {
                                selection,
                                choices: 0,
                                fallback: false,
                                selected: false,
                            }),
                            attrs.rules,
                            false,
                        )
                    } else {
                        rules::require_understood(&attrs.must_understand, profile)?;
                        let discard = |a: &crate::Attribute| {
                            a.name.namespace == NAMESPACE
                                || (attrs.rules.ignorable(&a.name.namespace)
                                    && !profile.understood_namespaces.contains(&a.name.namespace))
                        };
                        if element.attributes.iter().any(discard) {
                            let mut copy = element.clone();
                            copy.attributes.retain(|a| {
                                if discard(a) {
                                    if a.name.namespace != NAMESPACE {
                                        ignored_attributes += 1;
                                    }
                                    false
                                } else {
                                    true
                                }
                            });
                            filtered = Some(copy);
                        }
                        (Kind::Normal, attrs.rules, true)
                    }
                };
                if forwarded {
                    visitor(Event::Content {
                        event: XmlEvent::Start {
                            element: filtered.as_ref().unwrap_or(element),
                            namespaces,
                            depth: output_depth,
                            span,
                        },
                        source_ordinal: Some(source_ordinal),
                        alternate_ancestors: &branches,
                        extension_content: matches!(kind, Kind::Extension),
                    })?;
                    output_depth += 1;
                }
                frames.push(Frame {
                    kind,
                    rules: scope,
                    forwarded,
                    branch_added,
                });
            }
            XmlEvent::End { name, span, .. } => {
                let frame = frames.pop().expect("XML scanner balances frames");
                if let Kind::Alternate(alt) = &frame.kind
                    && alt.choices == 0
                {
                    return Err(rules::invalid(
                        "AlternateContent requires at least one Choice",
                    ));
                }
                if frame.forwarded {
                    output_depth -= 1;
                    visitor(Event::Content {
                        event: XmlEvent::End {
                            name,
                            depth: output_depth,
                            span,
                        },
                        source_ordinal: None,
                        alternate_ancestors: &branches,
                        extension_content: matches!(frame.kind, Kind::Extension),
                    })?;
                }
                if frame.branch_added {
                    branches.pop();
                }
            }
            XmlEvent::Text { text, span, .. } => {
                if frames.last().is_some_and(Frame::suppressed) {
                    return Ok(());
                }
                if frames
                    .last()
                    .is_some_and(|f| matches!(f.kind, Kind::Alternate(_)))
                {
                    if !text.chars().all(|c| matches!(c, ' ' | '\r' | '\n' | '\t')) {
                        return Err(rules::invalid("text directly inside AlternateContent"));
                    }
                } else {
                    visitor(Event::Content {
                        event: XmlEvent::Text {
                            text,
                            depth: output_depth,
                            span,
                        },
                        source_ordinal: None,
                        alternate_ancestors: &branches,
                        extension_content: frames
                            .last()
                            .is_some_and(|f| matches!(f.kind, Kind::Extension)),
                    })?;
                }
            }
            other => {
                if !frames
                    .last()
                    .is_some_and(|f| f.suppressed() || matches!(f.kind, Kind::Alternate(_)))
                {
                    visitor(Event::Content {
                        event: other,
                        source_ordinal: None,
                        alternate_ancestors: &branches,
                        extension_content: frames
                            .last()
                            .is_some_and(|f| matches!(f.kind, Kind::Extension)),
                    })?;
                }
            }
        }
        Ok(())
    })?;
    Ok(Summary {
        xml,
        selections,
        ignored_elements,
        ignored_attributes,
        unwrapped_elements,
    })
}
