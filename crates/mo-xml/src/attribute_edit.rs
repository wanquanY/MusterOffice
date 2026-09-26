//! Atomic edits to source-bound attributes; no reserialization of untouched XML.
use crate::{
    ExpandedName, XmlError, XmlEvent, XmlLimits,
    edit_bytes::{Patch, apply_patches},
    encoding, names, scan,
};
use std::{collections::BTreeMap, ops::Range};

/// Internal source binding, not an Agent operation. The format adapter resolves
/// stable document targets against the immutable, digest-checked source first.
#[derive(Debug, Clone)]
pub struct AttributeEdit {
    /// Physical Start-event ordinal in the supplied source, including MCE branches.
    pub element_ordinal: usize,
    pub expected_element: ExpandedName,
    pub attribute: ExpandedName,
    /// XML-normalized old value; None asserts absence, not an unspecified check.
    pub expected_value: Option<String>,
    /// None removes the attribute. Some("") sets an explicitly empty value.
    pub value: Option<String>,
    /// Required only when inserting a namespaced attribute. Must already resolve
    /// to `attribute` in this element's scope. No namespace declarations are added.
    /// Existing attributes retain their original prefix and quote character.
    pub insertion_name: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct AttributeRewriteLimits {
    pub xml: XmlLimits,
    pub max_edits: usize,
    pub max_edit_bytes: usize,
}
impl Default for AttributeRewriteLimits {
    fn default() -> Self {
        Self {
            xml: XmlLimits::default(),
            max_edits: 10_000,
            max_edit_bytes: 32 * 1024 * 1024,
        }
    }
}
fn conflict(message: &str) -> XmlError {
    XmlError::EditConflict(message.into())
}
fn space(v: u8) -> bool {
    matches!(v, b' ' | b'\t' | b'\r' | b'\n')
}
fn advance(index: &mut usize, check: &dyn Fn() -> bool) -> Result<(), XmlError> {
    *index += 1;
    if index.is_multiple_of(4096) && check() {
        return Err(XmlError::Cancelled);
    }
    Ok(())
}

struct Lexical<'a> {
    name: &'a str,
    whole: Range<usize>,
    value: Range<usize>,
    quote: u8,
}

/// Called only on a start tag that the shared scanner has already validated.
/// The offsets are decoded UTF-8 offsets, never client-provided source spans.
fn lexical<'a>(
    text: &'a str,
    span: Range<usize>,
    check: &dyn Fn() -> bool,
) -> Result<(Vec<Lexical<'a>>, usize), XmlError> {
    let b = text.as_bytes();
    let mut i = span.start + 1;
    while i < span.end && !space(b[i]) && !matches!(b[i], b'/' | b'>') {
        advance(&mut i, check)?;
    }
    let mut attributes = Vec::new();
    loop {
        while i < span.end && space(b[i]) {
            advance(&mut i, check)?;
        }
        if i >= span.end {
            return Err(conflict("validated start tag boundary"));
        }
        if matches!(b[i], b'/' | b'>') {
            return Ok((attributes, i));
        }
        let start = i;
        while i < span.end && !space(b[i]) && b[i] != b'=' {
            advance(&mut i, check)?;
        }
        let end = i;
        while i < span.end && space(b[i]) {
            advance(&mut i, check)?;
        }
        if b.get(i) != Some(&b'=') {
            return Err(conflict("validated attribute separator"));
        }
        advance(&mut i, check)?;
        while i < span.end && space(b[i]) {
            advance(&mut i, check)?;
        }
        let quote = *b
            .get(i)
            .ok_or_else(|| conflict("validated attribute quote"))?;
        if !matches!(quote, b'\'' | b'"') {
            return Err(conflict("validated attribute quote"));
        }
        advance(&mut i, check)?;
        let value_start = i;
        while i < span.end && b[i] != quote {
            advance(&mut i, check)?;
        }
        if i >= span.end {
            return Err(conflict("validated attribute end"));
        }
        attributes.push(Lexical {
            name: &text[start..end],
            whole: start..i + 1,
            value: value_start..i,
            quote,
        });
        advance(&mut i, check)?;
    }
}
fn escape(
    value: &str,
    quote: u8,
    max: usize,
    check: &dyn Fn() -> bool,
) -> Result<String, XmlError> {
    let mut result = String::new();
    for (i, c) in value.chars().enumerate() {
        if i % 4096 == 0 && check() {
            return Err(XmlError::Cancelled);
        }
        if !names::valid_char(c) {
            return Err(conflict("replacement contains a non-XML character"));
        }
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '\r' => result.push_str("&#xD;"),
            '\n' => result.push_str("&#xA;"),
            '\t' => result.push_str("&#x9;"),
            '\'' if quote == b'\'' => result.push_str("&apos;"),
            '"' if quote == b'"' => result.push_str("&quot;"),
            other => result.push(other),
        }
        if result.len() > max {
            return Err(XmlError::Limit("escaped attribute bytes"));
        }
    }
    Ok(result)
}

