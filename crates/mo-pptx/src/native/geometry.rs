use super::*;
use crate::source::{SourceTransform, geometry::*};

pub(crate) fn transform(x: &mut Xml, value: &SourceTransform) -> Result<(), PptxError> {
    transform_tag(x, value, "a:xfrm")
}
pub(crate) fn frame_transform(x: &mut Xml, value: &SourceTransform) -> Result<(), PptxError> {
    transform_tag(x, value, "p:xfrm")
}
fn transform_tag(x: &mut Xml, value: &SourceTransform, tag: &str) -> Result<(), PptxError> {
    x.raw("<")?;
    x.raw(tag)?;
    attribute(x, "rot", &value.rotation)?;
    attribute(x, "flipH", &value.flip_horizontal)?;
    attribute(x, "flipV", &value.flip_vertical)?;
    x.raw(">")?;
    for (tag, origin, size) in [
        ("off", value.origin, None),
        ("ext", None, value.size),
        ("chOff", value.child_origin, None),
        ("chExt", None, value.child_size),
    ] {
        if let Some(p) = origin {
            x.raw("<a:")?;
            x.raw(tag)?;
            x.attr("x", p.x.get())?;
            x.attr("y", p.y.get())?;
            x.raw("/>")?;
        }
        if let Some(s) = size {
            x.raw("<a:")?;
            x.raw(tag)?;
            x.attr("cx", s.width.get())?;
            x.attr("cy", s.height.get())?;
            x.raw("/>")?;
        }
    }
    x.raw("</")?;
    x.raw(tag)?;
    x.raw(">")
}
fn guides(
    x: &mut Xml,
    tag: &str,
    values: &Option<SourceGeometryList<SourceGuide>>,
    compact_empty: bool,
) -> Result<(), PptxError> {
    if let Some(values) = values {
        x.raw("<a:")?;
        x.raw(tag)?;
        if compact_empty && values.entries.is_empty() {
            return x.raw("/>");
        }
        x.raw(">")?;
        for v in &values.entries {
            x.raw("<a:gd")?;
            x.attr("name", &v.name)?;
            x.attr("fmla", &v.formula)?;
            x.raw("/>")?;
        }
        x.raw("</a:")?;
        x.raw(tag)?;
        x.raw(">")?;
    }
    Ok(())
}
fn point(x: &mut Xml, value: &SourceGeometryPoint) -> Result<(), PptxError> {
    x.raw("<a:pt")?;
    x.attr("x", &value.x)?;
    x.attr("y", &value.y)?;
    x.raw("/>")
}
pub(crate) fn geometry(x: &mut Xml, value: &SourceGeometry) -> Result<(), PptxError> {
    match &value.definition {
        SourceGeometryDefinition::Preset {
            preset,
            adjustments,
        } => {
            x.raw("<a:prstGeom")?;
            x.attr("prst", lexical(preset)?)?;
            x.raw(">")?;
            guides(x, "avLst", adjustments, lexical(preset)? == "line")?;
            x.raw("</a:prstGeom>")
        }
        SourceGeometryDefinition::Custom(v) => {
            x.raw("<a:custGeom>")?;
            guides(x, "avLst", &v.adjustments, true)?;
            guides(x, "gdLst", &v.guides, true)?;
            if v.handles.as_ref().is_some_and(|a| !a.entries.is_empty())
                || v.connections
                    .as_ref()
                    .is_some_and(|a| !a.entries.is_empty())
            {
                return Err(unexpected());
            }
            x.raw("<a:ahLst/><a:cxnLst/>")?;
            if let Some(r) = &v.text_rect {
                x.raw("<a:rect")?;
                for (name, v) in [
                    ("l", &r.left),
                    ("t", &r.top),
                    ("r", &r.right),
                    ("b", &r.bottom),
                ] {
                    x.attr(name, v)?;
                }
                x.raw("/>")?;
            }
            x.raw("<a:pathLst>")?;
            for path in &v.paths.entries {
                x.raw("<a:path")?;
                if let Some(w) = path.width {
                    x.attr("w", w.get())?;
                }
                if let Some(h) = path.height {
                    x.attr("h", h.get())?;
                }
                x.raw(">")?;
                for command in &path.commands {
                    let (tag, points) = match command {
                        SourceGeometryCommand::Move { to, .. } => ("moveTo", vec![to]),
                        SourceGeometryCommand::Line { to, .. } => ("lnTo", vec![to]),
                        SourceGeometryCommand::Quadratic { control, to, .. } => {
                            ("quadBezTo", vec![control, to])
                        }
                        SourceGeometryCommand::Cubic {
                            control1,
                            control2,
                            to,
                            ..
                        } => ("cubicBezTo", vec![control1, control2, to]),
                        SourceGeometryCommand::Close { .. } => ("close", vec![]),
                        _ => return Err(unexpected()),
                    };
                    x.raw("<a:")?;
                    x.raw(tag)?;
                    x.raw(">")?;
                    for p in points {
                        point(x, p)?;
                    }
                    x.raw("</a:")?;
                    x.raw(tag)?;
                    x.raw(">")?;
                }
                x.raw("</a:path>")?;
            }
            x.raw("</a:pathLst></a:custGeom>")
        }
    }
}
