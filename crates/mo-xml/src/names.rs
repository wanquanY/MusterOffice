use crate::XmlError;
use std::collections::BTreeMap;

pub(crate) const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";
pub(crate) const XMLNS_NS: &str = "http://www.w3.org/2000/xmlns/";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ExpandedName {
    pub namespace: String,
    pub local: String,
}

impl ExpandedName {
    pub fn is(&self, namespace: &str, local: &str) -> bool {
        self.namespace == namespace && self.local == local
    }
}

fn name_start(c: char) -> bool {
    c == '_'
        || c.is_ascii_alphabetic()
        || matches!(c as u32,
        0xC0..=0xD6 | 0xD8..=0xF6 | 0xF8..=0x2FF | 0x370..=0x37D |
        0x37F..=0x1FFF | 0x200C..=0x200D | 0x2070..=0x218F | 0x2C00..=0x2FEF |
        0x3001..=0xD7FF | 0xF900..=0xFDCF | 0xFDF0..=0xFFFD | 0x10000..=0xEFFFF)
}

pub(crate) fn ncname(value: &str) -> bool {
    let mut chars = value.chars();
    chars.next().is_some_and(name_start) && chars.all(|c| {
        name_start(c)
            || c.is_ascii_digit()
            || matches!(c, '-' | '.' | '\u{B7}' | '\u{300}'..='\u{36F}' | '\u{203F}'..='\u{2040}')
    })
}

pub(crate) fn split_name(value: &str) -> Result<(Option<&str>, &str), XmlError> {
    let (prefix, local) = match value.split_once(':') {
        Some((p, l)) => (Some(p), l),
        None => (None, value),
    };
    if !ncname(local) || prefix.is_some_and(|p| !ncname(p)) {
        return Err(XmlError::Malformed(format!("invalid XML QName: {value}")));
    }
    Ok((prefix, local))
}

pub(crate) fn resolve(
    value: &str,
    attributes: bool,
    namespaces: &BTreeMap<String, String>,
) -> Result<ExpandedName, XmlError> {
    let (prefix, local) = split_name(value)?;
    let namespace = match prefix {
        Some("xmlns") => {
            return Err(XmlError::Malformed(
                "reserved xmlns prefix used as a name".into(),
            ));
        }
        Some(prefix) => namespaces
            .get(prefix)
            .cloned()
            .ok_or_else(|| XmlError::Malformed(format!("unbound namespace prefix: {prefix}")))?,
        None if attributes => String::new(),
        None => namespaces.get("").cloned().unwrap_or_default(),
    };
    Ok(ExpandedName {
        namespace,
        local: local.into(),
    })
}

pub(crate) fn valid_char(c: char) -> bool {
    matches!(c as u32, 0x9 | 0xA | 0xD | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF)
}
