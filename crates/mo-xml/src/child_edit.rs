//! Append one independently namespace-valid element without reserializing its
//! parent. Source comments, prefixes, attributes and encoding remain intact.
use crate::{
    ExpandedName, XmlError, XmlEvent, XmlLimits,
    edit_bytes::{Patch, apply_patches},
    encoding, scan,
};

pub fn append_child(
    input: &[u8],
    parent_ordinal: usize,
    expected_parent: &ExpandedName,
    child: &str,
    limits: XmlLimits,
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<u8>, XmlError> {
    // A standalone element cannot rely on ambient namespace bindings. XML
    // declarations/prolog instructions have no place inside an element.
    let mut child_names = Vec::new();
    crate::scan_with_control(child.as_bytes(), limits, cancelled, |event| {
        if let XmlEvent::Start { element, .. } = &event {
            child_names.push(element.name.clone());
        }
        if matches!(event, XmlEvent::ProcessingInstruction { .. }) {
            return Err(conflict("child contains a processing instruction"));
        }
        Ok(())
    })?;
    if child.trim_start().starts_with("<?xml") {
        return Err(conflict("child contains an XML declaration"));
    }
    let (decoded, encoding) = encoding::decode(input, limits.max_bytes, cancelled)?;
    let mut ordinal = 0;
    let mut selected = None;
    let mut patch = None;
    let mut insertion_ordinal = None;
    scan::scan_decoded(&decoded, encoding, limits, cancelled, |event| {
        match event {
            XmlEvent::Start {
                element,
                depth,
                span,
                ..
            } => {
                if ordinal == parent_ordinal {
                    if &element.name != expected_parent {
                        return Err(conflict("parent expanded name changed"));
                    }
                    selected = Some((depth, span, element.qualified_name.clone()));
                }
                ordinal += 1;
            }
            XmlEvent::End { depth, span, .. } => {
                if let Some((parent_depth, opening, name)) = &selected
                    && depth == *parent_depth
                {
                    patch = Some(if *opening == span {
                        Patch {
                            span: span.end - 2..span.end,
                            text: format!(">{child}</{name}>"),
                        }
                    } else {
                        Patch {
                            span: span.start..span.start,
                            text: child.into(),
                        }
                    });
                    insertion_ordinal = Some(ordinal);
                    selected = None;
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    let result = apply_patches(
        input,
        &decoded,
        encoding,
        vec![patch.ok_or_else(|| conflict("parent element is absent"))?],
        limits.max_bytes,
        cancelled,
    )?;
    let start = insertion_ordinal.expect("selected parent has a closing span");
    let mut ordinal = 0;
    crate::scan_with_control(&result, limits, cancelled, |event| {
        if let XmlEvent::Start { element, .. } = event {
            if ordinal >= start
                && ordinal - start < child_names.len()
                && element.name != child_names[ordinal - start]
            {
                return Err(conflict(
                    "parent namespace changes inserted child semantics",
                ));
            }
            ordinal += 1;
        }
        Ok(())
    })?;
    Ok(result)
}
fn conflict(message: &str) -> XmlError {
    XmlError::EditConflict(message.into())
}
