use crate::{OpcError, PackageLimits, PartName, resolve_internal_target};
use mo_xml::{Element, XmlError, XmlEvent};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const TYPES_NS: &str = "http://schemas.openxmlformats.org/package/2006/content-types";
pub(crate) const RELS_NS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RelationshipSource {
    Package,
    Part(PartName),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelationshipTarget {
    Internal {
        part: PartName,
        fragment: Option<String>,
    },
    External,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relationship {
    pub id: String,
    pub relationship_type: String,
    /// Original URI spelling is retained, including relative paths and fragment.
    pub target: String,
    pub resolved: RelationshipTarget,
}

impl Relationship {
    pub fn new(
        source: &RelationshipSource,
        id: String,
        relationship_type: String,
        target: String,
        external: bool,
    ) -> Result<Self, OpcError> {
        if !mo_xml::is_ncname(&id) {
            return Err(OpcError::Structure(
                "relationship Id is not an XML ID".into(),
            ));
        }
        if !absolute_uri(&relationship_type) {
            return Err(OpcError::Structure(
                "relationship Type must be an absolute URI".into(),
            ));
        }
        if target.is_empty() || !uri_characters(&target) {
            return Err(OpcError::Structure(
                "invalid relationship Target URI".into(),
            ));
        }
        let resolved = if external {
            RelationshipTarget::External
        } else {
            let (part, fragment) = resolve_internal_target(source, &target)?;
            RelationshipTarget::Internal { part, fragment }
        };
        Ok(Self {
            id,
            relationship_type,
            target,
            resolved,
        })
    }
}

fn uri_characters(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'%' {
            if !bytes
                .get(index + 1..index + 3)
                .is_some_and(|p| p.iter().all(u8::is_ascii_hexdigit))
            {
                return false;
            }
            index += 3;
        } else {
            if byte <= b' ' || byte >= 0x7F || b"<>\"{}|\\^`".contains(&byte) {
                return false;
            }
            index += 1;
        }
    }
    true
}

fn absolute_uri(value: &str) -> bool {
    let Some((scheme, _)) = value.split_once(':') else {
        return false;
    };
    scheme
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphabetic)
        && scheme
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"+-.".contains(&b))
        && uri_characters(value)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContentTypes {
    pub(crate) defaults: BTreeMap<String, String>,
    pub(crate) overrides: BTreeMap<PartName, String>,
}

impl ContentTypes {
    pub fn content_type(&self, part: &PartName) -> Option<&str> {
        self.overrides
            .get(part)
            .or_else(|| {
                part.zip_name()
                    .rsplit_once('.')
                    .and_then(|(_, extension)| self.defaults.get(&extension.to_ascii_lowercase()))
            })
            .map(String::as_str)
    }
    pub fn defaults(&self) -> &BTreeMap<String, String> {
        &self.defaults
    }
    pub fn overrides(&self) -> &BTreeMap<PartName, String> {
        &self.overrides
    }
    pub(crate) fn from_xml(
        bytes: &[u8],
        limits: PackageLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Self, OpcError> {
        let mut result = Self::default();
        mo_xml::scan_with_control(bytes, limits.xml, cancelled, |event| {
            match event {
                XmlEvent::Start { element, depth, .. } => match depth {
                    0 if element.name.is(TYPES_NS, "Types") => attributes(element, &[])?,
                    1 if element.name.is(TYPES_NS, "Default") => {
                        attributes(element, &["Extension", "ContentType"])?;
                        let extension = required(element, "Extension")?.to_ascii_lowercase();
                        if extension.is_empty()
                            || !extension.is_ascii()
                            || extension.contains(['.', '/', '\\', '%'])
                            || extension.chars().any(char::is_whitespace)
                        {
                            return Err(invalid("invalid default extension"));
                        }
                        let content_type = required(element, "ContentType")?;
                        validate_content_type(content_type).map_err(|e| invalid(&e.to_string()))?;
                        if result
                            .defaults
                            .insert(extension, content_type.into())
                            .is_some()
                        {
                            return Err(invalid("duplicate default content type"));
                        }
                    }
                    1 if element.name.is(TYPES_NS, "Override") => {
                        attributes(element, &["PartName", "ContentType"])?;
                        let name = PartName::new(required(element, "PartName")?)
                            .map_err(|e| invalid(&e.to_string()))?;
                        let content_type = required(element, "ContentType")?;
                        validate_content_type(content_type).map_err(|e| invalid(&e.to_string()))?;
                        if result.overrides.insert(name, content_type.into()).is_some() {
                            return Err(invalid("duplicate override content type"));
                        }
                    }
                    _ => return Err(invalid("unexpected content-types element")),
                },
                XmlEvent::Text { text, .. } if !text.chars().all(char::is_whitespace) => {
                    return Err(invalid("text inside content-types metadata"));
                }
                _ => (),
            }
            Ok(())
        })
        .map_err(|source| xml_error("/[Content_Types].xml".into(), source))?;
        Ok(result)
    }
    pub(crate) fn to_xml(&self) -> Result<Vec<u8>, OpcError> {
        let mut out =
            format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><Types xmlns=\"{TYPES_NS}\">");
        for (extension, content_type) in &self.defaults {
            out.push_str(&format!(
                "<Default Extension=\"{}\" ContentType=\"{}\"/>",
                escape(extension)?,
                escape(content_type)?
            ));
        }
        for (part, content_type) in &self.overrides {
            out.push_str(&format!(
                "<Override PartName=\"{}\" ContentType=\"{}\"/>",
                escape(part.as_str())?,
                escape(content_type)?
            ));
        }
        out.push_str("</Types>");
        Ok(out.into_bytes())
    }
}

