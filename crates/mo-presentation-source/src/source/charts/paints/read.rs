//! Project chart spPr nodes through the shared DrawingML fill/line grammar.
use super::*;
use crate::source::{SourceColorMap, drawingml::enumeration, fill, line, paint};
use mo_xml::{XmlError, XmlEvent, mce};

pub(super) struct Parsed {
    pub declarations: Vec<ChartPaintDeclaration>,
    pub color_map: Option<(u32, SourceColorMap)>,
}
pub(super) fn read(
    bytes: &[u8],
    limits: SourceLimits,
    max_declarations: usize,
    check: &dyn Fn() -> bool,
) -> Result<Parsed, PptxError> {
    let mut result = Parsed {
        declarations: vec![],
        color_map: None,
    };
    let mut depth = 0usize;
    let mut active: Option<(usize, ChartPaintDeclaration)> = None;
    let mut fill: Option<paint::Reader> = None;
    let mut line: Option<line::Reader> = None;
    let mut fill_budget = paint::Budget::default();
    let mut line_budget = line::Budget::default();
    let mut opaque = None;
    let mut outside_extension = 0usize;
    let mut map_depth = None;
    mce::scan(
        bytes,
        limits.package.xml,
        &super::super::profile(),
        check,
        |event| {
            let mce::Event::Content {
                event,
                source_ordinal,
                extension_content,
                ..
            } = event
            else {
                return Ok(());
            };
            match event {
                XmlEvent::Start { element, .. } => {
                    let ordinal = u32::try_from(source_ordinal.expect("physical chart element"))
                        .map_err(|_| XmlError::Limit("chart paint ordinal"))?;
                    if let Some(reader) = &mut fill {
                        reader.start(
                            element,
                            depth,
                            ordinal,
                            extension_content,
                            &mut fill_budget,
                            limits,
                        )?;
                    } else if let Some(reader) = &mut line {
                        reader.start(
                            element,
                            depth,
                            ordinal,
                            extension_content,
                            &mut line_budget,
                            &mut fill_budget,
                            limits,
                        )?;
                    } else if let Some((parent, decl)) = &mut active {
                        if opaque.is_none() {
                            if depth != *parent + 1 {
                                return Err(XmlError::Malformed(
                                    "unexpected chart paint nesting".into(),
                                ));
                            }
                            if !extension_content
                                && (fill::is_fill(&element.name)
                                    || paint::is_effect_properties(&element.name))
                            {
                                if (fill::is_fill(&element.name) && decl.fill.is_some())
                                    || (paint::is_effect_properties(&element.name)
                                        && decl.effects.is_some())
                                {
                                    return Err(XmlError::Malformed("duplicate chart fill".into()));
                                }
                                fill = Some(paint::Reader::new(
                                    element,
                                    depth,
                                    ordinal,
                                    &mut fill_budget,
                                    limits,
                                )?);
                            } else if !extension_content && element.name.is(A, "ln") {
                                if decl.line.is_some() {
                                    return Err(XmlError::Malformed("duplicate chart line".into()));
                                }
                                line = Some(line::Reader::new(
                                    element,
                                    depth,
                                    ordinal,
                                    &mut line_budget,
                                    limits,
                                )?);
                            } else {
                                decl.retained_ordinals.push(ordinal);
                                opaque = Some(depth);
                            }
                        }
                    } else if map_depth.is_some() {
                        return Err(XmlError::Compatibility(
                            "chart color map contains unresolved content".into(),
                        ));
                    } else if outside_extension > 0 || extension_content {
                        outside_extension += 1;
                    } else if depth == 1 && element.name.is(C, "clrMapOvr") {
                        if result.color_map.is_some() {
                            return Err(XmlError::Malformed("duplicate chart color map".into()));
                        }
                        if element.attributes.iter().any(|a| {
                            !a.name.namespace.is_empty()
                                || ![
                                    "bg1", "tx1", "bg2", "tx2", "accent1", "accent2", "accent3",
                                    "accent4", "accent5", "accent6", "hlink", "folHlink",
                                ]
                                .contains(&a.name.local.as_str())
                        }) {
                            return Err(XmlError::Compatibility(
                                "unknown chart color-map attribute".into(),
                            ));
                        }
                        result.color_map = Some((ordinal, SourceColorMap::read(element)?));
                        map_depth = Some(depth);
                    } else if element.name.is(C, "spPr") {
                        if result.declarations.len() >= max_declarations {
                            return Err(XmlError::Limit("chart paint declarations"));
                        }
                        let unknown = element
                            .attributes
                            .iter()
                            .any(|a| !a.name.namespace.is_empty() || a.name.local != "bwMode");
                        active = Some((
                            depth,
                            ChartPaintDeclaration {
                                source_ordinal: ordinal,
                                black_white_mode: element
                                    .attribute("bwMode")
                                    .map(enumeration)
                                    .transpose()?,
                                fill: None,
                                line: None,
                                effects: None,
                                effect_nodes: BTreeMap::new(),
                                retained_ordinals: if unknown { vec![ordinal] } else { vec![] },
                                colors: vec![],
                            },
                        ));
                    }
                    depth += 1;
                }
                XmlEvent::End { .. } => {
                    depth = depth
                        .checked_sub(1)
                        .ok_or_else(|| XmlError::Malformed("chart paint depth".into()))?;
                    if let Some(reader) = &mut fill {
                        if reader.depth == depth {
                            let parsed = fill.take().expect("active fill").finish()?;
                            let decl = &mut active.as_mut().expect("fill owner").1;
                            match parsed.publish(&mut decl.effect_nodes)? {
                                paint::Declaration::Fill(value) => decl.fill = Some(value),
                                paint::Declaration::Effects(value) => decl.effects = Some(value),
                                _ => {
                                    return Err(XmlError::Malformed(
                                        "chart paint root mismatch".into(),
                                    ));
                                }
                            }
                        } else {
                            reader.end(depth)?;
                        }
                    } else if let Some(reader) = &mut line {
                        if reader.depth == depth {
                            let line::Declaration::Line(value) =
                                line.take().expect("active line").finish()
                            else {
                                return Err(XmlError::Malformed("chart line root mismatch".into()));
                            };
                            active.as_mut().expect("line owner").1.line = Some(value);
                        } else {
                            reader.end(depth)?;
                        }
                    } else if let Some((parent, _)) = &active {
                        if opaque == Some(depth) {
                            opaque = None;
                        }
                        if *parent == depth {
                            result
                                .declarations
                                .push(active.take().expect("chart style").1);
                        }
                    } else {
                        outside_extension = outside_extension.saturating_sub(1);
                    }
                    if map_depth == Some(depth) {
                        map_depth = None;
                    }
                }
                XmlEvent::Text { text, .. } => {
                    if let Some(reader) = &fill {
                        reader.text(text)?
                    } else if let Some(reader) = &line {
                        reader.text(text)?
                    } else if ((active.is_some() && opaque.is_none()) || map_depth.is_some())
                        && !text.trim().is_empty()
                    {
                        return Err(XmlError::Malformed("chart paint character data".into()));
                    }
                }
                _ => {}
            }
            Ok(())
        },
    )?;
    if depth != 0 || active.is_some() || fill.is_some() || line.is_some() {
        return Err(invalid("unclosed chart paint"));
    }
    Ok(result)
}
