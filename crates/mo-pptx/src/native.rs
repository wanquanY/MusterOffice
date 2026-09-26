//! Format serialization of validated author-plan declarations. Interpretation,
//! native defaults and author-unit conversion belong to the shared plan.
mod geometry;
mod paint;
mod text;
use crate::{PptxError, value, xml::Xml};
pub(crate) use geometry::{geometry, transform};
pub(crate) use paint::{background, color, fill, line};
use serde::Serialize;
pub(crate) use text::{font, text};

pub(crate) fn lexical(value: &impl Serialize) -> Result<String, PptxError> {
    match serde_json::to_value(value).map_err(|e| crate::value("native value", e.to_string()))? {
        serde_json::Value::String(s) => Ok(s),
        serde_json::Value::Number(n) => Ok(n.to_string()),
        serde_json::Value::Bool(v) => Ok(u8::from(v).to_string()),
        _ => Err(crate::value("native value", "expected scalar")),
    }
}
fn attribute(x: &mut Xml, name: &str, value: &Option<impl Serialize>) -> Result<(), PptxError> {
    if let Some(value) = value {
        x.attr(name, lexical(value)?)?;
    }
    Ok(())
}
fn unexpected() -> PptxError {
    value(
        "author plan",
        "unsupported native declaration in writer projection",
    )
}
