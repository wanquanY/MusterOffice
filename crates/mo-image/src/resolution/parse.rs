//! A bounded metadata pass after successful full container/pixel decoding.
//! It neither decompresses pixels nor follows thumbnail/EXIF/GPS IFD chains.
use super::{Density, ResolutionDeclaration, ResolutionSource, ResolutionUnit};
use crate::{ImageError, ImageFormat, cancel};

fn invalid() -> ImageError {
    ImageError::Invalid("image resolution metadata")
}
fn span(bytes: &[u8], start: usize, len: usize) -> Result<&[u8], ImageError> {
    let end = start.checked_add(len).ok_or_else(invalid)?;
    bytes.get(start..end).ok_or_else(invalid)
}
fn be16(b: &[u8]) -> u16 {
    u16::from_be_bytes([b[0], b[1]])
}
fn be32(b: &[u8]) -> u32 {
    u32::from_be_bytes([b[0], b[1], b[2], b[3]])
}
fn integer(numerator: u32) -> Option<Density> {
    Some(Density {
        numerator,
        denominator: 1,
    })
}
fn exif(
    bytes: &[u8],
    source_offset: u32,
    check: &dyn Fn() -> bool,
) -> Result<ResolutionDeclaration, ImageError> {
    let header = span(bytes, 0, 8)?;
    let little = match &header[..2] {
        b"II" => true,
        b"MM" => false,
        _ => return Err(invalid()),
    };
    let u16 = |b: &[u8]| {
        if little {
            u16::from_le_bytes([b[0], b[1]])
        } else {
            be16(b)
        }
    };
    let u32 = |b: &[u8]| {
        if little {
            u32::from_le_bytes([b[0], b[1], b[2], b[3]])
        } else {
            be32(b)
        }
    };
    if u16(&header[2..]) != 42 {
        return Err(invalid());
    }
    let offset = u32(&header[4..]) as usize;
    if offset < 8 {
        return Err(invalid());
    }
    let count = usize::from(u16(span(bytes, offset, 2)?));
    let table = span(bytes, offset + 2, count * 12 + 4)?;
    let mut result = ResolutionDeclaration {
        source: ResolutionSource::ExifIfd0,
        source_offset,
        x: None,
        y: None,
        unit: None,
    };
    for entry in table[..count * 12].chunks_exact(12) {
        cancel(check)?;
        let tag = u16(entry);
        if !matches!(tag, 0x11a | 0x11b | 0x128) {
            continue;
        }
        if u32(&entry[4..]) != 1 {
            return Err(invalid());
        }
        if tag == 0x128 {
            if result.unit.is_some() || u16(&entry[2..]) != 3 {
                return Err(invalid());
            }
            result.unit = Some(match u16(&entry[8..]) {
                2 => ResolutionUnit::Inch,
                3 => ResolutionUnit::Centimetre,
                _ => return Err(invalid()),
            });
        } else {
            let axis = if tag == 0x11a {
                &mut result.x
            } else {
                &mut result.y
            };
            if axis.is_some() || u16(&entry[2..]) != 5 {
                return Err(invalid());
            }
            let value = span(bytes, u32(&entry[8..]) as usize, 8)?;
            let density = Density {
                numerator: u32(value),
                denominator: u32(&value[4..]),
            };
            if density.denominator == 0 {
                return Err(invalid());
            }
            *axis = Some(density);
        }
    }
    Ok(result)
}
fn png(bytes: &[u8], check: &dyn Fn() -> bool) -> Result<Vec<ResolutionDeclaration>, ImageError> {
    let mut result = Vec::new();
    let (mut offset, mut count, mut physical, mut exif_seen, mut data) =
        (8, 0, false, false, false);
    loop {
        cancel(check)?;
        count += 1;
        if count > 4096 {
            return Err(ImageError::Limit("image metadata chunks"));
        }
        let header = span(bytes, offset, 8)?;
        let length = be32(header) as usize;
        // CRC and complete pixel semantics are the decoder's responsibility.
        let body = span(
            bytes,
            offset + 8,
            length.checked_add(4).ok_or_else(invalid)?,
        )?;
        let body = &body[..length];
        match &header[4..] {
            b"pHYs" => {
                if physical || data || length != 9 {
                    return Err(invalid());
                }
                physical = true;
                result.push(ResolutionDeclaration {
                    source: ResolutionSource::PngPhysical,
                    source_offset: offset as u32,
                    x: integer(be32(body)),
                    y: integer(be32(&body[4..])),
                    unit: Some(match body[8] {
                        0 => ResolutionUnit::AspectRatio,
                        1 => ResolutionUnit::Metre,
                        _ => return Err(invalid()),
                    }),
                });
            }
            b"eXIf" => {
                if exif_seen {
                    return Err(invalid());
                }
                exif_seen = true;
                result.push(exif(body, offset as u32, check)?);
            }
            b"IDAT" => data = true,
            b"IEND" => {
                if length != 0 || offset + 12 != bytes.len() {
                    return Err(invalid());
                }
                return Ok(result);
            }
            _ => (),
        }
        offset += length + 12;
    }
}
fn jpeg(bytes: &[u8], check: &dyn Fn() -> bool) -> Result<Vec<ResolutionDeclaration>, ImageError> {
    let (mut offset, mut count, mut entropy, mut jfif_seen, mut exif_seen) =
        (2, 0, false, false, false);
    let mut result = Vec::new();
    loop {
        cancel(check)?;
        if entropy {
            // Check cancellation at bounded intervals even inside a long scan.
            while offset < bytes.len() && bytes[offset] != 0xff {
                let n = (bytes.len() - offset).min(16384);
                let advance = bytes[offset..offset + n]
                    .iter()
                    .position(|b| *b == 0xff)
                    .unwrap_or(n);
                offset += advance;
                cancel(check)?;
            }
        }
        let source_offset = offset;
        if span(bytes, offset, 1)?[0] != 0xff {
            return Err(invalid());
        }
        while offset < bytes.len() && bytes[offset] == 0xff {
            offset += 1;
            if offset % 16384 == 0 {
                cancel(check)?;
            }
        }
        let marker = span(bytes, offset, 1)?[0];
        offset += 1;
        if entropy && (marker == 0 || (0xd0..=0xd7).contains(&marker)) {
            continue;
        }
        entropy = false;
        count += 1;
        if count > 4096 {
            return Err(ImageError::Limit("image metadata markers"));
        }
        if marker == 0xd9 {
            return if offset == bytes.len() {
                Ok(result)
            } else {
                Err(invalid())
            };
        }
        if matches!(marker, 0 | 1 | 0xd0..=0xd8) {
            return Err(invalid());
        }
        let length = usize::from(be16(span(bytes, offset, 2)?));
        if length < 2 {
            return Err(invalid());
        }
        let body = span(bytes, offset + 2, length - 2)?;
        if marker == 0xe0 && body.starts_with(b"JFIF\0") {
            if jfif_seen || body.len() < 14 {
                return Err(invalid());
            }
            jfif_seen = true;
            if body.len() != 14 + 3 * usize::from(body[12]) * usize::from(body[13]) {
                return Err(invalid());
            }
            result.push(ResolutionDeclaration {
                source: ResolutionSource::Jfif,
                source_offset: source_offset as u32,
                x: integer(u32::from(be16(&body[8..]))),
                y: integer(u32::from(be16(&body[10..]))),
                unit: Some(match body[7] {
                    0 => ResolutionUnit::AspectRatio,
                    1 => ResolutionUnit::Inch,
                    2 => ResolutionUnit::Centimetre,
                    _ => return Err(invalid()),
                }),
            });
        } else if marker == 0xe1 && body.starts_with(b"Exif\0\0") {
            if exif_seen {
                return Err(invalid());
            }
            exif_seen = true;
            result.push(exif(&body[6..], source_offset as u32, check)?);
        } else if marker == 0xda {
            entropy = true;
        }
        offset += length;
    }
}
pub(super) fn read(
    bytes: &[u8],
    format: ImageFormat,
    check: &dyn Fn() -> bool,
) -> Result<Vec<ResolutionDeclaration>, ImageError> {
    match format {
        ImageFormat::Png => png(bytes, check),
        ImageFormat::Jpeg => jpeg(bytes, check),
    }
}
