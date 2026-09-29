//! ISO 29500 CT_HeaderFooter. An omitted attribute enables its placeholder;
//! absent declarations are retained separately from explicit defaults.
use super::{PlaceholderKind, boolean};
use mo_xml::{Element, XmlError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceHeaderFooter {
    pub date_time: Option<bool>,
    pub footer: Option<bool>,
    pub header: Option<bool>,
    pub slide_number: Option<bool>,
}
impl SourceHeaderFooter {
    pub(super) fn read(element: &Element) -> Result<Self, XmlError> {
        Ok(Self {
            date_time: element.attribute("dt").map(boolean).transpose()?,
            footer: element.attribute("ftr").map(boolean).transpose()?,
            header: element.attribute("hdr").map(boolean).transpose()?,
            slide_number: element.attribute("sldNum").map(boolean).transpose()?,
        })
    }
    /// Only an explicit disabled declaration removes a master/layout placeholder.
    /// It does not hide an independent shape authored on the slide itself.
    pub fn disables(&self, kind: PlaceholderKind) -> bool {
        (match kind {
            PlaceholderKind::Date => self.date_time,
            PlaceholderKind::Footer => self.footer,
            PlaceholderKind::Header => self.header,
            PlaceholderKind::SlideNumber => self.slide_number,
            _ => None,
        }) == Some(false)
    }
}
