use crate::{
    Attribute, Element, ExpandedName, XmlError, XmlEvent, XmlLimits, XmlSummary, encoding, names,
};
use quick_xml::{
    Reader, XmlVersion,
    events::{BytesStart, Event},
};
use std::collections::{BTreeMap, BTreeSet};

struct Frame {
    name: ExpandedName,
    namespaces: Vec<(String, Option<String>)>,
}

pub fn scan(
    input: &[u8],
    limits: XmlLimits,
    visitor: impl FnMut(XmlEvent<'_>) -> Result<(), XmlError>,
) -> Result<XmlSummary, XmlError> {
    scan_with_control(input, limits, &|| false, visitor)
}

pub fn scan_with_control(
    input: &[u8],
    limits: XmlLimits,
    cancelled: &dyn Fn() -> bool,
    visitor: impl FnMut(XmlEvent<'_>) -> Result<(), XmlError>,
) -> Result<XmlSummary, XmlError> {
    let (text, encoding) = encoding::decode(input, limits.max_bytes, cancelled)?;
    scan_decoded(&text, encoding, limits, cancelled, visitor)
}

pub(crate) fn scan_decoded(
    text: &str,
    encoding: crate::XmlEncoding,
    limits: XmlLimits,
    cancelled: &dyn Fn() -> bool,
    mut visitor: impl FnMut(XmlEvent<'_>) -> Result<(), XmlError>,
) -> Result<XmlSummary, XmlError> {
    if text.len() > limits.max_bytes {
        return Err(XmlError::Limit("decoded bytes"));
    }
    for (index, character) in text.chars().enumerate() {
        if index % 4096 == 0 && cancelled() {
            return Err(XmlError::Cancelled);
        }
        if !names::valid_char(character) {
            return Err(XmlError::Malformed("invalid XML 1.0 character".into()));
        }
    }
    let mut reader = Reader::from_str(text);
    reader.config_mut().check_comments = true;
    let mut frames = Vec::new();
    let mut namespaces = BTreeMap::from([("xml".into(), names::XML_NS.into())]);
    let mut summary = XmlSummary {
        encoding,
        elements: 0,
        max_depth: 0,
        text_bytes: 0,
    };
    let mut root_seen = false;
    let mut declaration_seen = false;
    loop {
        if cancelled() {
            return Err(XmlError::Cancelled);
        }
        let from = reader.buffer_position() as usize;
        let event = reader
            .read_event()
            .map_err(|e| XmlError::Malformed(e.to_string()))?;
        let span = from..reader.buffer_position() as usize;
        match event {
            Event::Start(ref start) | Event::Empty(ref start) => {
                if frames.is_empty() {
                    if root_seen {
                        return Err(XmlError::Malformed("multiple root elements".into()));
                    }
                    root_seen = true;
                }
                if frames.len() >= limits.max_depth {
                    return Err(XmlError::Limit("element depth"));
                }
                if summary.elements >= limits.max_elements {
                    return Err(XmlError::Limit("element count"));
                }
                let (element, changes) = element(start, limits, &mut namespaces)?;
                let depth = frames.len();
                summary.elements += 1;
                summary.max_depth = summary.max_depth.max(depth + 1);
                visitor(XmlEvent::Start {
                    element: &element,
                    namespaces: &namespaces,
                    depth,
                    span: span.clone(),
                })?;
                if matches!(event, Event::Empty(_)) {
                    visitor(XmlEvent::End {
                        name: &element.name,
                        depth,
                        span,
                    })?;
                    restore(&mut namespaces, changes);
                } else {
                    frames.push(Frame {
                        name: element.name,
                        namespaces: changes,
                    });
                }
            }
            Event::End(_) => {
                let frame = frames
                    .pop()
                    .ok_or_else(|| XmlError::Malformed("unmatched end tag".into()))?;
                visitor(XmlEvent::End {
                    name: &frame.name,
                    depth: frames.len(),
                    span,
                })?;
                restore(&mut namespaces, frame.namespaces);
            }
            Event::Text(value) => {
                let value = value.xml10_content();
                if frames.is_empty() && !value.chars().all(xml_space) {
                    return Err(XmlError::Malformed("text outside root element".into()));
                }
                if value.contains("]]>") {
                    return Err(XmlError::Malformed(
                        "CDATA terminator in character data".into(),
                    ));
                }
                emit_text(
                    &value,
                    frames.len(),
                    span,
                    &mut summary,
                    limits,
                    &mut visitor,
                )?;
            }
            Event::CData(value) => {
                if frames.is_empty() {
                    return Err(XmlError::Malformed("CDATA outside root element".into()));
                }
                emit_text(
                    &value.xml10_content(),
                    frames.len(),
                    span,
                    &mut summary,
                    limits,
                    &mut visitor,
                )?;
            }
            Event::GeneralRef(value) => {
                if frames.is_empty() {
                    return Err(XmlError::Malformed("reference outside root element".into()));
                }
                let encoded = format!("&{};", value.as_ref());
                let decoded = quick_xml::escape::unescape(&encoded)
                    .map_err(|e| XmlError::Malformed(e.to_string()))?;
                if !decoded.chars().all(names::valid_char) {
                    return Err(XmlError::Malformed("invalid character reference".into()));
                }
                emit_text(
                    &decoded,
                    frames.len(),
                    span,
                    &mut summary,
                    limits,
                    &mut visitor,
                )?;
            }
            Event::Decl(decl) => {
                if from != 0 || declaration_seen || root_seen {
                    return Err(XmlError::Malformed("XML declaration is not first".into()));
                }
                declaration_seen = true;
                let declaration = BytesStart::from_content(decl.as_ref(), 3);
                let mut last = 0;
                for attribute in declaration.attributes() {
                    let attribute = attribute.map_err(|e| XmlError::Malformed(e.to_string()))?;
                    let order = match attribute.key.as_ref() {
                        "version" => 1,
                        "encoding" => 2,
                        "standalone" => 3,
                        _ => {
                            return Err(XmlError::Malformed(
                                "unknown XML declaration attribute".into(),
                            ));
                        }
                    };
                    if order <= last || (last == 0 && order != 1) {
                        return Err(XmlError::Malformed(
                            "invalid XML declaration attribute order".into(),
                        ));
                    }
                    if order == 3 && attribute.value != "yes" && attribute.value != "no" {
                        return Err(XmlError::Malformed(
                            "invalid XML standalone declaration".into(),
                        ));
                    }
                    last = order;
                }
                if decl
                    .version()
                    .map_err(|e| XmlError::Malformed(e.to_string()))?
                    .as_ref()
                    != "1.0"
                {
                    return Err(XmlError::Malformed(
                        "only XML 1.0 is supported by OPC".into(),
                    ));
                }
                if let Some(declared) = decl.encoding() {
                    encoding::check_declaration(
                        encoding,
                        &declared.map_err(|e| XmlError::Malformed(e.to_string()))?,
                    )?;
                }
            }
            Event::DocType(_) => return Err(XmlError::Dtd),
            Event::Comment(_) => visitor(XmlEvent::Comment { span })?,
            Event::PI(pi) => {
                let target = pi.target();
                names::split_name(target)?;
                if target.eq_ignore_ascii_case("xml") {
                    return Err(XmlError::Malformed(
                        "reserved processing instruction target".into(),
                    ));
                }
                visitor(XmlEvent::ProcessingInstruction { span })?;
            }
            Event::Eof => break,
        }
    }
    if !root_seen || !frames.is_empty() {
        return Err(XmlError::Malformed(
            "missing or unclosed root element".into(),
        ));
    }
    Ok(summary)
}

fn xml_space(c: char) -> bool {
    matches!(c, ' ' | '\r' | '\n' | '\t')
}

fn emit_text(
    value: &str,
    depth: usize,
    span: std::ops::Range<usize>,
    summary: &mut XmlSummary,
    limits: XmlLimits,
    visitor: &mut impl FnMut(XmlEvent<'_>) -> Result<(), XmlError>,
) -> Result<(), XmlError> {
    summary.text_bytes = summary
        .text_bytes
        .checked_add(value.len())
        .ok_or(XmlError::Limit("text bytes"))?;
    if summary.text_bytes > limits.max_text_bytes {
        return Err(XmlError::Limit("text bytes"));
    }
    visitor(XmlEvent::Text {
        text: value,
        depth,
        span,
    })
}

type NamespaceChanges = Vec<(String, Option<String>)>;

fn element(
    start: &BytesStart<'_>,
    limits: XmlLimits,
    namespaces: &mut BTreeMap<String, String>,
) -> Result<(Element, NamespaceChanges), XmlError> {
    let qualified_name = start.name().as_ref().to_owned();
    names::split_name(&qualified_name)?;
    let mut raw = Vec::new();
    let mut changes = Vec::new();
    let mut size = 0_usize;
    for (i, attribute) in start.attributes().enumerate() {
        if i >= limits.max_attributes {
            return Err(XmlError::Limit("attributes per element"));
        }
        let attribute = attribute.map_err(|e| XmlError::Malformed(e.to_string()))?;
        let key = attribute.key.as_ref();
        size = size
            .checked_add(key.len())
            .and_then(|n| n.checked_add(attribute.value.len()))
            .ok_or(XmlError::Limit("attribute bytes"))?;
        if size > limits.max_attribute_bytes {
            return Err(XmlError::Limit("attribute bytes"));
        }
        names::split_name(key)?;
        if attribute.value.contains('<') {
            return Err(XmlError::Malformed(
                "literal less-than in attribute value".into(),
            ));
        }
        let value = attribute
            .normalized_value(XmlVersion::Explicit1_0)
            .map_err(|e| XmlError::Malformed(e.to_string()))?
            .into_owned();
        if !value.chars().all(names::valid_char) {
            return Err(XmlError::Malformed(
                "invalid attribute character reference".into(),
            ));
        }
        if key == "xmlns" || key.starts_with("xmlns:") {
            let prefix = key.strip_prefix("xmlns:").unwrap_or("");
            if prefix == "xmlns"
                || value == names::XMLNS_NS
                || (prefix == "xml") != (value == names::XML_NS)
                || (!prefix.is_empty() && value.is_empty())
            {
                return Err(XmlError::Malformed(
                    "invalid reserved namespace binding".into(),
                ));
            }
            changes.push((prefix.into(), namespaces.insert(prefix.into(), value)));
        } else {
            raw.push((key.to_owned(), value));
        }
    }
    let name = names::resolve(&qualified_name, false, namespaces)?;
    let mut seen = BTreeSet::new();
    let mut attributes = Vec::with_capacity(raw.len());
    for (qualified_name, value) in raw {
        let name = names::resolve(&qualified_name, true, namespaces)?;
        if !seen.insert(name.clone()) {
            return Err(XmlError::Malformed(
                "duplicate expanded attribute name".into(),
            ));
        }
        attributes.push(Attribute {
            name,
            qualified_name,
            value,
        });
    }
    Ok((
        Element {
            name,
            qualified_name,
            attributes,
        },
        changes,
    ))
}

fn restore(namespaces: &mut BTreeMap<String, String>, changes: NamespaceChanges) {
    for (key, previous) in changes.into_iter().rev() {
        if let Some(value) = previous {
            namespaces.insert(key, value);
        } else {
            namespaces.remove(&key);
        }
    }
}
