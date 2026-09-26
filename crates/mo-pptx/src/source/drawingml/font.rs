//! Shared native font declarations. No font lookup or implicit defaulting.
use super::required;
use crate::source::{integer, malformed};
use mo_xml::{Element, XmlError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTextFont {
    pub typeface: String,
    pub panose: Option<String>,
    pub pitch_family: Option<u8>,
    pub charset: Option<i8>,
}
pub(in crate::source) fn text_font(e: &Element) -> Result<SourceTextFont, XmlError> {
    let panose = e.attribute("panose").map(str::trim);
    if panose.is_some_and(|p| p.len() != 20 || !p.bytes().all(|c| c.is_ascii_hexdigit())) {
        return Err(malformed("invalid font Panose"));
    }
    let pitch_family = e
        .attribute("pitchFamily")
        .map(|s| integer(Some(s), "font pitchFamily"))
        .transpose()?;
    if pitch_family.is_some_and(|n| {
        ![
            0, 1, 2, 16, 17, 18, 32, 33, 34, 48, 49, 50, 64, 65, 66, 80, 81, 82,
        ]
        .contains(&n)
    }) {
        return Err(malformed("invalid font pitchFamily"));
    }
    Ok(SourceTextFont {
        typeface: required(e, "typeface")?.into(),
        panose: panose.map(str::to_owned),
        pitch_family,
        charset: e
            .attribute("charset")
            .map(|s| integer(Some(s), "font charset"))
            .transpose()?,
    })
}
