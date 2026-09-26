use super::*;
use crate::source::{drawingml::*, fill::*, line::*};

pub(crate) fn color(x: &mut Xml, color: &SourceColor) -> Result<(), PptxError> {
    let tag = match color.value {
        SourceColorValue::Srgb { .. } => "srgbClr",
        SourceColorValue::Scheme { .. } => "schemeClr",
        _ => return Err(unexpected()),
    };
    x.raw("<a:")?;
    x.raw(tag)?;
    match &color.value {
        SourceColorValue::Srgb { rgb } => {
            x.attr("val", format!("{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]))?
        }
        SourceColorValue::Scheme { slot } => x.attr("val", lexical(slot)?)?,
        _ => return Err(unexpected()),
    }
    if matches!(color.value, SourceColorValue::Scheme { .. }) && color.transforms.is_empty() {
        return x.raw("/>");
    }
    x.raw(">")?;
    for transform in &color.transforms {
        let SourceColorTransform::Alpha(v) = transform else {
            return Err(unexpected());
        };
        x.raw("<a:alpha")?;
        x.attr("val", lexical(v)?)?;
        x.raw("/>")?;
    }
    x.raw("</a:")?;
    x.raw(tag)?;
    x.raw(">")
}
pub(crate) fn fill(x: &mut Xml, value: &SourceFill) -> Result<(), PptxError> {
    match &value.definition {
        SourceFillDefinition::None {} => x.raw("<a:noFill/>"),
        SourceFillDefinition::Solid { color: Some(c) } => {
            x.raw("<a:solidFill>")?;
            color(x, c)?;
            x.raw("</a:solidFill>")
        }
        _ => Err(unexpected()),
    }
}
pub(crate) fn line(x: &mut Xml, value: &SourceLine) -> Result<(), PptxError> {
    x.raw("<a:ln")?;
    if let Some(width) = value.width {
        x.attr("w", width.get())?;
    }
    attribute(x, "cap", &value.cap)?;
    x.raw(">")?;
    match &value.fill {
        Some(SourceLineFill::None { .. }) => x.raw("<a:noFill/>")?,
        Some(SourceLineFill::Solid { color: Some(c), .. }) => {
            x.raw("<a:solidFill>")?;
            color(x, c)?;
            x.raw("</a:solidFill>")?;
        }
        _ => return Err(unexpected()),
    }
    if let Some(dash) = &value.dash {
        let SourceLineDash::Preset { value, .. } = dash else {
            return Err(unexpected());
        };
        x.raw("<a:prstDash")?;
        attribute(x, "val", value)?;
        x.raw("/>")?;
    }
    match &value.join {
        Some(SourceLineJoin::Round { .. }) => x.raw("<a:round/>")?,
        Some(SourceLineJoin::Bevel { .. }) => x.raw("<a:bevel/>")?,
        Some(SourceLineJoin::Miter { limit, .. }) => {
            x.raw("<a:miter")?;
            attribute(x, "lim", limit)?;
            x.raw("/>")?;
        }
        None => (),
    }
    x.raw("</a:ln>")
}
pub(crate) fn background(x: &mut Xml, value: &Option<SourceBackground>) -> Result<(), PptxError> {
    if let Some(value) = value {
        let SourceBackgroundDefinition::Properties {
            fill: f, effects, ..
        } = &value.definition
        else {
            return Err(unexpected());
        };
        if effects
            .as_ref()
            .is_none_or(|e| !e.is_explicitly_empty_list())
        {
            return Err(unexpected());
        }
        x.raw("<p:bg><p:bgPr>")?;
        fill(x, f)?;
        x.raw("<a:effectLst/></p:bgPr></p:bg>")?;
    }
    Ok(())
}
