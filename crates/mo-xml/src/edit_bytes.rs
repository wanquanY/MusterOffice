//! Shared encoding-preserving source splices for typed XML editors.
use crate::{XmlEncoding, XmlError};
use std::ops::Range;

pub(crate) struct Patch {
    pub span: Range<usize>,
    pub text: String,
}
fn conflict(message: &str) -> XmlError {
    XmlError::EditConflict(message.into())
}

fn encoded_len(text: &str, encoding: XmlEncoding) -> usize {
    match encoding {
        XmlEncoding::Utf8 => text.len(),
        _ => text.encode_utf16().count() * 2,
    }
}

fn append_encoded(
    output: &mut Vec<u8>,
    text: &str,
    encoding: XmlEncoding,
    cancelled: &dyn Fn() -> bool,
) -> Result<(), XmlError> {
    if encoding == XmlEncoding::Utf8 {
        output.extend_from_slice(text.as_bytes());
    } else {
        for (index, unit) in text.encode_utf16().enumerate() {
            if index % 4096 == 0 && cancelled() {
                return Err(XmlError::Cancelled);
            }
            output.extend_from_slice(&if encoding == XmlEncoding::Utf16Le {
                unit.to_le_bytes()
            } else {
                unit.to_be_bytes()
            });
        }
    }
    Ok(())
}

pub(crate) fn apply_patches(
    input: &[u8],
    text: &str,
    encoding: XmlEncoding,
    mut patches: Vec<Patch>,
    max: usize,
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<u8>, XmlError> {
    patches.sort_by_key(|patch| patch.span.start);
    let bom = match encoding {
        XmlEncoding::Utf8 if input.starts_with(&[0xEF, 0xBB, 0xBF]) => 3,
        XmlEncoding::Utf16Le if input.starts_with(&[0xFF, 0xFE]) => 2,
        XmlEncoding::Utf16Be if input.starts_with(&[0xFE, 0xFF]) => 2,
        _ => 0,
    };
    let mut decoded_from = 0;
    let mut raw_from = bom;
    let mut output_length = input.len();
    for patch in &mut patches {
        if cancelled() {
            return Err(XmlError::Cancelled);
        }
        if patch.span.start < decoded_from {
            return Err(conflict("overlapping XML edits"));
        }
        let raw_start = raw_from + encoded_len(&text[decoded_from..patch.span.start], encoding);
        let raw_end = raw_start + encoded_len(&text[patch.span.clone()], encoding);
        output_length = output_length
            .checked_sub(raw_end - raw_start)
            .and_then(|v| v.checked_add(encoded_len(&patch.text, encoding)))
            .ok_or(XmlError::Limit("edited XML bytes"))?;
        decoded_from = patch.span.end;
        raw_from = raw_end;
        patch.span = raw_start..raw_end;
    }
    if output_length > max {
        return Err(XmlError::Limit("edited XML bytes"));
    }
    let mut result = Vec::with_capacity(output_length);
    let mut from = 0;
    for patch in patches {
        if cancelled() {
            return Err(XmlError::Cancelled);
        }
        result.extend_from_slice(&input[from..patch.span.start]);
        append_encoded(&mut result, &patch.text, encoding, cancelled)?;
        from = patch.span.end;
    }
    result.extend_from_slice(&input[from..]);
    Ok(result)
}