/// Validate every precondition, preserve all bytes outside the selected spans,
/// then re-read and verify the actual returned attributes before returning.
/// Namespace declarations are deliberately outside this operation's domain:
/// changing one can retarget arbitrary descendant elements and QName values.
pub fn rewrite_attributes(
    input: &[u8],
    changes: &[AttributeEdit],
    limits: AttributeRewriteLimits,
    check: &dyn Fn() -> bool,
) -> Result<Vec<u8>, XmlError> {
    if check() {
        return Err(XmlError::Cancelled);
    }
    if changes.len() > limits.max_edits {
        return Err(XmlError::Limit("attribute edit count"));
    }
    let mut by_element: BTreeMap<usize, BTreeMap<&ExpandedName, &AttributeEdit>> = BTreeMap::new();
    let mut edit_bytes = 0usize;
    for edit in changes {
        if check() {
            return Err(XmlError::Cancelled);
        }
        for s in [
            Some(&edit.expected_element.namespace),
            Some(&edit.expected_element.local),
            Some(&edit.attribute.namespace),
            Some(&edit.attribute.local),
            edit.expected_value.as_ref(),
            edit.value.as_ref(),
            edit.insertion_name.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            edit_bytes = edit_bytes
                .checked_add(s.len())
                .ok_or(XmlError::Limit("attribute edit bytes"))?;
        }
        if edit_bytes > limits.max_edit_bytes {
            return Err(XmlError::Limit("attribute edit bytes"));
        }
        if !names::ncname(&edit.attribute.local)
            || !names::ncname(&edit.expected_element.local)
            || edit.attribute.namespace == names::XMLNS_NS
            || edit.attribute.is("", "xmlns")
        {
            return Err(conflict(
                "invalid attribute edit name or namespace declaration",
            ));
        }
        if edit.insertion_name.is_some() && (edit.expected_value.is_some() || edit.value.is_none())
        {
            return Err(conflict("insertion name requires insertion"));
        }
        if by_element
            .entry(edit.element_ordinal)
            .or_default()
            .insert(&edit.attribute, edit)
            .is_some()
        {
            return Err(conflict("duplicate expanded attribute edit"));
        }
    }
    let (text, encoding) = encoding::decode(input, limits.xml.max_bytes, check)?;
    let mut ordinal = 0usize;
    let mut matched = 0usize;
    let mut patches = Vec::new();
    let mut patch_bytes = 0usize;
    scan::scan_decoded(&text, encoding, limits.xml, check, |event| {
        if let XmlEvent::Start {
            element,
            namespaces,
            span,
            ..
        } = event
        {
            if let Some(edits) = by_element.get(&ordinal) {
                let (raw, at) = lexical(&text, span, check)?;
                let mut inserted = String::new();
                for edit in edits.values() {
                    if check() {
                        return Err(XmlError::Cancelled);
                    }
                    if element.name != edit.expected_element {
                        return Err(conflict("element expanded name changed"));
                    }
                    let old = element.attributes.iter().find(|a| a.name == edit.attribute);
                    if old.map(|a| a.value.as_str()) != edit.expected_value.as_deref() {
                        return Err(conflict("attribute precondition changed"));
                    }
                    matched += 1;
                    if edit.expected_value == edit.value {
                        continue;
                    }
                    match (old, &edit.value) {
                        (Some(a), value) => {
                            let token = raw
                                .iter()
                                .find(|r| r.name == a.qualified_name)
                                .ok_or_else(|| conflict("attribute lexical binding missing"))?;
                            let patch = match value {
                                Some(v) => Patch {
                                    span: token.value.clone(),
                                    text: escape(
                                        v,
                                        token.quote,
                                        limits
                                            .xml
                                            .max_bytes
                                            .saturating_sub(patch_bytes)
                                            .saturating_sub(inserted.len()),
                                        check,
                                    )?,
                                },
                                None => Patch {
                                    span: token.whole.clone(),
                                    text: String::new(),
                                },
                            };
                            patch_bytes += patch.text.len();
                            patches.push(patch);
                        }
                        (None, Some(value)) => {
                            let name = edit
                                .insertion_name
                                .as_deref()
                                .unwrap_or(&edit.attribute.local);
                            if name == "xmlns"
                                || name.starts_with("xmlns:")
                                || names::resolve(name, true, namespaces)? != edit.attribute
                            {
                                return Err(conflict(
                                    "insertion QName does not resolve to attribute",
                                ));
                            }
                            let overhead = name
                                .len()
                                .checked_add(4)
                                .ok_or(XmlError::Limit("inserted attribute bytes"))?;
                            let used = patch_bytes
                                .checked_add(inserted.len())
                                .and_then(|n| n.checked_add(overhead))
                                .ok_or(XmlError::Limit("inserted attribute bytes"))?;
                            if used > limits.xml.max_bytes {
                                return Err(XmlError::Limit("inserted attribute bytes"));
                            }
                            inserted.push(' ');
                            inserted.push_str(name);
                            inserted.push_str("=\"");
                            inserted.push_str(&escape(
                                value,
                                b'"',
                                limits.xml.max_bytes - used,
                                check,
                            )?);
                            inserted.push('"');
                            if inserted.len() > limits.xml.max_bytes {
                                return Err(XmlError::Limit("inserted attribute bytes"));
                            }
                        }
                        (None, None) => unreachable!("unchanged absence handled above"),
                    }
                }
                if !inserted.is_empty() {
                    patch_bytes += inserted.len();
                    patches.push(Patch {
                        span: at..at,
                        text: inserted,
                    });
                }
            }
            ordinal += 1;
        }
        Ok(())
    })?;
    if matched != changes.len() {
        return Err(conflict("selected attribute element does not exist"));
    }
    let result = apply_patches(input, &text, encoding, patches, limits.xml.max_bytes, check)?;
    ordinal = 0;
    matched = 0;
    crate::scan_with_control(&result, limits.xml, check, |event| {
        if let XmlEvent::Start { element, .. } = event {
            if let Some(edits) = by_element.get(&ordinal) {
                for edit in edits.values() {
                    if element.name != edit.expected_element
                        || element
                            .attributes
                            .iter()
                            .find(|a| a.name == edit.attribute)
                            .map(|a| a.value.as_str())
                            != edit.value.as_deref()
                    {
                        return Err(conflict("candidate attributes differ from intended edits"));
                    }
                    matched += 1;
                }
            }
            ordinal += 1;
        }
        Ok(())
    })?;
    if matched != changes.len() {
        return Err(conflict("candidate target element missing"));
    }
    Ok(result)
}