pub(crate) fn parse_relationships(
    source: &RelationshipSource,
    bytes: &[u8],
    limits: PackageLimits,
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<Relationship>, OpcError> {
    let mut result = Vec::new();
    let mut ids = BTreeSet::new();
    mo_xml::scan_with_control(bytes, limits.xml, cancelled, |event| {
        match event {
            XmlEvent::Start { element, depth, .. } => match depth {
                0 if element.name.is(RELS_NS, "Relationships") => attributes(element, &[])?,
                1 if element.name.is(RELS_NS, "Relationship") => {
                    if result.len() >= limits.max_relationships {
                        return Err(XmlError::Limit("relationship count"));
                    }
                    attributes(element, &["Id", "Type", "Target", "TargetMode"])?;
                    let external = match element.attribute("TargetMode") {
                        None | Some("Internal") => false,
                        Some("External") => true,
                        _ => return Err(invalid("invalid TargetMode")),
                    };
                    let relationship = Relationship::new(
                        source,
                        required(element, "Id")?.into(),
                        required(element, "Type")?.into(),
                        required(element, "Target")?.into(),
                        external,
                    )
                    .map_err(|e| invalid(&e.to_string()))?;
                    if !ids.insert(relationship.id.clone()) {
                        return Err(invalid("duplicate relationship Id in source scope"));
                    }
                    result.push(relationship);
                }
                _ => return Err(invalid("unexpected relationships element")),
            },
            XmlEvent::Text { text, .. } if !text.chars().all(char::is_whitespace) => {
                return Err(invalid("text inside relationships metadata"));
            }
            _ => (),
        }
        Ok(())
    })
    .map_err(|source_error| xml_error(format!("{source:?}"), source_error))?;
    Ok(result)
}

pub(crate) fn serialize_relationships(relationships: &[Relationship]) -> Result<Vec<u8>, OpcError> {
    let mut out =
        format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"{RELS_NS}\">");
    for r in relationships {
        out.push_str(&format!(
            "<Relationship Id=\"{}\" Type=\"{}\" Target=\"{}\"",
            escape(&r.id)?,
            escape(&r.relationship_type)?,
            escape(&r.target)?
        ));
        if r.resolved == RelationshipTarget::External {
            out.push_str(" TargetMode=\"External\"");
        }
        out.push_str("/>");
    }
    out.push_str("</Relationships>");
    Ok(out.into_bytes())
}

pub(crate) fn validate_content_type(value: &str) -> Result<(), OpcError> {
    let token = |s: &str| {
        !s.is_empty()
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"!#$&^_.+-".contains(&b))
    };
    if !value
        .split_once('/')
        .is_some_and(|(a, b)| token(a) && token(b))
    {
        return Err(OpcError::Structure(
            "invalid or parameterized OPC content type".into(),
        ));
    }
    Ok(())
}

fn required<'a>(element: &'a Element, attribute: &str) -> Result<&'a str, XmlError> {
    element
        .attribute(attribute)
        .ok_or_else(|| invalid(&format!("missing attribute {attribute}")))
}
fn attributes(element: &Element, allowed: &[&str]) -> Result<(), XmlError> {
    if element
        .attributes
        .iter()
        .any(|a| !a.name.namespace.is_empty() || !allowed.contains(&a.name.local.as_str()))
    {
        return Err(invalid("unexpected metadata attribute"));
    }
    Ok(())
}
fn invalid(message: &str) -> XmlError {
    XmlError::Malformed(message.into())
}

pub(crate) fn xml_error(part: String, source: XmlError) -> OpcError {
    if matches!(source, XmlError::Cancelled) {
        OpcError::Cancelled
    } else {
        OpcError::Xml { part, source }
    }
}

pub(crate) fn is_xml(content_type: &str) -> bool {
    let value = content_type.to_ascii_lowercase();
    value.ends_with("+xml") || value == "application/xml" || value == "text/xml"
}
pub(crate) fn escape(value: &str) -> Result<String, OpcError> {
    if value
        .chars()
        .any(|c| c.is_control() || matches!(c, '\u{FFFE}' | '\u{FFFF}'))
    {
        return Err(OpcError::Structure("invalid metadata character".into()));
    }
    Ok(value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;"))
}
