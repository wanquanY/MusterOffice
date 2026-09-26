use super::*;
fn unknown_font_attributes(element: &mo_xml::Element, allowed: &[&str]) -> bool {
    element.attributes.iter().any(|a| {
        a.name.namespace != "http://schemas.openxmlformats.org/markup-compatibility/2006"
            && (!a.name.namespace.is_empty() || !allowed.contains(&a.name.local.as_str()))
    })
}
use crate::A;
use crate::source::drawingml::{color, enumeration, required, text_font, transform};
use mo_xml::{ExpandedName, XmlError, XmlEvent, mce};
use std::collections::BTreeSet;
#[derive(Clone, Copy)]
enum PaintStyle {
    Fill,
    Background,
    Effect,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn parse(
    bytes: &[u8],
    kind: SourceThemeKind,
    sha256: Digest,
    limits: SourceLimits,
    budget: &mut Budget,
    line_budget: &mut line::Budget,
    paint_budget: &mut paint::Budget,
    text_budget: &mut text::Budget,
    check: &dyn Fn() -> bool,
) -> Result<SourceThemePart, PptxError> {
    let mut result = SourceThemePart {
        text_defaults: None,
        sha256,
        kind,
        name: None,
        color_scheme: None,
        font_scheme: None,
        format_scheme: None,
        compatibility: SourceCompatibility::default(),
        notices: Vec::new(),
        effect_nodes: BTreeMap::new(),
    };
    let mut stack: Vec<ExpandedName> = Vec::new();
    let mut root_seen = false;
    let mut base_seen = false;
    let mut sections = BTreeSet::new();
    let mut slots = BTreeSet::new();
    let mut font_groups = BTreeSet::new();
    let mut format_lists = BTreeSet::new();
    let mut slot: Option<(usize, ColorSlot)> = None;
    let mut color_depth = None;
    let mut retained_depth = None;
    let mut line_capture: Option<(usize, line::Reader)> = None;
    let mut fill_capture: Option<(PaintStyle, usize, paint::Reader)> = None;
    let mut defaults_capture: Option<defaults::Reader> = None;
    let mut notices = BTreeSet::new();
    let section_depth = if kind == SourceThemeKind::Theme { 2 } else { 1 };
    let summary = mce::scan(
        bytes,
        limits.package.xml,
        &compatibility::profile(),
        check,
        |event| {
            let (event, ordinal, extension) = match event {
                mce::Event::SourceElement { element, .. } => {
                    budget.elements = budget
                        .elements
                        .checked_add(1)
                        .ok_or(XmlError::Limit("theme elements"))?;
                    for attribute in &element.attributes {
                        budget.attribute_bytes = budget
                            .attribute_bytes
                            .checked_add(attribute.value.len())
                            .ok_or(XmlError::Limit("theme attribute bytes"))?;
                    }
                    if budget.elements > limits.max_theme_elements {
                        return Err(XmlError::Limit("theme elements"));
                    }
                    if budget.attribute_bytes > limits.max_theme_attribute_bytes {
                        return Err(XmlError::Limit("theme attribute bytes"));
                    }
                    return Ok(());
                }
                mce::Event::Content {
                    event,
                    source_ordinal,
                    extension_content,
                    ..
                } => (event, source_ordinal, extension_content),
            };
            match event {
                XmlEvent::Start { element, .. } => {
                    let depth = stack.len();
                    if let Some(reader) = &mut defaults_capture {
                        reader.start(
                            element,
                            depth,
                            ordinal
                                .expect("start ordinal")
                                .try_into()
                                .map_err(|_| XmlError::Limit("theme default ordinal"))?,
                            extension,
                            text_budget,
                            line_budget,
                            paint_budget,
                            limits,
                        )?;
                        stack.push(element.name.clone());
                        return Ok(());
                    }
                    if let Some((_, reader)) = &mut line_capture {
                        reader.start(
                            element,
                            depth,
                            ordinal
                                .expect("start ordinal")
                                .try_into()
                                .map_err(|_| XmlError::Limit("line source ordinal"))?,
                            extension,
                            line_budget,
                            paint_budget,
                            limits,
                        )?;
                    }
                    if let Some((_, _, reader)) = &mut fill_capture {
                        reader.start(
                            element,
                            depth,
                            ordinal
                                .expect("start ordinal")
                                .try_into()
                                .map_err(|_| XmlError::Limit("fill source ordinal"))?,
                            extension,
                            paint_budget,
                            limits,
                        )?;
                    }
                    if extension || retained_depth.is_some() {
                        if extension
                            && stack
                                .get(section_depth)
                                .is_some_and(|n| n.is(A, "fontScheme"))
                        {
                            result
                                .font_scheme
                                .as_mut()
                                .expect("font scheme")
                                .retained_ordinals
                                .push(
                                    ordinal
                                        .expect("start ordinal")
                                        .try_into()
                                        .map_err(|_| XmlError::Limit("font source ordinal"))?,
                                );
                        }
                        stack.push(element.name.clone());
                        return Ok(());
                    }
                    let ordinal: u32 = ordinal
                        .expect("start ordinal")
                        .try_into()
                        .map_err(|_| XmlError::Limit("theme source ordinal"))?;
                    let local = element.name.local.as_str();
                    if depth == 0 {
                        let root = if kind == SourceThemeKind::Theme {
                            "theme"
                        } else {
                            "themeOverride"
                        };
                        if root_seen || !element.name.is(A, root) {
                            return Err(malformed("theme root mismatch or multiple roots"));
                        }
                        root_seen = true;
                        result.name = element.attribute("name").map(str::to_owned);
                    } else if depth == 1
                        && kind == SourceThemeKind::Theme
                        && element.name.is(A, "themeElements")
                    {
                        if base_seen {
                            return Err(malformed("duplicate themeElements"));
                        }
                        base_seen = true;
                    } else if depth == section_depth
                        && (kind == SourceThemeKind::Override
                            || stack.last().is_some_and(|n| n.is(A, "themeElements")))
                    {
                        if element.name.namespace != A {
                            return Err(XmlError::Compatibility(
                                "unknown theme section namespace".into(),
                            ));
                        }
                        if !sections.insert(local.to_owned()) {
                            return Err(malformed("duplicate theme scheme"));
                        }
                        match local {
                            "clrScheme" => {
                                result.color_scheme = Some(SourceColorScheme {
                                    source_ordinal: ordinal,
                                    name: required(element, "name")?.into(),
                                    colors: BTreeMap::new(),
                                })
                            }
                            "fontScheme" => {
                                result.font_scheme = Some(SourceFontScheme {
                                    source_ordinal: ordinal,
                                    retained_ordinals: if unknown_font_attributes(
                                        element,
                                        &["name"],
                                    ) {
                                        vec![ordinal]
                                    } else {
                                        vec![]
                                    },
                                    name: required(element, "name")?.into(),
                                    major: SourceFontCollection::default(),
                                    minor: SourceFontCollection::default(),
                                })
                            }
                            "fmtScheme" => {
                                result.format_scheme = Some(SourceFormatScheme {
                                    source_ordinal: ordinal,
                                    name: element.attribute("name").map(str::to_owned),
                                    fills: Vec::new(),
                                    lines: Vec::new(),
                                    effects: Vec::new(),
                                    background_fills: Vec::new(),
                                })
                            }
                            _ => {
                                return Err(XmlError::Compatibility("unknown theme scheme".into()));
                            }
                        }
                    } else if depth == 1
                        && kind == SourceThemeKind::Theme
                        && element.name.is(A, "objectDefaults")
                    {
                        if result.text_defaults.is_some() {
                            return Err(malformed("duplicate theme object defaults"));
                        }
                        defaults_capture = Some(defaults::Reader::new(element, depth, ordinal));
                    } else if depth == 1 && kind == SourceThemeKind::Theme {
                        // Root declarations outside themeElements are retained
                        // as a whole. Nested familiar names must not be parsed
                        // as active schemes or gain access to section state.
                        retained_depth = Some(depth);
                        notices.insert(format!(
                            "theme declaration retained but not resolved: {}",
                            element.name.local
                        ));
                    } else if stack
                        .get(section_depth)
                        .is_some_and(|n| n.is(A, "clrScheme"))
                    {
                        if element.name.namespace != A {
                            return Err(XmlError::Compatibility(
                                "unknown theme color namespace".into(),
                            ));
                        }
                        if depth == section_depth + 1 {
                            let which: ColorSlot = enumeration(local)?;
                            if !slots.insert(which) {
                                return Err(malformed("duplicate theme color slot"));
                            }
                            slot = Some((depth, which));
                        } else if let Some((slot_depth, which)) = slot {
                            let colors =
                                &mut result.color_scheme.as_mut().expect("color section").colors;
                            if depth == slot_depth + 1 {
                                if colors.contains_key(&which) {
                                    return Err(malformed("multiple colors in theme slot"));
                                }
                                colors.insert(which, color(element, ordinal)?);
                                color_depth = Some(depth);
                            } else if color_depth == Some(depth - 1) {
                                colors
                                    .get_mut(&which)
                                    .expect("active color")
                                    .transforms
                                    .push(transform(element)?);
                            } else {
                                return Err(malformed("nested element inside color transform"));
                            }
                        }
                    } else if stack
                        .get(section_depth)
                        .is_some_and(|n| n.is(A, "fontScheme"))
                    {
                        if element.name.namespace != A {
                            return Err(XmlError::Compatibility(
                                "unknown theme font namespace".into(),
                            ));
                        }
                        let allowed: &[&str] = match local {
                            "majorFont" | "minorFont" => &[],
                            "latin" | "ea" | "cs" => {
                                &["typeface", "panose", "pitchFamily", "charset"]
                            }
                            "font" => &["script", "typeface"],
                            _ => &[],
                        };
                        if unknown_font_attributes(element, allowed) {
                            result
                                .font_scheme
                                .as_mut()
                                .expect("font scheme")
                                .retained_ordinals
                                .push(ordinal);
                        }
                        if depth == section_depth + 1 {
                            if !["majorFont", "minorFont"].contains(&local)
                                || !font_groups.insert(local.to_owned())
                            {
                                return Err(malformed("invalid or duplicate font collection"));
                            }
                        } else if depth == section_depth + 2 {
                            let scheme = result.font_scheme.as_mut().expect("font section");
                            let collection = if stack.last().is_some_and(|n| n.is(A, "majorFont")) {
                                &mut scheme.major
                            } else {
                                &mut scheme.minor
                            };
                            match local {
                                "latin" => unique(&mut collection.latin, text_font(element)?)?,
                                "ea" => unique(&mut collection.east_asian, text_font(element)?)?,
                                "cs" => {
                                    unique(&mut collection.complex_script, text_font(element)?)?
                                }
                                "font" => collection.supplemental.push(SourceSupplementalFont {
                                    script: required(element, "script")?.into(),
                                    typeface: required(element, "typeface")?.into(),
                                }),
                                _ => {
                                    return Err(XmlError::Compatibility(
                                        "unknown theme font declaration".into(),
                                    ));
                                }
                            }
                        } else {
                            return Err(malformed("nested element inside theme font declaration"));
                        }
                    } else if stack
                        .get(section_depth)
                        .is_some_and(|n| n.is(A, "fmtScheme"))
                    {
                        let scheme = result.format_scheme.as_mut().expect("format section");
                        if depth == section_depth + 1 {
                            if element.name.namespace != A
                                || ![
                                    "fillStyleLst",
                                    "lnStyleLst",
                                    "effectStyleLst",
                                    "bgFillStyleLst",
                                ]
                                .contains(&local)
                                || !format_lists.insert(local.to_owned())
                            {
                                return Err(malformed("invalid or duplicate format style list"));
                            }
                        } else if depth == section_depth + 2 {
                            let list = stack.last().expect("format list").local.as_str();
                            let allowed = match list {
                                "lnStyleLst" => local == "ln",
                                "effectStyleLst" => local == "effectStyle",
                                _ => [
                                    "noFill",
                                    "solidFill",
                                    "gradFill",
                                    "blipFill",
                                    "pattFill",
                                    "grpFill",
                                ]
                                .contains(&local),
                            };
                            if element.name.namespace != A || !allowed {
                                return Err(XmlError::Compatibility(
                                    "unknown theme style entry".into(),
                                ));
                            }
                            let target = match list {
                                "fillStyleLst" => &mut scheme.fills,
                                "lnStyleLst" => &mut scheme.lines,
                                "effectStyleLst" => &mut scheme.effects,
                                _ => &mut scheme.background_fills,
                            };
                            target.push(SourceStyleEntry {
                                source_ordinal: ordinal,
                                local_name: local.into(),
                                line: None,
                                fill: None,
                                effect_style: None,
                            });
                            if ["fillStyleLst", "bgFillStyleLst", "effectStyleLst"].contains(&list)
                            {
                                fill_capture = Some((
                                    match list {
                                        "bgFillStyleLst" => PaintStyle::Background,
                                        "effectStyleLst" => PaintStyle::Effect,
                                        _ => PaintStyle::Fill,
                                    },
                                    target.len() - 1,
                                    paint::Reader::new(
                                        element,
                                        depth,
                                        ordinal,
                                        paint_budget,
                                        limits,
                                    )?,
                                ));
                            }
                            if list == "lnStyleLst" {
                                line_capture = Some((
                                    target.len() - 1,
                                    line::Reader::new(
                                        element,
                                        depth,
                                        ordinal,
                                        line_budget,
                                        limits,
                                    )?,
                                ));
                            }
                        }
                    }
                    stack.push(element.name.clone());
                }
                XmlEvent::End { .. } => {
                    let depth = stack.len() - 1;
                    if let Some(reader) = &mut defaults_capture {
                        reader.end(depth)?;
                        if reader.depth == depth {
                            result.text_defaults =
                                Some(defaults_capture.take().expect("default reader").finish()?);
                        }
                        stack.pop();
                        return Ok(());
                    }
                    if fill_capture
                        .as_ref()
                        .is_some_and(|(_, _, r)| r.depth == depth)
                    {
                        let (family, index, reader) =
                            fill_capture.take().expect("checked paint capture");
                        let declaration = reader.finish()?.publish(&mut result.effect_nodes)?;
                        let scheme = result.format_scheme.as_mut().expect("format scheme");
                        match (family, declaration) {
                            (PaintStyle::Fill, paint::Declaration::Fill(v)) => {
                                scheme.fills[index].fill = Some(v)
                            }
                            (PaintStyle::Background, paint::Declaration::Fill(v)) => {
                                scheme.background_fills[index].fill = Some(v)
                            }
                            (PaintStyle::Effect, paint::Declaration::EffectStyle(v)) => {
                                scheme.effects[index].effect_style = Some(v)
                            }
                            _ => return Err(malformed("theme paint kind mismatch")),
                        }
                    } else if let Some((_, _, reader)) = &mut fill_capture {
                        reader.end(depth)?;
                    }
                    if line_capture.as_ref().is_some_and(|(_, r)| r.depth == depth) {
                        let (index, reader) = line_capture.take().expect("checked line capture");
                        let line::Declaration::Line(value) = reader.finish() else {
                            unreachable!("theme captures line entries")
                        };
                        result.format_scheme.as_mut().expect("format scheme").lines[index].line =
                            Some(value);
                    } else if let Some((_, reader)) = &mut line_capture {
                        reader.end(depth)?;
                    }
                    if retained_depth == Some(depth) {
                        retained_depth = None;
                    } else if !extension && retained_depth.is_none() {
                        if color_depth == Some(depth) {
                            color_depth = None;
                        }
                        if let Some((slot_depth, which)) = slot
                            && depth == slot_depth
                        {
                            if !result
                                .color_scheme
                                .as_ref()
                                .expect("color section")
                                .colors
                                .contains_key(&which)
                            {
                                return Err(malformed("empty theme color slot"));
                            }
                            slot = None;
                        }
                    }
                    stack.pop();
                }
                XmlEvent::Text { text, .. } => {
                    if let Some(reader) = &defaults_capture {
                        reader.characters(text)?;
                    } else if let Some((_, reader)) = &line_capture {
                        reader.text(text)?;
                    } else if let Some((_, _, reader)) = &fill_capture {
                        reader.text(text)?;
                    } else if !extension && retained_depth.is_none() && !text.trim().is_empty() {
                        return Err(malformed("unexpected theme character data"));
                    }
                }
                _ => {}
            }
            Ok(())
        },
    )?;
    if !root_seen || (kind == SourceThemeKind::Theme && (!base_seen || sections.len() != 3)) {
        return Err(malformed("incomplete base theme").into());
    }
    if let Some(scheme) = &result.color_scheme
        && scheme.colors.len() != 12
    {
        return Err(malformed("theme must declare all twelve color slots").into());
    }
    if let Some(scheme) = &result.font_scheme {
        for font in [&scheme.major, &scheme.minor] {
            if font.latin.is_none() || font.east_asian.is_none() || font.complex_script.is_none() {
                return Err(malformed("theme font collection missing latin/ea/cs").into());
            }
        }
    }
    if let Some(scheme) = &result.format_scheme {
        if [
            &scheme.fills,
            &scheme.lines,
            &scheme.effects,
            &scheme.background_fills,
        ]
        .iter()
        .any(|v| v.len() < 3)
        {
            return Err(malformed("theme style lists need at least three entries").into());
        }
        notices.insert(
            "line declarations parsed; style inheritance and other format families unresolved"
                .into(),
        );
    }
    result.compatibility = compatibility::record(summary)?;
    result.notices = notices.into_iter().collect();
    Ok(result)
}

fn unique<T>(slot: &mut Option<T>, value: T) -> Result<(), XmlError> {
    if slot.is_some() {
        return Err(malformed("duplicate theme font declaration"));
    }
    *slot = Some(value);
    Ok(())
}
