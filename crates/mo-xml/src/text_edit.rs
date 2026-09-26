//! Syntax-aware leaf-text editing. Untouched source bytes, including unknown
//! attributes/elements, comments, PIs, namespace spelling and encoding, are retained.
use crate::edit_bytes::{Patch, apply_patches};
use crate::{ExpandedName, XmlError, XmlEvent, XmlLimits, encoding, names, scan};
use std::{collections::BTreeMap, ops::Range};

/// Internal source binding, not a public agent operation. Ordinals are zero-based
/// Start-event indices in the immutable source supplied to `rewrite_text`.
#[derive(Debug, Clone)]
pub struct TextReplacement {
    pub element_ordinal: usize,
    pub expected_name: ExpandedName,
    /// XML-normalized text, as observed by the scanner; no Unicode normalization.
    pub expected_text: String,
    pub replacement: String,
}

#[derive(Debug, Clone, Copy)]
pub struct TextRewriteLimits {
    pub xml: XmlLimits,
    pub max_replacements: usize,
    pub max_text_spans: usize,
}
impl Default for TextRewriteLimits {
    fn default() -> Self {
        Self {
            xml: XmlLimits::default(),
            max_replacements: 10_000,
            max_text_spans: 100_000,
        }
    }
}

struct Capture {
    ordinal: usize,
    depth: usize,
    opening: Range<usize>,
    qualified_name: String,
    value: String,
    spans: Vec<Range<usize>>,
}

/// All preconditions and actual resulting XML are validated before returning bytes.
/// A selected element containing a child element is rejected, never flattened.
pub fn rewrite_text(
    input: &[u8],
    replacements: &[TextReplacement],
    limits: TextRewriteLimits,
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<u8>, XmlError> {
    if replacements.len() > limits.max_replacements {
        return Err(XmlError::Limit("text replacement count"));
    }
    let mut edits = BTreeMap::new();
    let mut replacement_bytes = 0_usize;
    for edit in replacements {
        if cancelled() {
            return Err(XmlError::Cancelled);
        }
        if edits.insert(edit.element_ordinal, edit).is_some() {
            return Err(conflict("duplicate element text replacement"));
        }
        replacement_bytes = replacement_bytes
            .checked_add(edit.replacement.len())
            .and_then(|v| v.checked_add(edit.expected_text.len()))
            .ok_or(XmlError::Limit("text replacement bytes"))?;
        if replacement_bytes > limits.xml.max_bytes {
            return Err(XmlError::Limit("text replacement bytes"));
        }
        for (index, character) in edit.replacement.chars().enumerate() {
            if index % 4096 == 0 && cancelled() {
                return Err(XmlError::Cancelled);
            }
            if !names::valid_char(character) {
                return Err(conflict("replacement contains a non-XML character"));
            }
        }
    }
    let (text, encoding) = encoding::decode(input, limits.xml.max_bytes, cancelled)?;
    let mut ordinal = 0;
    let mut matched = 0;
    let mut span_count = 0;
    let mut active: Option<Capture> = None;
    let mut patches = Vec::new();
    scan::scan_decoded(&text, encoding, limits.xml, cancelled, |event| {
        match event {
            XmlEvent::Start {
                element,
                depth,
                span,
                ..
            } => {
                if active.is_some() {
                    return Err(conflict("selected text element contains a child element"));
                }
                if let Some(edit) = edits.get(&ordinal) {
                    if element.name != edit.expected_name {
                        return Err(conflict("element expanded name changed"));
                    }
                    active = Some(Capture {
                        ordinal,
                        depth,
                        opening: span,
                        qualified_name: element.qualified_name.clone(),
                        value: String::new(),
                        spans: Vec::new(),
                    });
                }
                ordinal += 1;
            }
            XmlEvent::Text { text, span, .. } => {
                if let Some(capture) = &mut active {
                    if span_count >= limits.max_text_spans {
                        return Err(XmlError::Limit("edited text spans"));
                    }
                    span_count += 1;
                    capture.value.push_str(text);
                    capture.spans.push(span);
                }
            }
            XmlEvent::End { depth, span, .. } => {
                if active.as_ref().is_some_and(|c| c.depth == depth) {
                    let capture = active.take().expect("checked active capture");
                    let edit = edits[&capture.ordinal];
                    if capture.value != edit.expected_text {
                        return Err(conflict("expected text changed"));
                    }
                    matched += 1;
                    if edit.replacement == capture.value {
                        return Ok(());
                    }
                    let escaped = escape_text(&edit.replacement, limits.xml.max_bytes, cancelled)?;
                    if capture.opening == span {
                        patches.push(Patch {
                            span: span.end - 2..span.end,
                            text: format!(">{escaped}</{}>", capture.qualified_name),
                        });
                    } else if capture.spans.is_empty() {
                        patches.push(Patch {
                            span: capture.opening.end..capture.opening.end,
                            text: escaped,
                        });
                    } else {
                        let mut spans = capture.spans.into_iter();
                        patches.push(Patch {
                            span: spans.next().expect("nonempty spans"),
                            text: escaped,
                        });
                        patches.extend(spans.map(|span| Patch {
                            span,
                            text: String::new(),
                        }));
                    }
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    if matched != edits.len() {
        return Err(conflict("selected element does not exist"));
    }
    let output = apply_patches(
        input,
        &text,
        encoding,
        patches,
        limits.xml.max_bytes,
        cancelled,
    )?;
    // Re-read the actual returned representation, including UTF-16 and declarations.
    crate::scan_with_control(&output, limits.xml, cancelled, |_| Ok(()))?;
    Ok(output)
}

fn conflict(message: &str) -> XmlError {
    XmlError::EditConflict(message.into())
}

fn escape_text(value: &str, max: usize, cancelled: &dyn Fn() -> bool) -> Result<String, XmlError> {
    let mut result = String::new();
    for (index, character) in value.chars().enumerate() {
        if index % 4096 == 0 && cancelled() {
            return Err(XmlError::Cancelled);
        }
        match character {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '\r' => result.push_str("&#xD;"),
            other => result.push(other),
        }
        if result.len() > max {
            return Err(XmlError::Limit("escaped replacement bytes"));
        }
    }
    Ok(result)
}
