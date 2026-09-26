use super::*;
use crate::{A, P};
use mo_opc::PartName;
use mo_xml::{ExpandedName, XmlError, XmlEvent};
use std::collections::BTreeSet;

struct Frame {
    index: usize,
    depth: usize,
    has_id: bool,
    has_text: bool,
}
enum FillOwner {
    Effects(usize),
    EffectReference(usize),
    RootGroupEffects,
    Object(usize),
    Reference(usize),
    Picture(usize),
    Background,
    RootGroup,
}
struct TextCapture {
    object: usize,
    paragraph: usize,
    run: usize,
    depth: usize,
    ordinal: usize,
    in_alternate: bool,
    physical_children: bool,
}
pub(super) struct ReadResult {
    pub surface: SourceSurface,
    pub text: BTreeMap<SourceTextTarget, usize>,
    pub transforms: BTreeMap<u32, transform_edit::Binding>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn read(
    bytes: &[u8],
    root: &str,
    kind: SurfaceKind,
    digest: &Digest,
    part: &PartName,
    limits: SourceLimits,
    object_count: &mut usize,
    text_bytes: &mut usize,
    line_budget: &mut line::Budget,
    geometry_budget: &mut geometry::Budget,
    paint_budget: &mut paint::Budget,
    text_budget: &mut text::Budget,
    check: &dyn Fn() -> bool,
) -> Result<ReadResult, crate::PptxError> {
    let mut surface = SourceSurface {
        text: text::SourceTextCatalog::default(),
        show_master_shapes: None,
        visual_issues: Vec::new(),
        color_mapping: None,
        resolved_color_mapping: None,
        links: SourceSurfaceLinks::default(),
        effective_theme: SourceThemeStack::default(),
        theme_selection: theme::SourceThemeSelection::default(),
        compatibility: SourceCompatibility::default(),
        kind,
        sha256: digest.clone(),
        name: None,
        hidden: false,
        root_object_id: 0,
        root_group_transform: None,
        background: None,
        root_group_fill: None,
        root_group_effects: None,
        effect_nodes: BTreeMap::new(),
        objects: Vec::new(),
        text_edit_barriers: Vec::new(),
        notices: Vec::new(),
    };
    let mut stack: Vec<ExpandedName> = Vec::new();
    let mut frames: Vec<Frame> = Vec::new();
    let mut seen_ids = BTreeSet::new();
    let mut root_id = false;
    let mut common = false;
    let mut tree = false;
    let mut surface_root_seen = false;
    let mut capture: Option<TextCapture> = None;
    let mut run_text_seen = BTreeSet::new();
    let mut bindings = BTreeMap::new();
    let mut transforms = BTreeMap::new();
    let mut notices = BTreeSet::new();
    let mut barriers = BTreeSet::new();
    let mut color_map = color_mapping::Reader::default();
    let mut line_capture: Option<(usize, line::Reader)> = None;
    let mut fill_capture: Option<(FillOwner, paint::Reader)> = None;
    let mut transform_capture: Option<(Option<usize>, transform::Reader)> = None;
    let mut geometry_capture: Option<(usize, geometry::Reader)> = None;
    let mut visual = visual::Reader::default();
    let mut style_capture: Option<text::Reader> = None;
    let mce_summary = mo_xml::mce::scan(
        bytes,
        limits.package.xml,
        &compatibility::profile(),
        check,
        |event| {
            let (event, source_ordinal, alternate_ancestors, extension) = match event {
                mo_xml::mce::Event::SourceElement { element, .. } => {
                    // Preserve dependencies in inactive branches too: another application
                    // can select them, so text ranges cannot be ignored when editing.
                    if element.name.is(P, "timing") {
                        barriers
                            .insert("timing references require text-range retargeting".to_owned());
                    }
                    if let Some(c) = &mut capture {
                        c.physical_children = true;
                    }
                    return Ok(());
                }
                mo_xml::mce::Event::Content {
                    event,
                    source_ordinal,
                    alternate_ancestors,
                    extension_content,
                    ..
                } => (
                    event,
                    source_ordinal,
                    alternate_ancestors,
                    extension_content,
                ),
            };
            match event {
                XmlEvent::Start { element, .. } => {
                    let depth = stack.len();
                    let node_ordinal = source_ordinal.expect("projected start has source ordinal");
                    if let Some(reader) = &mut style_capture {
                        reader.start(
                            element,
                            depth,
                            node_ordinal
                                .try_into()
                                .map_err(|_| XmlError::Limit("text style ordinal"))?,
                            extension,
                            text_budget,
                            line_budget,
                            paint_budget,
                            limits,
                        )?;
                    }
                    color_map.start(element, &stack, kind, node_ordinal, extension)?;
                    if let Some((_, reader)) = &mut line_capture {
                        reader.start(
                            element,
                            depth,
                            node_ordinal
                                .try_into()
                                .map_err(|_| XmlError::Limit("line source ordinal"))?,
                            extension,
                            line_budget,
                            paint_budget,
                            limits,
                        )?;
                    }
                    if let Some((_, reader)) = &mut fill_capture {
                        reader.start(
                            element,
                            depth,
                            node_ordinal
                                .try_into()
                                .map_err(|_| XmlError::Limit("fill source ordinal"))?,
                            extension,
                            paint_budget,
                            limits,
                        )?;
                    }
                    if let Some((_, reader)) = &mut geometry_capture {
                        reader.start(
                            element,
                            depth,
                            node_ordinal
                                .try_into()
                                .map_err(|_| XmlError::Limit("geometry source ordinal"))?,
                            extension,
                            geometry_budget,
                            limits,
                        )?;
                    }
                    if let Some((_, reader)) = &mut transform_capture {
                        reader.start(
                            element,
                            depth,
                            node_ordinal
                                .try_into()
                                .map_err(|_| XmlError::Limit("transform ordinal"))?,
                            extension,
                        )?;
                    }
                    if depth == 4
                        && stack[3].is(P, "grpSpPr")
                        && stack[2].is(P, "spTree")
                        && element.name.is(A, "xfrm")
                        && !extension
                    {
                        if surface.root_group_transform.is_some() {
                            return Err(malformed("duplicate shape-tree transform"));
                        }
                        transform_capture = Some((
                            None,
                            transform::Reader::new(
                                element,
                                depth,
                                node_ordinal
                                    .try_into()
                                    .map_err(|_| XmlError::Limit("transform ordinal"))?,
                                true,
                                !alternate_ancestors.is_empty(),
                            )?,
                        ));
                    }
                    if capture.is_some() {
                        return Err(malformed("native text leaf contains an element"));
                    }
                    if depth == 0 {
                        if surface_root_seen || !element.name.is(P, root) {
                            return Err(malformed("surface root mismatch"));
                        }
                        surface_root_seen = true;
                        surface.show_master_shapes =
                            element.attribute("showMasterSp").map(boolean).transpose()?;
                        if let Some(show) = element.attribute("show") {
                            surface.hidden = !boolean(show)?;
                        }
                    }
                    if depth == 1 && element.name.is(P, "cSld") {
                        if common {
                            return Err(malformed("duplicate common slide data"));
                        }
                        common = true;
                        surface.name = element.attribute("name").map(str::to_owned);
                    }
                    if depth == 2 && stack[1].is(P, "cSld") && element.name.is(P, "spTree") {
                        if tree {
                            return Err(malformed("duplicate shape tree"));
                        }
                        tree = true;
                    }
                    if depth == 4
                        && stack[2].is(P, "spTree")
                        && stack[3].is(P, "nvGrpSpPr")
                        && element.name.is(P, "cNvPr")
                    {
                        if root_id {
                            return Err(malformed("duplicate root object identity"));
                        }
                        let id = integer(element.attribute("id"), "root object ID")?;
                        if !seen_ids.insert(id) {
                            return Err(malformed("duplicate native object ID"));
                        }
                        surface.root_object_id = id;
                        root_id = true;
                    }
                    let in_tree =
                        (depth == 3 && stack[1].is(P, "cSld") && stack[2].is(P, "spTree"))
                            || frames.last().is_some_and(|f| {
                                depth == f.depth + 1
                                    && surface.objects[f.index].kind == SourceObjectKind::Group
                            });
                    let object_kind = if in_tree && element.name.namespace == P {
                        match element.name.local.as_str() {
                            "sp" => Some(SourceObjectKind::Shape),
                            "pic" => Some(SourceObjectKind::Picture),
                            "grpSp" => Some(SourceObjectKind::Group),
                            "cxnSp" => Some(SourceObjectKind::Connector),
                            "graphicFrame" => Some(SourceObjectKind::GraphicFrame),
                            _ => None,
                        }
                    } else {
                        None
                    };
                    if let Some(kind) = object_kind {
                        if *object_count >= limits.max_objects {
                            return Err(XmlError::Limit("source object count"));
                        }
                        *object_count += 1;
                        let parent_group =
                            frames.last().map(|f| surface.objects[f.index].native_id);
                        if frames.last().is_some_and(|f| !f.has_id) {
                            return Err(malformed("group identity must precede children"));
                        }
                        let index = surface.objects.len();
                        surface.objects.push(SourceObject {
                            hidden: None,
                            text_body_ordinal: None,
                            visual_issues: Vec::new(),
                            native_id: 0,
                            name: String::new(),
                            kind,
                            parent_group,
                            placeholder: None,
                            transform: None,
                            line: None,
                            line_reference: None,
                            geometry: None,
                            fill: None,
                            fill_reference: None,
                            picture_fill: None,
                            effects: None,
                            effect_reference: None,
                            use_background_fill: if kind == SourceObjectKind::Shape {
                                element
                                    .attribute("useBgFill")
                                    .map(|v| {
                                        Ok::<_, XmlError>(fill::SourceBackgroundFillUsage {
                                            source_ordinal: node_ordinal.try_into().map_err(
                                                |_| XmlError::Limit("fill source ordinal"),
                                            )?,
                                            value: boolean(v)?,
                                        })
                                    })
                                    .transpose()?
                            } else {
                                None
                            },
                            resolution: SourceObjectResolution::default(),
                            paragraphs: Vec::new(),
                        });
                        frames.push(Frame {
                            index,
                            depth,
                            has_id: false,
                            has_text: false,
                        });
                        if kind == SourceObjectKind::GraphicFrame {
                            notices.insert(
                                "graphicFrame content retained; semantic import pending".to_owned(),
                            );
                        }
                    }
                    if !extension {
                        let owner = if depth == 2
                            && stack.last().is_some_and(|n| n.is(P, "cSld"))
                            && element.name.is(P, "bg")
                        {
                            if surface.background.is_some() {
                                return Err(malformed("duplicate surface background"));
                            }
                            Some(FillOwner::Background)
                        } else if depth == 4
                            && stack[2].is(P, "spTree")
                            && stack[3].is(P, "grpSpPr")
                            && fill::is_fill(&element.name)
                        {
                            if surface.root_group_fill.is_some() {
                                return Err(malformed("duplicate root group fill"));
                            }
                            Some(FillOwner::RootGroup)
                        } else if depth == 4
                            && stack[2].is(P, "spTree")
                            && stack[3].is(P, "grpSpPr")
                            && paint::is_effect_properties(&element.name)
                        {
                            if surface.root_group_effects.is_some() {
                                return Err(malformed("duplicate root group effects"));
                            }
                            Some(FillOwner::RootGroupEffects)
                        } else if let Some(frame) = frames.last() {
                            let object = &surface.objects[frame.index];
                            let properties = if object.kind == SourceObjectKind::Group {
                                "grpSpPr"
                            } else {
                                "spPr"
                            };
                            if object.kind != SourceObjectKind::GraphicFrame
                                && depth == frame.depth + 2
                                && stack.last().is_some_and(|n| n.is(P, properties))
                                && fill::is_fill(&element.name)
                            {
                                if object.fill.is_some() {
                                    return Err(malformed("duplicate object fill"));
                                }
                                Some(FillOwner::Object(frame.index))
                            } else if object.kind != SourceObjectKind::GraphicFrame
                                && depth == frame.depth + 2
                                && stack.last().is_some_and(|n| n.is(P, properties))
                                && paint::is_effect_properties(&element.name)
                            {
                                if object.effects.is_some() {
                                    return Err(malformed("duplicate object effects"));
                                }
                                Some(FillOwner::Effects(frame.index))
                            } else if !matches!(
                                object.kind,
                                SourceObjectKind::Group | SourceObjectKind::GraphicFrame
                            ) && depth == frame.depth + 2
                                && stack.last().is_some_and(|n| n.is(P, "style"))
                                && element.name.is(A, "effectRef")
                            {
                                if object.effect_reference.is_some() {
                                    return Err(malformed("duplicate object effect reference"));
                                }
                                Some(FillOwner::EffectReference(frame.index))
                            } else if !matches!(
                                object.kind,
                                SourceObjectKind::Group | SourceObjectKind::GraphicFrame
                            ) && depth == frame.depth + 2
                                && stack.last().is_some_and(|n| n.is(P, "style"))
                                && element.name.is(A, "fillRef")
                            {
                                if object.fill_reference.is_some() {
                                    return Err(malformed("duplicate object fill reference"));
                                }
                                Some(FillOwner::Reference(frame.index))
                            } else if object.kind == SourceObjectKind::Picture
                                && depth == frame.depth + 1
                                && element.name.is(P, "blipFill")
                            {
                                if object.picture_fill.is_some() {
                                    return Err(malformed("duplicate picture fill"));
                                }
                                Some(FillOwner::Picture(frame.index))
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        if let Some(owner) = owner {
                            if fill_capture.is_some() {
                                return Err(malformed("nested surface fill capture"));
                            }
                            fill_capture = Some((
                                owner,
                                paint::Reader::new(
                                    element,
                                    depth,
                                    node_ordinal
                                        .try_into()
                                        .map_err(|_| XmlError::Limit("fill source ordinal"))?,
                                    paint_budget,
                                    limits,
                                )?,
                            ));
                        }
                    }
                    if let Some(frame) = frames.last_mut() {
                        let object = &mut surface.objects[frame.index];
                        if !extension
                            && depth == frame.depth + 2
                            && !matches!(
                                object.kind,
                                SourceObjectKind::Group | SourceObjectKind::GraphicFrame
                            )
                        {
                            if (element.name.is(A, "prstGeom") || element.name.is(A, "custGeom"))
                                && stack.last().is_some_and(|n| n.is(P, "spPr"))
                            {
                                if geometry_capture.is_some() || object.geometry.is_some() {
                                    return Err(malformed("duplicate object geometry declaration"));
                                }
                                geometry_capture = Some((
                                    frame.index,
                                    geometry::Reader::new(
                                        element,
                                        depth,
                                        node_ordinal.try_into().map_err(|_| {
                                            XmlError::Limit("geometry source ordinal")
                                        })?,
                                        geometry_budget,
                                        limits,
                                    )?,
                                ));
                            }
                            let direct = element.name.is(A, "ln")
                                && stack.last().is_some_and(|n| n.is(P, "spPr"));
                            let reference = element.name.is(A, "lnRef")
                                && stack.last().is_some_and(|n| n.is(P, "style"));
                            if direct || reference {
                                if line_capture.is_some()
                                    || (direct && object.line.is_some())
                                    || (reference && object.line_reference.is_some())
                                {
                                    return Err(malformed("duplicate object line declaration"));
                                }
                                line_capture = Some((
                                    frame.index,
                                    line::Reader::new(
                                        element,
                                        depth,
                                        node_ordinal
                                            .try_into()
                                            .map_err(|_| XmlError::Limit("line source ordinal"))?,
                                        line_budget,
                                        limits,
                                    )?,
                                ));
                            }
                        }
                        let nv = match object.kind {
                            SourceObjectKind::Shape => "nvSpPr",
                            SourceObjectKind::Picture => "nvPicPr",
                            SourceObjectKind::Group => "nvGrpSpPr",
                            SourceObjectKind::Connector => "nvCxnSpPr",
                            SourceObjectKind::GraphicFrame => "nvGraphicFramePr",
                        };
                        if depth == frame.depth + 2
                            && stack.last().is_some_and(|n| n.is(P, nv))
                            && element.name.is(P, "cNvPr")
                        {
                            if frame.has_id {
                                return Err(malformed("duplicate object identity"));
                            }
                            object.hidden = element.attribute("hidden").map(boolean).transpose()?;
                            object.native_id = integer(element.attribute("id"), "object ID")?;
                            if !seen_ids.insert(object.native_id) {
                                return Err(malformed("duplicate native object ID"));
                            }
                            object.name = element
                                .attribute("name")
                                .ok_or_else(|| malformed("missing native object name"))?
                                .to_owned();
                            frame.has_id = true;
                        }
                        if depth == frame.depth + 3
                            && stack[frame.depth + 1].is(P, nv)
                            && stack.last().is_some_and(|n| n.is(P, "nvPr"))
                            && element.name.is(P, "ph")
                        {
                            if object.placeholder.is_some() {
                                return Err(malformed("duplicate object placeholder"));
                            }
                            object.placeholder = Some(SourcePlaceholder::read(element)?);
                        }
                        let xfrm = (depth == frame.depth + 2
                            && element.name.is(A, "xfrm")
                            && stack
                                .last()
                                .is_some_and(|n| n.is(P, "spPr") || n.is(P, "grpSpPr")))
                            || (depth == frame.depth + 1
                                && object.kind == SourceObjectKind::GraphicFrame
                                && element.name.is(P, "xfrm"));
                        if xfrm {
                            if object.transform.is_some() {
                                return Err(malformed("duplicate object transform"));
                            }
                            transform_capture = Some((
                                Some(frame.index),
                                transform::Reader::new(
                                    element,
                                    depth,
                                    node_ordinal
                                        .try_into()
                                        .map_err(|_| XmlError::Limit("transform ordinal"))?,
                                    object.kind == SourceObjectKind::Group,
                                    !alternate_ancestors.is_empty(),
                                )?,
                            ));
                        }
                        if depth == frame.depth + 1 && element.name.is(P, "txBody") {
                            if frame.has_text || object.kind != SourceObjectKind::Shape {
                                return Err(malformed("invalid or duplicate native text body"));
                            }
                            frame.has_text = true;
                            object.text_body_ordinal = Some(
                                node_ordinal
                                    .try_into()
                                    .map_err(|_| XmlError::Limit("text body ordinal"))?,
                            );
                        }
                        if depth == frame.depth + 2
                            && stack.last().is_some_and(|n| n.is(P, "txBody"))
                            && element.name.is(A, "p")
                        {
                            object.paragraphs.push(Vec::new());
                        }
                        let in_paragraph = depth == frame.depth + 3
                            && stack.last().is_some_and(|n| n.is(A, "p"))
                            && stack[frame.depth + 1].is(P, "txBody");
                        if in_paragraph && element.name.namespace == A {
                            let kind = match element.name.local.as_str() {
                                "r" => Some(SourceRunKind::Text),
                                "br" => Some(SourceRunKind::Break),
                                "fld" => Some(SourceRunKind::Field),
                                _ => None,
                            };
                            if let Some(kind) = kind {
                                object
                                    .paragraphs
                                    .last_mut()
                                    .ok_or_else(|| malformed("run outside paragraph"))?
                                    .push(SourceRun {
                                        kind,
                                        text: String::new(),
                                        editable: false,
                                        edit_constraint: (kind == SourceRunKind::Field)
                                            .then_some(SourceTextConstraint::DynamicField),
                                    });
                            }
                        }
                        if depth == frame.depth + 4
                            && element.name.is(A, "t")
                            && stack[frame.depth + 1].is(P, "txBody")
                            && stack.last().is_some_and(|n| n.is(A, "r") || n.is(A, "fld"))
                        {
                            if !frame.has_id {
                                return Err(malformed("text before object identity"));
                            }
                            let paragraph = object
                                .paragraphs
                                .len()
                                .checked_sub(1)
                                .ok_or_else(|| malformed("text outside paragraph"))?;
                            let run = object.paragraphs[paragraph]
                                .len()
                                .checked_sub(1)
                                .ok_or_else(|| malformed("text outside run"))?;
                            if !run_text_seen.insert((frame.index, paragraph, run)) {
                                return Err(malformed("duplicate native run text"));
                            }
                            capture = Some(TextCapture {
                                object: frame.index,
                                paragraph,
                                run,
                                depth,
                                ordinal: node_ordinal,
                                in_alternate: !alternate_ancestors.is_empty(),
                                physical_children: false,
                            });
                        }
                    }
                    if element.name.local == "extLst" {
                        notices.insert(
                            "extension payloads retained without semantic interpretation"
                                .to_owned(),
                        );
                    }
                    if in_tree
                        && object_kind.is_none()
                        && !["nvGrpSpPr", "grpSpPr", "extLst"]
                            .contains(&element.name.local.as_str())
                    {
                        notices.insert(format!(
                            "unresolved tree child {{{}}}{}",
                            element.name.namespace, element.name.local
                        ));
                    }
                    let style_owner = frames.last().filter(|f| {
                        (depth == f.depth + 1 && element.name.is(P, "txBody"))
                            || (depth == f.depth + 2
                                && stack.last().is_some_and(|n| n.is(P, "style"))
                                && element.name.is(A, "fontRef"))
                    });
                    if !extension
                        && (style_owner.is_some()
                            || (kind == SurfaceKind::Master
                                && depth == 1
                                && element.name.is(P, "txStyles")))
                    {
                        if style_capture.is_some() {
                            return Err(malformed("nested source text root"));
                        }
                        if style_owner.is_some_and(|f| !f.has_id) {
                            return Err(malformed("text style before object identity"));
                        }
                        style_capture = Some(text::Reader::new(
                            element,
                            depth,
                            node_ordinal
                                .try_into()
                                .map_err(|_| XmlError::Limit("text style ordinal"))?,
                            style_owner.map(|f| surface.objects[f.index].native_id),
                            text_budget,
                            limits,
                        )?);
                    }
                    visual.start(
                        element,
                        &stack,
                        node_ordinal
                            .try_into()
                            .map_err(|_| XmlError::Limit("visual source ordinal"))?,
                        frames.last().map(|f| (f.index, f.depth)),
                        style_capture.is_some()
                            || fill_capture.is_some()
                            || line_capture.is_some()
                            || geometry_capture.is_some()
                            || transform_capture.is_some(),
                        extension,
                        &mut surface,
                    )?;
                    stack.push(element.name.clone());
                }
                XmlEvent::Text { text, .. } => {
                    if let Some(reader) = &style_capture {
                        reader.text(text)?;
                    }
                    visual.text(
                        text,
                        style_capture.is_some()
                            || fill_capture.is_some()
                            || line_capture.is_some()
                            || geometry_capture.is_some()
                            || transform_capture.is_some(),
                    )?;
                    color_map.text(text, &stack, extension)?;
                    if let Some((_, reader)) = &transform_capture {
                        reader.text(text)?;
                    }
                    if let Some((_, reader)) = &fill_capture {
                        reader.text(text)?;
                    }
                    if let Some((_, reader)) = &geometry_capture {
                        reader.text(text)?;
                    }
                    if let Some((_, reader)) = &line_capture {
                        reader.text(text)?;
                    }
                    if let Some(c) = &capture {
                        *text_bytes = text_bytes
                            .checked_add(text.len())
                            .ok_or(XmlError::Limit("source text bytes"))?;
                        if *text_bytes > limits.max_text_bytes {
                            return Err(XmlError::Limit("source text bytes"));
                        }
                        surface.objects[c.object].paragraphs[c.paragraph][c.run]
                            .text
                            .push_str(text);
                    }
                }
                XmlEvent::End { .. } => {
                    let depth = stack.len() - 1;
                    if let Some(reader) = &mut style_capture {
                        reader.end(depth)?;
                    }
                    if style_capture.as_ref().is_some_and(|r| r.depth == depth) {
                        style_capture
                            .take()
                            .expect("checked text style capture")
                            .finish(&mut surface.text)?;
                    }
                    if transform_capture
                        .as_ref()
                        .is_some_and(|(_, r)| r.depth == depth)
                    {
                        let (owner, reader) =
                            transform_capture.take().expect("checked transform capture");
                        transforms.insert(
                            owner,
                            transform_edit::Binding {
                                ordinal: reader.ordinal,
                                in_alternate: reader.in_alternate,
                            },
                        );
                        if let Some(owner) = owner {
                            surface.objects[owner].transform = Some(reader.finish());
                        } else {
                            surface.root_group_transform = Some(reader.finish());
                        }
                    } else if let Some((_, reader)) = &mut transform_capture {
                        reader.end(depth);
                    }
                    if fill_capture.as_ref().is_some_and(|(_, r)| r.depth == depth) {
                        let (owner, reader) = fill_capture.take().expect("checked fill capture");
                        match (owner, reader.finish()?.publish(&mut surface.effect_nodes)?) {
                            (FillOwner::Effects(index), paint::Declaration::Effects(value)) => {
                                surface.objects[index].effects = Some(value)
                            }
                            (
                                FillOwner::EffectReference(index),
                                paint::Declaration::EffectReference(value),
                            ) => surface.objects[index].effect_reference = Some(value),
                            (FillOwner::RootGroupEffects, paint::Declaration::Effects(value)) => {
                                surface.root_group_effects = Some(value)
                            }
                            (FillOwner::Object(index), paint::Declaration::Fill(value)) => {
                                surface.objects[index].fill = Some(value)
                            }
                            (FillOwner::Reference(index), paint::Declaration::Reference(value)) => {
                                surface.objects[index].fill_reference = Some(value)
                            }
                            (FillOwner::Picture(index), paint::Declaration::Fill(value)) => {
                                surface.objects[index].picture_fill = Some(value)
                            }
                            (FillOwner::Background, paint::Declaration::Background(value)) => {
                                surface.background = Some(value)
                            }
                            (FillOwner::RootGroup, paint::Declaration::Fill(value)) => {
                                surface.root_group_fill = Some(value)
                            }
                            _ => unreachable!("capture root and owner matched"),
                        }
                    } else if let Some((_, reader)) = &mut fill_capture {
                        reader.end(depth)?;
                    }
                    if geometry_capture
                        .as_ref()
                        .is_some_and(|(_, r)| r.depth == depth)
                    {
                        let (owner, reader) =
                            geometry_capture.take().expect("checked geometry capture");
                        surface.objects[owner].geometry = Some(reader.finish()?);
                    } else if let Some((_, reader)) = &mut geometry_capture {
                        reader.end(depth)?;
                    }
                    if line_capture.as_ref().is_some_and(|(_, r)| r.depth == depth) {
                        let (owner, reader) = line_capture.take().expect("checked line capture");
                        match reader.finish() {
                            line::Declaration::Line(value) => {
                                surface.objects[owner].line = Some(value)
                            }
                            line::Declaration::Reference(value) => {
                                surface.objects[owner].line_reference = Some(value)
                            }
                        }
                    } else if let Some((_, reader)) = &mut line_capture {
                        reader.end(depth)?;
                    }
                    if capture.as_ref().is_some_and(|c| c.depth == depth) {
                        let c = capture.take().expect("checked capture");
                        let object = &mut surface.objects[c.object];
                        let run = &mut object.paragraphs[c.paragraph][c.run];
                        if run.kind == SourceRunKind::Text {
                            run.edit_constraint = if c.in_alternate {
                                Some(SourceTextConstraint::CompatibilityBranch)
                            } else if c.physical_children {
                                Some(SourceTextConstraint::StructuredLeaf)
                            } else {
                                None
                            };
                            run.editable = run.edit_constraint.is_none();
                            bindings.insert(
                                SourceTextTarget {
                                    part: part.to_string(),
                                    object_id: object.native_id,
                                    paragraph: c
                                        .paragraph
                                        .try_into()
                                        .map_err(|_| XmlError::Limit("paragraph index"))?,
                                    run: c
                                        .run
                                        .try_into()
                                        .map_err(|_| XmlError::Limit("run index"))?,
                                },
                                c.ordinal,
                            );
                        }
                    }
                    if frames.last().is_some_and(|f| f.depth == depth) {
                        let frame = frames.pop().expect("checked frame");
                        if !frame.has_id {
                            return Err(malformed("missing native object identity"));
                        }
                        let object = &surface.objects[frame.index];
                        if frame.has_text && object.paragraphs.is_empty() {
                            return Err(malformed("text body has no paragraph"));
                        }
                        if object.paragraphs.iter().enumerate().any(|(p, runs)| {
                            runs.iter().enumerate().any(|(r, run)| {
                                run.kind == SourceRunKind::Text
                                    && !run_text_seen.contains(&(frame.index, p, r))
                            })
                        }) {
                            return Err(malformed("regular text run has no text leaf"));
                        }
                    }
                    visual.end(stack.len() - 1);
                    stack.pop();
                }
                _ => {}
            }
            Ok(())
        },
    )?;
    if !common || !tree || !root_id {
        return Err(crate::value(
            part.to_string(),
            "missing common data, shape tree or root identity",
        ));
    }
    surface.notices = notices.into_iter().collect();
    surface.color_mapping = color_map.finish()?;
    surface.compatibility = compatibility::record(mce_summary)?;
    surface.text_edit_barriers = barriers.into_iter().collect();
    if !surface.text_edit_barriers.is_empty() {
        for run in surface
            .objects
            .iter_mut()
            .flat_map(|o| o.paragraphs.iter_mut().flatten())
        {
            run.editable = false;
            run.edit_constraint = Some(SourceTextConstraint::TimingReferences);
        }
    }
    let transforms = transforms
        .into_iter()
        .map(|(owner, binding)| {
            let id = owner.map_or(surface.root_object_id, |i| surface.objects[i].native_id);
            (id, binding)
        })
        .collect();
    Ok(ReadResult {
        surface,
        text: bindings,
        transforms,
    })
}
