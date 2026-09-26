use super::*;
use crate::{P, PptxError, R, cancelled, value};
use mo_opc::{PartName, RelationshipSource, RelationshipTarget};
use mo_xml::{ExpandedName, XmlEvent};
use std::collections::BTreeSet;

const MAIN_TYPES: [&str; 6] = [
    "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml",
    "application/vnd.openxmlformats-officedocument.presentationml.slideshow.main+xml",
    "application/vnd.openxmlformats-officedocument.presentationml.template.main+xml",
    "application/vnd.ms-powerpoint.presentation.macroEnabled.main+xml",
    "application/vnd.ms-powerpoint.slideshow.macroEnabled.main+xml",
    "application/vnd.ms-powerpoint.template.macroEnabled.main+xml",
];

pub(super) fn read(
    source: &dyn PackageRead,
    limits: SourceLimits,
    check: &dyn Fn() -> bool,
) -> Result<BoundIndex, PptxError> {
    cancelled(check)?;
    let root = source
        .relationships()
        .get(&RelationshipSource::Package)
        .ok_or_else(|| value("package", "no package relationships"))?;
    let mut main = None;
    for rel in root {
        if rel.relationship_type
            == "http://purl.oclc.org/ooxml/officeDocument/relationships/officeDocument"
        {
            return Err(PptxError::Unsupported(
                "Strict PresentationML source reading".into(),
            ));
        }
        if rel.relationship_type == format!("{R}/officeDocument") {
            if main.is_some() {
                return Err(value("package", "multiple officeDocument relationships"));
            }
            main = Some(internal(rel)?);
        }
    }
    let main = main.ok_or_else(|| value("package", "no presentation relationship"))?;
    let content_type = &source.parts()[&main].content_type;
    if !MAIN_TYPES.contains(&content_type.as_str()) {
        return Err(value(
            main.to_string(),
            "officeDocument is not a presentation",
        ));
    }
    let bytes = source.read_part(&main, limits.package.xml.max_bytes as u64, check)?;
    let mut stack: Vec<ExpandedName> = Vec::new();
    let mut slide_refs = Vec::new();
    let mut ids = BTreeSet::new();
    let mut size = None;
    let mut list_seen = false;
    let mut notices = BTreeSet::new();
    let mut root_seen = false;
    let mut text_catalog = text::SourceTextCatalog::default();
    let mut text_capture: Option<text::Reader> = None;
    let mut text_budget = text::Budget::default();
    let mut line_budget = line::Budget::default();
    let mut paint_budget = paint::Budget::default();
    let mce_summary = mo_xml::mce::scan(
        &bytes,
        limits.package.xml,
        &compatibility::profile(),
        check,
        |event| {
            let mo_xml::mce::Event::Content {
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
                    let depth = stack.len();
                    let ordinal = source_ordinal
                        .expect("projected element")
                        .try_into()
                        .map_err(|_| mo_xml::XmlError::Limit("text style ordinal"))?;
                    if let Some(reader) = &mut text_capture {
                        reader.start(
                            element,
                            depth,
                            ordinal,
                            extension_content,
                            &mut text_budget,
                            &mut line_budget,
                            &mut paint_budget,
                            limits,
                        )?;
                    }
                    if depth == 1 && element.name.is(P, "defaultTextStyle") && !extension_content {
                        if text_capture.is_some() {
                            return Err(malformed("nested default text style"));
                        }
                        text_capture = Some(text::Reader::new(
                            element,
                            depth,
                            ordinal,
                            None,
                            &mut text_budget,
                            limits,
                        )?);
                    }
                    if stack.is_empty() {
                        if root_seen || !element.name.is(P, "presentation") {
                            return Err(malformed(
                                "presentation root mismatch or multiple projected roots",
                            ));
                        }
                        root_seen = true;
                    }
                    if stack.len() == 1 && element.name.is(P, "sldIdLst") {
                        if list_seen {
                            return Err(malformed("duplicate slide list"));
                        }
                        list_seen = true;
                    }
                    if stack.len() == 2 && stack[1].is(P, "sldIdLst") && element.name.is(P, "sldId")
                    {
                        if slide_refs.len() >= limits.max_surfaces {
                            return Err(mo_xml::XmlError::Limit("slide count"));
                        }
                        let id: u32 = integer(element.attribute("id"), "slide ID")?;
                        if !(256..2147483648).contains(&id) || !ids.insert(id) {
                            return Err(malformed("duplicate or invalid native slide ID"));
                        }
                        let rel = element
                            .attributes
                            .iter()
                            .find(|a| a.name.is(R, "id"))
                            .ok_or_else(|| malformed("missing slide relationship ID"))?;
                        slide_refs.push((id, rel.value.clone()));
                    }
                    if stack.len() == 1 && element.name.is(P, "sldSz") {
                        if size.is_some() {
                            return Err(malformed("duplicate slide size"));
                        }
                        let width = coordinate(element.attribute("cx"), "slide width")?;
                        let height = coordinate(element.attribute("cy"), "slide height")?;
                        if [width, height]
                            .iter()
                            .any(|v| !(914400..=51206400).contains(&v.get()))
                        {
                            return Err(malformed("native slide size out of range"));
                        }
                        size = Some(mo_presentation_model::Size { width, height });
                    }
                    stack.push(element.name.clone());
                }
                XmlEvent::End { .. } => {
                    let depth = stack.len() - 1;
                    if let Some(reader) = &mut text_capture {
                        reader.end(depth)?;
                    }
                    if text_capture.as_ref().is_some_and(|r| r.depth == depth) {
                        text_capture
                            .take()
                            .expect("text capture")
                            .finish(&mut text_catalog)?;
                    }
                    stack.pop();
                }
                XmlEvent::Text { text, .. } => {
                    if let Some(reader) = &text_capture {
                        reader.text(text)?;
                    }
                }
                _ => {}
            }
            Ok(())
        },
    )?;
    if !root_seen {
        return Err(value(
            main.to_string(),
            "compatibility profile produced no presentation root",
        ));
    }
    let rels = source
        .relationships()
        .get(&RelationshipSource::Part(main.clone()));
    let mut slides = Vec::new();
    let mut slide_parts = BTreeSet::new();
    let mut queue = BTreeMap::new();
    for (id, rel_id) in slide_refs {
        let rel = rels
            .into_iter()
            .flatten()
            .find(|r| r.id == rel_id)
            .ok_or_else(|| value(main.to_string(), "slide relationship does not exist"))?;
        if rel.relationship_type != format!("{R}/slide") {
            return Err(value(main.to_string(), "slide relationship type mismatch"));
        }
        let part = internal(rel)?;
        if !slide_parts.insert(part.clone()) {
            return Err(value(
                main.to_string(),
                "multiple slide IDs reference one part",
            ));
        }
        slides.push(SourceSlide {
            native_id: id,
            part: part.to_string(),
        });
        queue.insert(part, SurfaceKind::Slide);
    }
    for rel in rels.into_iter().flatten() {
        if rel.relationship_type == format!("{R}/slideMaster") {
            queue.insert(internal(rel)?, SurfaceKind::Master);
        }
    }
    let mut surfaces = BTreeMap::new();
    let mut bindings = BTreeMap::new();
    let mut transforms = BTreeMap::new();
    let mut object_count = 0;
    let mut text_bytes = 0;
    let mut geometry_budget = geometry::Budget::default();
    while let Some((part, kind)) = queue.pop_first() {
        cancelled(check)?;
        if let Some(existing) = surfaces.get(part.as_str()) {
            let existing: &SourceSurface = existing;
            if existing.kind != kind {
                return Err(value(
                    part.to_string(),
                    "surface relationship kind conflict",
                ));
            }
            continue;
        }
        if surfaces.len() >= limits.max_surfaces {
            return Err(PptxError::Limit("source surfaces"));
        }
        let (suffix, root) = match kind {
            SurfaceKind::Slide => ("slide", "sld"),
            SurfaceKind::Master => ("slideMaster", "sldMaster"),
            SurfaceKind::Layout => ("slideLayout", "sldLayout"),
        };
        if source.parts()[&part].content_type
            != format!("application/vnd.openxmlformats-officedocument.presentationml.{suffix}+xml")
        {
            return Err(value(part.to_string(), "surface content type mismatch"));
        }
        let bytes = source.read_part(&part, limits.package.xml.max_bytes as u64, check)?;
        let mut read = surface::read(
            &bytes,
            root,
            kind,
            &source.parts()[&part].sha256,
            &part,
            limits,
            &mut object_count,
            &mut text_bytes,
            &mut line_budget,
            &mut geometry_budget,
            &mut paint_budget,
            &mut text_budget,
            check,
        )?;
        read.surface.links = links::read(source, &part, kind)?;
        bindings.extend(read.text);
        transforms.insert(part.to_string(), read.transforms);
        surfaces.insert(part.to_string(), read.surface);
        for rel in source
            .relationships()
            .get(&RelationshipSource::Part(part.clone()))
            .into_iter()
            .flatten()
        {
            let next = if rel.relationship_type == format!("{R}/slideMaster") {
                Some(SurfaceKind::Master)
            } else if rel.relationship_type == format!("{R}/slideLayout") {
                Some(SurfaceKind::Layout)
            } else {
                None
            };
            if let Some(next) = next {
                let target = internal(rel)?;
                if queue.insert(target, next).is_some_and(|old| old != next) {
                    return Err(value(part.to_string(), "conflicting surface relationship"));
                }
            }
        }
    }
    inheritance::resolve(&mut surfaces, check)?;
    let themes = theme::load(
        source,
        &mut surfaces,
        limits,
        &mut line_budget,
        &mut paint_budget,
        &mut text_budget,
        check,
    )?;
    notices.insert(
        "partial native projection; styles, layout, advanced objects and playback unresolved"
            .into(),
    );
    let object_positions = surfaces
        .iter()
        .map(|(part, surface)| {
            (
                part.clone(),
                surface
                    .objects
                    .iter()
                    .enumerate()
                    .map(|(i, o)| (o.native_id, i))
                    .collect(),
            )
        })
        .collect();
    Ok(BoundIndex {
        object_positions,
        index: SourceIndex {
            text: text_catalog,
            compatibility_profile: compatibility::PROFILE_ID.into(),
            main_compatibility: compatibility::record(mce_summary)?,
            source_sha256: source.sha256().clone(),
            byte_length: ByteLength::new(source.byte_length()),
            main_part: main.to_string(),
            main_content_type: content_type.clone(),
            contains_signatures: source.has_signatures(),
            page_size: size,
            slides,
            surfaces,
            themes,
            notices: notices.into_iter().collect(),
        },
        bindings,
        transforms,
    })
}

pub(super) fn internal(rel: &mo_opc::Relationship) -> Result<PartName, PptxError> {
    match &rel.resolved {
        RelationshipTarget::Internal {
            part,
            fragment: None,
        } => Ok(part.clone()),
        _ => Err(value(
            &rel.id,
            "presentation surface relationship must target an internal whole part",
        )),
    }
}
