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
    table_data: bool,
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
    table_budget: &mut table::Budget,
    check: &dyn Fn() -> bool,
) -> Result<ReadResult, crate::PptxError> {
    let mut surface = SourceSurface {
        text: text::SourceTextCatalog::default(),
        show_master_shapes: None,
        header_footer: None,
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
    let mut text_roots = text::CatalogRoots::default();
    let mut stack: Vec<ExpandedName> = Vec::new();
    let mut frames: Vec<Frame> = Vec::new();
    let mut seen_ids = BTreeSet::new();
    let mut root_id = false;
    let mut common = false;
    let mut tree = false;
    let mut surface_root_seen = false;
    let mut content_capture: Option<(usize, text::ContentReader)> = None;
    let mut table_capture: Option<(usize, table::Reader)> = None;
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
                    if let Some((_, reader)) = &mut content_capture {
                        reader.physical_element();
                    }
                    if let Some((_, reader)) = &mut table_capture {
                        reader.physical_element();
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
                    if let Some((_, reader)) = &mut content_capture {
                        reader.start(
                            element,
                            depth,
                            node_ordinal,
                            !alternate_ancestors.is_empty(),
                        )?;
                    }
                    if let Some((_, reader)) = &mut table_capture {
                        reader.start(
                            element,
                            depth,
                            node_ordinal
                                .try_into()
                                .map_err(|_| XmlError::Limit("table ordinal"))?,
                            extension,
                            !alternate_ancestors.is_empty(),
                            table_budget,
                            text_budget,
                            line_budget,
                            paint_budget,
                            limits,
                        )?;
                    }
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
                    if depth == 1
                        && element.name.is(P, "hf")
                        && !extension
                        && matches!(kind, SurfaceKind::Master | SurfaceKind::Layout)
                    {
                        if surface.header_footer.is_some() {
                            return Err(malformed("duplicate header/footer declaration"));
                        }
                        surface.header_footer = Some(super::SourceHeaderFooter::read(element)?);
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
                            table: None,
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
                            table_data: false,
                        });
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
                            if !frame.has_id {
                                return Err(malformed("text before object identity"));
                            }
                            frame.has_text = true;
                            content_capture = Some((frame.index, text::ContentReader::new(depth)));
                            object.text_body_ordinal = Some(
                                node_ordinal
                                    .try_into()
                                    .map_err(|_| XmlError::Limit("text body ordinal"))?,
                            );
                        }
                        if !extension && object.kind == SourceObjectKind::GraphicFrame {
                            if depth == frame.depth + 2
                                && stack[frame.depth + 1].is(A, "graphic")
                                && element.name.is(A, "graphicData")
                            {
                                frame.table_data =
                                    element.attribute("uri") == Some(table::TABLE_URI);
                            }
                            if frame.table_data
                                && depth == frame.depth + 3
                                && stack.last().is_some_and(|n| n.is(A, "graphicData"))
                                && element.name.is(A, "tbl")
                            {
                                if !frame.has_id
                                    || object.table.is_some()
                                    || table_capture.is_some()
                                {
                                    return Err(malformed("invalid or duplicate native table"));
                                }
                                table_capture = Some((
                                    frame.index,
                                    table::Reader::new(
                                        element,
                                        depth,
                                        node_ordinal
                                            .try_into()
                                            .map_err(|_| XmlError::Limit("table ordinal"))?,
                                        object.native_id,
                                        table_budget,
                                        limits,
                                    )?,
                                ));
                            }
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
                            || table_capture.is_some()
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
                            || table_capture.is_some()
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
                    if let Some((_, reader)) = &mut content_capture {
                        reader.text(text, text_bytes, limits)?;
                    }
                    if let Some((_, reader)) = &mut table_capture {
                        reader.text(text, text_bytes, table_budget, limits)?;
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
                            .finish_indexed(&mut surface.text, &mut text_roots)?;
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
                    if let Some((_, reader)) = &mut content_capture {
                        reader.end(depth);
                    }
                    if content_capture
                        .as_ref()
                        .is_some_and(|(_, r)| r.depth == depth)
                    {
                        let (owner, reader) = content_capture.take().expect("text content capture");
                        reader.finish()?.publish(
                            part.as_str(),
                            &mut surface.objects[owner],
                            &mut bindings,
                        )?;
                    }
                    if let Some((_, reader)) = &mut table_capture {
                        reader.end(depth)?;
                    }
                    if table_capture
                        .as_ref()
                        .is_some_and(|(_, r)| r.depth == depth)
                    {
                        let (owner, reader) = table_capture.take().expect("table capture");
                        let read = reader.finish()?;
                        read.content.publish(
                            part.as_str(),
                            &mut surface.objects[owner],
                            &mut bindings,
                        )?;
                        surface.objects[owner].table = Some(read.table);
                        text_roots.append(&mut surface.text, read.text)?;
                        for (id, node) in read.effects {
                            if surface.effect_nodes.insert(id, node).is_some() {
                                return Err(malformed("duplicate table effect ordinal"));
                            }
                        }
                    }
                    if let Some(frame) = frames.last_mut()
                        && depth == frame.depth + 2
                        && stack.last().is_some_and(|n| n.is(A, "graphicData"))
                    {
                        frame.table_data = false;
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
                        if object.kind == SourceObjectKind::GraphicFrame && object.table.is_none() {
                            notices.insert(
                                "graphicFrame content retained; semantic import pending".to_owned(),
                            );
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
