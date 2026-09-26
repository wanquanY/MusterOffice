use super::*;
use crate::source::{boolean, drawingml::enumeration, integer, malformed};
use mo_xml::{Element, XmlError};

// One declaration supplies the public typed fields and native attribute reader.
// Missing values remain None; defaulting belongs to the inheritance computation.
macro_rules! attributes {
    ($name:ident { $($field:ident : $ty:ty = $native:literal => $read:expr),* $(,)? }) => {
        #[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
        #[serde(rename_all="camelCase", deny_unknown_fields)]
        pub struct $name { $(pub $field: Option<$ty>,)* }
        impl $name {
            pub(super) const NAMES: &'static [&'static str] = &[$($native),*];
            pub(super) fn read(e: &Element) -> Result<Self, XmlError> {
                Ok(Self { $($field: attribute(e, $native).map($read).transpose()?,)* })
            }
        }
    }
}
pub(super) fn attribute<'a>(e: &'a Element, name: &str) -> Option<&'a str> {
    if let Some(local) = name.strip_prefix("r:") {
        e.attributes
            .iter()
            .find(|a| a.name.is(crate::R, local))
            .map(|a| a.value.as_str())
    } else {
        e.attribute(name)
    }
}
fn string(s: &str) -> Result<String, XmlError> {
    Ok(s.into())
}
fn int(s: &str) -> Result<i32, XmlError> {
    integer(Some(s), "text integer")
}
fn unsigned(s: &str) -> Result<u32, XmlError> {
    integer(Some(s), "text unsigned integer")
}
pub(super) fn range(s: &str, min: i32, max: i32) -> Result<i32, XmlError> {
    let n = int(s)?;
    if !(min..=max).contains(&n) {
        return Err(malformed("native text value outside range"));
    }
    Ok(n)
}
pub(super) fn coordinate32(s: &str) -> Result<NativeCoordinate, XmlError> {
    let s = s.trim();
    if s.bytes().last().is_some_and(|b| b.is_ascii_digit()) {
        int(s)?;
    }
    NativeCoordinate::try_from(s.to_owned()).map_err(malformed)
}
fn positive_coordinate32(s: &str) -> Result<NativeCoordinate, XmlError> {
    // ST_PositiveCoordinate32 is the integer-only member, unlike Coordinate32.
    range(s, 0, i32::MAX)?;
    NativeCoordinate::try_from(s.trim().to_owned()).map_err(malformed)
}
pub(super) fn percentage_range(s: &str, min: i32, max: i32) -> Result<NativePercentage, XmlError> {
    let s = s.trim();
    // The native union has bounded integer and decimal-percent string branches.
    // Do not silently clamp the string member or round its lexical precision.
    if !s.ends_with('%') {
        range(s, min, max)?;
    }
    any_percentage(s)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum NativeTextPoint {
    HundredthPoints { value: i32 },
    UniversalMeasure { value: NativeCoordinate },
}
fn text_point(s: &str) -> Result<NativeTextPoint, XmlError> {
    let s = s.trim();
    Ok(if s.bytes().last().is_some_and(|b| b.is_ascii_digit()) {
        NativeTextPoint::HundredthPoints {
            value: range(s, -400000, 400000)?,
        }
    } else {
        NativeTextPoint::UniversalMeasure {
            value: NativeCoordinate::try_from(s.to_owned()).map_err(malformed)?,
        }
    })
}

attributes!(SourceTextBodyAttributes {
    rotation: i32 = "rot" => int,
    paragraph_spacing: bool = "spcFirstLastPara" => boolean,
    vertical_overflow: NativeTextVerticalOverflow = "vertOverflow" => enumeration,
    horizontal_overflow: NativeTextHorizontalOverflow = "horzOverflow" => enumeration,
    vertical: NativeTextVertical = "vert" => enumeration,
    wrap: NativeTextWrap = "wrap" => enumeration,
    left_inset: NativeCoordinate = "lIns" => coordinate32,
    top_inset: NativeCoordinate = "tIns" => coordinate32,
    right_inset: NativeCoordinate = "rIns" => coordinate32,
    bottom_inset: NativeCoordinate = "bIns" => coordinate32,
    columns: i32 = "numCol" => |s| range(s, 1, 16),
    column_spacing: NativeCoordinate = "spcCol" => positive_coordinate32,
    right_to_left_columns: bool = "rtlCol" => boolean,
    from_word_art: bool = "fromWordArt" => boolean,
    anchor: NativeTextAnchor = "anchor" => enumeration,
    center_anchor: bool = "anchorCtr" => boolean,
    force_antialiasing: bool = "forceAA" => boolean,
    upright: bool = "upright" => boolean,
    compatible_line_spacing: bool = "compatLnSpc" => boolean,
});
attributes!(SourceTextParagraphAttributes {
    left_margin: i32 = "marL" => |s| range(s, 0, 51206400),
    right_margin: i32 = "marR" => |s| range(s, 0, 51206400),
    level: i32 = "lvl" => |s| range(s, 0, 8),
    indent: i32 = "indent" => |s| range(s, -51206400, 51206400),
    alignment: NativeTextAlign = "algn" => enumeration,
    default_tab_size: NativeCoordinate = "defTabSz" => coordinate32,
    right_to_left: bool = "rtl" => boolean,
    east_asian_line_break: bool = "eaLnBrk" => boolean,
    font_alignment: NativeTextFontAlign = "fontAlgn" => enumeration,
    latin_line_break: bool = "latinLnBrk" => boolean,
    hanging_punctuation: bool = "hangingPunct" => boolean,
});
attributes!(SourceTextCharacterAttributes {
    kumimoji: bool = "kumimoji" => boolean,
    language: String = "lang" => string,
    alternative_language: String = "altLang" => string,
    size: i32 = "sz" => |s| range(s, 100, 400000),
    bold: bool = "b" => boolean,
    italic: bool = "i" => boolean,
    underline: NativeTextUnderline = "u" => enumeration,
    strike: NativeTextStrike = "strike" => enumeration,
    kerning: i32 = "kern" => |s| range(s, 0, 400000),
    caps: NativeTextCaps = "cap" => enumeration,
    spacing: NativeTextPoint = "spc" => text_point,
    normalize_height: bool = "normalizeH" => boolean,
    baseline: NativePercentage = "baseline" => any_percentage,
    no_proof: bool = "noProof" => boolean,
    dirty: bool = "dirty" => boolean,
    error: bool = "err" => boolean,
    smart_clean: bool = "smtClean" => boolean,
    smart_id: u32 = "smtId" => unsigned,
    bookmark: String = "bmk" => string,
});
attributes!(SourceTextHyperlinkAttributes {
    relationship_id: String = "r:id" => string,
    invalid_url: String = "invalidUrl" => string,
    action: String = "action" => string,
    target_frame: String = "tgtFrame" => string,
    tooltip: String = "tooltip" => string,
    history: bool = "history" => boolean,
    highlight_click: bool = "highlightClick" => boolean,
    end_sound: bool = "endSnd" => boolean,
});
