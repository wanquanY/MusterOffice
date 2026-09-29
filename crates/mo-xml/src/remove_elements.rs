//! Explicit element removal without reserializing unaffected source bytes.
use crate::{
    ExpandedName, XmlError, XmlEvent, XmlLimits,
    edit_bytes::{Patch, apply_patches},
    encoding, scan,
};
use std::collections::BTreeMap;

pub struct ElementRemoval {
    pub element_ordinal: usize,
    pub expected_name: ExpandedName,
}

/// Ordinals address the original document. Missing, duplicate, overlapping or
/// root selections fail atomically. Namespace spelling, comments and encoding
/// outside selected subtrees remain byte-identical.
pub fn remove_elements(
    input: &[u8],
    removals: &[ElementRemoval],
    limits: XmlLimits,
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<u8>, XmlError> {
    if removals.len() > limits.max_elements {
        return Err(XmlError::Limit("element removal count"));
    }
    let mut remaining = BTreeMap::new();
    for removal in removals {
        if removal.element_ordinal == 0
            || remaining
                .insert(removal.element_ordinal, &removal.expected_name)
                .is_some()
        {
            return Err(conflict("duplicate or root element removal"));
        }
    }
    let (decoded, encoding) = encoding::decode(input, limits.max_bytes, cancelled)?;
    let mut ordinal = 0;
    let mut selected = None;
    let mut patches = Vec::new();
    scan::scan_decoded(&decoded, encoding, limits, cancelled, |event| {
        match event {
            XmlEvent::Start {
                element,
                depth,
                span,
                ..
            } => {
                if let Some(expected) = remaining.remove(&ordinal) {
                    if selected.is_some() || &element.name != expected {
                        return Err(conflict("overlapping or changed element removal"));
                    }
                    selected = Some((depth, span.start));
                }
                ordinal += 1;
            }
            XmlEvent::End { depth, span, .. } => {
                if let Some((selected_depth, start)) = selected
                    && depth == selected_depth
                {
                    patches.push(Patch {
                        span: start..span.end,
                        text: String::new(),
                    });
                    selected = None;
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    if !remaining.is_empty() {
        return Err(conflict("removed element is absent"));
    }
    let output = apply_patches(
        input,
        &decoded,
        encoding,
        patches,
        limits.max_bytes,
        cancelled,
    )?;
    crate::scan_with_control(&output, limits, cancelled, |_| Ok(()))?;
    Ok(output)
}

fn conflict(message: &str) -> XmlError {
    XmlError::EditConflict(message.into())
}
