use super::*;
use read_fonts::tables::name::{Encoding, MacRomanMapping};
use std::collections::BTreeSet;

pub(super) fn os2(font: &FontRef<'_>) -> Result<Option<Os2Metadata>, FontError> {
    if font.table_data(Tag::new(b"OS/2")).is_none() {
        return Ok(None);
    }
    let t = font.os2()?;
    Ok(Some(Os2Metadata {
        version: t.version(),
        weight_class: t.us_weight_class(),
        width_class: t.us_width_class(),
        fs_type: t.fs_type(),
        fs_selection: t.fs_selection().bits(),
        typo_ascender: t.s_typo_ascender(),
        typo_descender: t.s_typo_descender(),
        typo_line_gap: t.s_typo_line_gap(),
        win_ascent: t.us_win_ascent(),
        win_descent: t.us_win_descent(),
    }))
}
fn utf16(data: &[u8]) -> Result<String, FontError> {
    if !data.len().is_multiple_of(2) {
        return Err(invalid("odd UTF-16 name length"));
    }
    char::decode_utf16(
        data.chunks_exact(2)
            .map(|v| u16::from_be_bytes([v[0], v[1]])),
    )
    .collect::<Result<String, _>>()
    .map_err(|_| invalid("malformed UTF-16 font name"))
}
fn string_bytes<'a>(
    data: &'a [u8],
    offset: usize,
    len: usize,
    budget: &mut usize,
    limit: usize,
) -> Result<&'a [u8], FontError> {
    *budget = budget
        .checked_add(len)
        .ok_or(FontError::Limit("name bytes"))?;
    if *budget > limit {
        return Err(FontError::Limit("name bytes"));
    }
    data.get(offset..offset + len)
        .ok_or_else(|| invalid("name string range"))
}
pub(super) fn names(
    font: &FontRef<'_>,
    limits: FontLimits,
    check: &dyn Fn() -> bool,
) -> Result<(Vec<FontName>, Vec<String>), FontError> {
    let table = font.name()?;
    if table.version() > 1 {
        return Err(FontError::Unsupported("name table version"));
    }
    let records = table.name_record();
    let tags = table.lang_tag_record().unwrap_or_default();
    if records.len() + tags.len() > limits.max_names {
        return Err(FontError::Limit("font names"));
    }
    let directory_end = 6
        + records.len() * 12
        + if table.version() == 1 {
            2 + tags.len() * 4
        } else {
            0
        };
    if (table.storage_offset() as usize) < directory_end {
        return Err(invalid("name storage overlaps records"));
    }
    let raw = font
        .table_data(Tag::new(b"name"))
        .ok_or_else(|| invalid("name table"))?;
    let data = raw
        .as_bytes()
        .get(table.storage_offset() as usize..)
        .ok_or_else(|| invalid("name storage range"))?;
    let mut budget = 0_usize;
    let mut decoded = 0_usize;
    let mut language_tags = Vec::new();
    for tag in tags {
        cancelled(check)?;
        let value = utf16(string_bytes(
            data,
            tag.lang_tag_offset().to_u32() as usize,
            tag.length() as usize,
            &mut budget,
            limits.max_name_bytes,
        )?)?;
        decoded += value.len();
        if decoded > limits.max_name_bytes {
            return Err(FontError::Limit("decoded name bytes"));
        }
        language_tags.push(value);
    }
    let mut names = Vec::with_capacity(records.len());
    for record in records {
        cancelled(check)?;
        if record.language_id() >= 0x8000
            && (record.language_id() - 0x8000) as usize >= language_tags.len()
        {
            return Err(invalid("font name language tag reference"));
        }
        let bytes = string_bytes(
            data,
            record.string_offset().to_u32() as usize,
            record.length() as usize,
            &mut budget,
            limits.max_name_bytes,
        )?;
        let text = match Encoding::new(record.platform_id(), record.encoding_id()) {
            Encoding::Utf16Be => Some(utf16(bytes)?),
            Encoding::MacRoman => Some(bytes.iter().map(|v| MacRomanMapping.decode(*v)).collect()),
            _ => None,
        };
        decoded += text.as_ref().map_or(0, String::len);
        if decoded > limits.max_name_bytes {
            return Err(FontError::Limit("decoded name bytes"));
        }
        names.push(FontName {
            platform_id: record.platform_id(),
            encoding_id: record.encoding_id(),
            language_id: record.language_id(),
            name_id: record.name_id().to_u16(),
            text,
        });
    }
    Ok((names, language_tags))
}
pub(super) fn variations(
    font: &FontRef<'_>,
    limits: FontLimits,
    check: &dyn Fn() -> bool,
) -> Result<(Vec<FontAxis>, Vec<FontInstance>), FontError> {
    if font.table_data(Tag::new(b"fvar")).is_none() {
        return Ok((Vec::new(), Vec::new()));
    }
    let t = font.fvar()?;
    if t.version().major != 1 || t.version().minor != 0 {
        return Err(FontError::Unsupported("fvar version"));
    }
    if t.axis_count() as usize > limits.max_axes
        || t.instance_count() as usize > limits.max_instances
    {
        return Err(FontError::Limit("variation axes or instances"));
    }
    let count = t.axis_count() as usize;
    if count == 0
        || t.axis_size() != 20
        || ![4 + count * 4, 6 + count * 4].contains(&(t.instance_size() as usize))
        || t.axis_instance_arrays_offset().to_u32() < 16
    {
        return Err(invalid("fvar record size or offset"));
    }
    let mut axes = Vec::with_capacity(count);
    let mut tags = BTreeSet::new();
    for axis in t.axes()? {
        cancelled(check)?;
        let tag = axis.axis_tag().to_be_bytes();
        if !container::valid_tag(tag)
            || !tags.insert(tag)
            || axis.min_value() > axis.default_value()
            || axis.default_value() > axis.max_value()
        {
            return Err(invalid("variation axis tag or range"));
        }
        axes.push(FontAxis {
            tag: axis.axis_tag().to_string(),
            minimum_16_16: axis.min_value().to_bits(),
            default_16_16: axis.default_value().to_bits(),
            maximum_16_16: axis.max_value().to_bits(),
            flags: axis.flags(),
            name_id: axis.axis_name_id().to_u16(),
        });
    }
    let mut instances = Vec::with_capacity(t.instance_count() as usize);
    for instance in t.instances()?.iter() {
        cancelled(check)?;
        let instance = instance?;
        let coordinates: Vec<i32> = instance
            .coordinates
            .iter()
            .map(|v| v.get().to_bits())
            .collect();
        if coordinates.len() != axes.len()
            || coordinates
                .iter()
                .zip(&axes)
                .any(|(c, a)| *c < a.minimum_16_16 || *c > a.maximum_16_16)
        {
            return Err(invalid("variation instance coordinates"));
        }
        instances.push(FontInstance {
            subfamily_name_id: instance.subfamily_name_id.to_u16(),
            postscript_name_id: instance.post_script_name_id.map(|v| v.to_u16()),
            flags: instance.flags,
            coordinates_16_16: coordinates,
        });
    }
    Ok((axes, instances))
}
