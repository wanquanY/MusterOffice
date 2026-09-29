use super::super::Budget;
use super::*;
use crate::{
    A, PptxError, R,
    source::{SourceLimits, compatibility, line, malformed, paint, presentation::internal},
    value,
};
use mo_opc::{PackageRead, PartName, RelationshipSource};
use mo_xml::XmlEvent;

pub(in crate::source) fn load(
    source: &dyn PackageRead,
    main: &PartName,
    budget: &mut Budget,
    line_budget: &mut line::Budget,
    paint_budget: &mut paint::Budget,
    limits: SourceLimits,
    check: &dyn Fn() -> bool,
) -> Result<Option<Box<SourceTableStylePart>>, PptxError> {
    let mut target = None;
    for rel in source
        .relationships()
        .get(&RelationshipSource::Part(main.clone()))
        .into_iter()
        .flatten()
    {
        crate::cancelled(check)?;
        if rel.relationship_type == format!("{R}/tableStyles")
            && target.replace(internal(rel)?).is_some()
        {
            return Err(value(main.to_string(), "multiple table style parts"));
        }
    }
    let Some(part) = target else { return Ok(None) };
    let info = source
        .parts()
        .get(&part)
        .ok_or_else(|| value(part.to_string(), "missing table style part"))?;
    if info.content_type
        != "application/vnd.openxmlformats-officedocument.presentationml.tableStyles+xml"
    {
        return Err(value(part.to_string(), "table style content type mismatch"));
    }
    let bytes = source.read_part(&part, limits.package.xml.max_bytes as u64, check)?;
    let mut root = None;
    let mut default_id = None;
    let mut styles = BTreeMap::new();
    let mut retained = vec![];
    let mut depth = 0usize;
    let mut opaque = None;
    let mut capture: Option<Reader> = None;
    let summary = mo_xml::mce::scan(
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
                    let ordinal = source_ordinal
                        .expect("style source ordinal")
                        .try_into()
                        .map_err(|_| mo_xml::XmlError::Limit("table style ordinal"))?;
                    if let Some(reader) = &mut capture {
                        reader.start(
                            element,
                            depth,
                            ordinal,
                            extension_content,
                            budget,
                            line_budget,
                            paint_budget,
                            limits,
                        )?;
                    } else {
                        budget.element(element, limits)?;
                        if opaque.is_some() {
                        } else if depth == 0 {
                            if root.is_some() || !element.name.is(A, "tblStyleLst") {
                                return Err(malformed("table style list root"));
                            }
                            root = Some(ordinal);
                            let id = required(element, "def")?;
                            read::guid(id)?;
                            default_id = Some(id.to_owned());
                            if element
                                .attributes
                                .iter()
                                .any(|a| !a.name.namespace.is_empty() || a.name.local != "def")
                            {
                                retained.push(ordinal);
                            }
                        } else if depth == 1 && element.name.is(A, "tblStyle") && !extension_content
                        {
                            capture = Some(Reader::new(element, depth, ordinal, budget, limits)?);
                        } else {
                            retained.push(ordinal);
                            opaque = Some(depth);
                        }
                    }
                    depth += 1;
                }
                XmlEvent::End { .. } => {
                    depth = depth
                        .checked_sub(1)
                        .ok_or_else(|| malformed("table style closing depth"))?;
                    if let Some(reader) = &mut capture {
                        reader.end(depth)?;
                    }
                    if capture.as_ref().is_some_and(|r| r.depth == depth) {
                        let style = capture.take().expect("style capture").finish()?;
                        let key = read::guid(&style.style_id)?;
                        if styles.insert(key, style).is_some() {
                            return Err(malformed("duplicate table style identity"));
                        }
                    }
                    if opaque == Some(depth) {
                        opaque = None;
                    }
                }
                XmlEvent::Text { text, .. } => {
                    if let Some(reader) = &capture {
                        reader.text(text)?;
                    } else if opaque.is_none() && !text.trim().is_empty() {
                        return Err(malformed("text outside table style declaration"));
                    }
                }
                _ => (),
            }
            Ok(())
        },
    )?;
    if depth != 0 || capture.is_some() || opaque.is_some() {
        return Err(value(part.to_string(), "incomplete table style list"));
    }
    Ok(Some(Box::new(SourceTableStylePart {
        part: part.to_string(),
        sha256: info.sha256.clone(),
        source_ordinal: root
            .ok_or_else(|| value(part.to_string(), "no projected table style list"))?,
        default_style_id: default_id
            .ok_or_else(|| value(part.to_string(), "no default table style"))?,
        styles,
        retained_ordinals: retained,
        compatibility: compatibility::record(summary)?,
    })))
}
