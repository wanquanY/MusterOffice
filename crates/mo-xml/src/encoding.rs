use crate::XmlError;
use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XmlEncoding {
    Utf8,
    Utf16Le,
    Utf16Be,
}

pub(crate) fn decode<'a>(
    input: &'a [u8],
    max_bytes: usize,
    cancelled: &dyn Fn() -> bool,
) -> Result<(Cow<'a, str>, XmlEncoding), XmlError> {
    if cancelled() {
        return Err(XmlError::Cancelled);
    }
    if input.len() > max_bytes {
        return Err(XmlError::Limit("input bytes"));
    }
    let (data, encoding) = if input.starts_with(&[0xFF, 0xFE]) {
        (&input[2..], XmlEncoding::Utf16Le)
    } else if input.starts_with(&[0xFE, 0xFF]) {
        (&input[2..], XmlEncoding::Utf16Be)
    } else if input.starts_with(&[b'<', 0, b'?', 0]) {
        (input, XmlEncoding::Utf16Le)
    } else if input.starts_with(&[0, b'<', 0, b'?']) {
        (input, XmlEncoding::Utf16Be)
    } else {
        (
            input.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(input),
            XmlEncoding::Utf8,
        )
    };
    if encoding == XmlEncoding::Utf8 {
        return Ok((
            Cow::Borrowed(
                std::str::from_utf8(data)
                    .map_err(|_| XmlError::Encoding("invalid UTF-8".into()))?,
            ),
            encoding,
        ));
    }
    if data.len() % 2 != 0 {
        return Err(XmlError::Encoding("odd UTF-16 byte length".into()));
    }
    let units = data.chunks_exact(2).map(|p| {
        if encoding == XmlEncoding::Utf16Le {
            u16::from_le_bytes([p[0], p[1]])
        } else {
            u16::from_be_bytes([p[0], p[1]])
        }
    });
    let mut text = String::new();
    for (index, decoded) in char::decode_utf16(units).enumerate() {
        if index % 4096 == 0 && cancelled() {
            return Err(XmlError::Cancelled);
        }
        let character =
            decoded.map_err(|_| XmlError::Encoding("unpaired UTF-16 surrogate".into()))?;
        if character.len_utf8() > max_bytes.saturating_sub(text.len()) {
            return Err(XmlError::Limit("decoded bytes"));
        }
        text.push(character);
    }
    Ok((Cow::Owned(text), encoding))
}

pub(crate) fn check_declaration(encoding: XmlEncoding, declared: &str) -> Result<(), XmlError> {
    let declared = declared.to_ascii_lowercase();
    let compatible = match encoding {
        XmlEncoding::Utf8 => declared == "utf-8",
        XmlEncoding::Utf16Le => declared == "utf-16" || declared == "utf-16le",
        XmlEncoding::Utf16Be => declared == "utf-16" || declared == "utf-16be",
    };
    if compatible {
        Ok(())
    } else {
        Err(XmlError::Encoding(format!(
            "unsupported or contradictory encoding declaration: {declared}"
        )))
    }
}
