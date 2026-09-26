use crate::{COLOR_SLOTS, PptxError, R, cancelled, text, value, xml::Xml};
use mo_common::{ObjectId, ResourceId};
use mo_opc::{PartName, Relationship, RelationshipSource};
use mo_presentation_model::*;
use std::collections::BTreeMap;

pub(crate) struct Drawing<'a> {
    pub document: &'a Document,
    pub object_ids: &'a BTreeMap<ObjectId, u32>,
    pub images: &'a BTreeMap<ResourceId, PartName>,
    pub source: PartName,
    pub relationships: Vec<Relationship>,
    pub check: &'a dyn Fn() -> bool,
}
impl Drawing<'_> {
    pub fn relationship(&mut self, kind: &str, target: &PartName) -> Result<String, PptxError> {
        let id = format!("rId{}", self.relationships.len() + 1);
        self.relationships.push(Relationship::new(
            &RelationshipSource::Part(self.source.clone()),
            id.clone(),
            format!("{R}/{kind}"),
            target.to_string(),
            false,
        )?);
        Ok(id)
    }
    pub fn tree(&mut self, x: &mut Xml, objects: &[ObjectId]) -> Result<(), PptxError> {
        x.raw("<p:spTree><p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>")?;
        for id in objects {
            self.object(x, id)?;
        }
        x.raw("</p:spTree>")
    }
    fn nonvisual(&self, x: &mut Xml, object: &Object) -> Result<(), PptxError> {
        if object.accessibility.decorative {
            return Err(PptxError::Unsupported(
                "decorative accessibility extension".into(),
            ));
        }
        x.raw("<p:cNvPr")?;
        x.attr("id", self.object_ids[&object.id])?;
        x.attr("name", object.id.as_str())?;
        x.attr("title", &object.accessibility.title)?;
        x.attr("descr", &object.accessibility.description)?;
        x.raw("/>")
    }
    fn object(&mut self, x: &mut Xml, id: &ObjectId) -> Result<(), PptxError> {
        cancelled(self.check)?;
        let o = &self.document.objects[id];
        match &o.content {
            ObjectContent::Shape {
                geometry,
                text: body,
            } => {
                x.raw("<p:sp><p:nvSpPr>")?;
                self.nonvisual(x, o)?;
                x.raw("<p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr>")?;
                transform(x, &o.transform, None)?;
                geometry_xml(x, geometry, o.transform.size)?;
                appearance(x, &o.appearance)?;
                x.raw("</p:spPr>")?;
                if let Some(body) = body {
                    text::body(x, body, self.document)?;
                }
                x.raw("</p:sp>")?;
            }
            ObjectContent::Picture { resource, crop } => {
                let image = self.images[resource].clone();
                let rel = self.relationship("image", &image)?;
                x.raw("<p:pic><p:nvPicPr>")?;
                self.nonvisual(x, o)?;
                x.raw("<p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip")?;
                x.attr("r:embed", rel)?;
                x.raw("/><a:srcRect")?;
                for (name, amount) in [
                    ("l", crop.left),
                    ("t", crop.top),
                    ("r", crop.right),
                    ("b", crop.bottom),
                ] {
                    if amount % 10 != 0 {
                        return Err(value(
                            "picture crop",
                            "millionths must be exactly representable as native thousandths of a percent",
                        ));
                    }
                    x.attr(name, amount / 10)?;
                }
                x.raw("/><a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr>")?;
                transform(x, &o.transform, None)?;
                geometry_xml(x, &Geometry::Rectangle, o.transform.size)?;
                appearance(x, &o.appearance)?;
                x.raw("</p:spPr></p:pic>")?;
            }
            ObjectContent::Group { children, viewport } => {
                if o.appearance.stroke != Inherited::Inherit {
                    return Err(PptxError::Unsupported("group stroke semantics".into()));
                }
                x.raw("<p:grpSp><p:nvGrpSpPr>")?;
                self.nonvisual(x, o)?;
                x.raw("<p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr>")?;
                transform(x, &o.transform, Some(*viewport))?;
                if let Inherited::Value(f) = &o.appearance.fill {
                    fill(x, f)?;
                }
                x.raw("</p:grpSpPr>")?;
                for child in children {
                    self.object(x, child)?;
                }
                x.raw("</p:grpSp>")?;
            }
            ObjectContent::Connector { start, end } => {
                x.raw("<p:cxnSp><p:nvCxnSpPr>")?;
                self.nonvisual(x, o)?;
                x.raw("<p:cNvCxnSpPr>")?;
                for (tag, endpoint, is_end) in [("stCxn", start, false), ("endCxn", end, true)] {
                    match endpoint {
                        ConnectorEndpoint::Attached { object, site } => {
                            if *site > 3
                                || !matches!(
                                    self.document.objects[object].content,
                                    ObjectContent::Shape {
                                        geometry: Geometry::Rectangle
                                            | Geometry::Ellipse
                                            | Geometry::RoundRectangle { .. },
                                        ..
                                    }
                                )
                            {
                                return Err(PptxError::Unsupported(
                                    "connector site on custom geometry or non-shape".into(),
                                ));
                            }
                            x.raw("<a:")?;
                            x.raw(tag)?;
                            x.attr("id", self.object_ids[object])?;
                            x.attr("idx", site)?;
                            x.raw("/>")?;
                        }
                        ConnectorEndpoint::Free { position } => {
                            let expected = Point {
                                x: if is_end {
                                    o.transform
                                        .origin
                                        .x
                                        .checked_add(o.transform.size.width)
                                        .map_err(|_| value("connector", "overflow"))?
                                } else {
                                    o.transform.origin.x
                                },
                                y: if is_end {
                                    o.transform
                                        .origin
                                        .y
                                        .checked_add(o.transform.size.height)
                                        .map_err(|_| value("connector", "overflow"))?
                                } else {
                                    o.transform.origin.y
                                },
                            };
                            if *position != expected
                                || o.transform.normalized_rotation() != 0
                                || o.transform.flip_horizontal
                                || o.transform.flip_vertical
                            {
                                return Err(PptxError::Unsupported(
                                    "free connector endpoint normalization".into(),
                                ));
                            }
                        }
                    }
                }
                x.raw("</p:cNvCxnSpPr><p:nvPr/></p:nvCxnSpPr><p:spPr>")?;
                transform(x, &o.transform, None)?;
                x.raw("<a:prstGeom prst=\"line\"><a:avLst/></a:prstGeom>")?;
                appearance(x, &o.appearance)?;
                x.raw("</p:spPr></p:cxnSp>")?;
            }
        }
        Ok(())
    }
}

pub(crate) fn color(x: &mut Xml, color: &Color) -> Result<(), PptxError> {
    match color {
        Color::Srgb { rgba } => {
            x.raw("<a:srgbClr")?;
            x.attr(
                "val",
                format!("{:02X}{:02X}{:02X}", rgba.red, rgba.green, rgba.blue),
            )?;
            x.raw(">")?;
            if rgba.alpha != 255 {
                // Nearest native 1/1000-percent value. Converting back to RGBA8
                // recovers every author alpha exactly; no float or host rounding.
                let scaled = (u32::from(rgba.alpha) * 100000 + 127) / 255;
                x.raw("<a:alpha")?;
                x.attr("val", scaled)?;
                x.raw("/>")?;
            }
            x.raw("</a:srgbClr>")?;
        }
        Color::Theme { slot } => {
            x.raw("<a:schemeClr")?;
            x.attr(
                "val",
                COLOR_SLOTS
                    .iter()
                    .find(|(s, _)| s == slot)
                    .expect("all slots covered")
                    .1,
            )?;
            x.raw("/>")?;
        }
    }
    Ok(())
}
pub(crate) fn fill(x: &mut Xml, fill: &Fill) -> Result<(), PptxError> {
    match fill {
        Fill::None => x.raw("<a:noFill/>"),
        Fill::Solid { color: c } => {
            x.raw("<a:solidFill>")?;
            color(x, c)?;
            x.raw("</a:solidFill>")
        }
    }
}
fn appearance(x: &mut Xml, appearance: &Appearance) -> Result<(), PptxError> {
    if let Inherited::Value(f) = &appearance.fill {
        fill(x, f)?;
    }
    if let Inherited::Value(stroke) = &appearance.stroke {
        x.raw("<a:ln")?;
        if let Stroke::Solid { width, cap, .. } = stroke {
            if !(0..=20116800).contains(&width.get()) {
                return Err(value("stroke.width", "outside native range"));
            }
            x.attr("w", width.get())?;
            if let Some(cap) = cap {
                x.attr(
                    "cap",
                    match cap {
                        LineCap::Flat => "flat",
                        LineCap::Round => "rnd",
                        LineCap::Square => "sq",
                    },
                )?;
            }
        }
        x.raw(">")?;
        match stroke {
            Stroke::None {} => fill(x, &Fill::None)?,
            Stroke::Solid { color, .. } => fill(
                x,
                &Fill::Solid {
                    color: color.clone(),
                },
            )?,
        }
        if let Stroke::Solid {
            join: Some(join), ..
        } = stroke
        {
            match join {
                LineJoin::Round {} => x.raw("<a:round/>")?,
                LineJoin::Bevel {} => x.raw("<a:bevel/>")?,
                LineJoin::Miter { limit } => {
                    if limit.is_some_and(|v| v > i32::MAX as u32) {
                        return Err(value(
                            "stroke.join.limit",
                            "outside native percentage range",
                        ));
                    }
                    x.raw("<a:miter")?;
                    if let Some(limit) = limit {
                        x.attr("lim", limit)?;
                    }
                    x.raw("/>")?;
                }
            }
        }
        x.raw("</a:ln>")?;
    }
    Ok(())
}
fn coordinate(n: i64) -> Result<i64, PptxError> {
    if !(-27273042329600..=27273042316900).contains(&n) {
        Err(value("coordinate", "outside DrawingML coordinate range"))
    } else {
        Ok(n)
    }
}
fn point(x: &mut Xml, tag: &str, p: Point) -> Result<(), PptxError> {
    x.raw("<a:")?;
    x.raw(tag)?;
    x.attr("x", coordinate(p.x.get())?)?;
    x.attr("y", coordinate(p.y.get())?)?;
    x.raw("/>")
}
fn size(x: &mut Xml, tag: &str, s: Size) -> Result<(), PptxError> {
    x.raw("<a:")?;
    x.raw(tag)?;
    x.attr("cx", coordinate(s.width.get())?)?;
    x.attr("cy", coordinate(s.height.get())?)?;
    x.raw("/>")
}
fn transform(x: &mut Xml, t: &Transform, viewport: Option<Size>) -> Result<(), PptxError> {
    x.raw("<a:xfrm")?;
    // New authored packages use one equivalent static orientation. Some readers
    // choose group scaling axes before reducing signed/multi-turn angles. Source
    // package preservation is a separate path and must retain untouched XML.
    x.attr("rot", t.normalized_rotation())?;
    x.attr("flipH", u8::from(t.flip_horizontal))?;
    x.attr("flipV", u8::from(t.flip_vertical))?;
    x.raw(">")?;
    point(x, "off", t.origin)?;
    size(x, "ext", t.size)?;
    if let Some(v) = viewport {
        point(
            x,
            "chOff",
            Point {
                x: mo_common::Emu::ZERO,
                y: mo_common::Emu::ZERO,
            },
        )?;
        size(x, "chExt", v)?;
    }
    x.raw("</a:xfrm>")
}
fn geometry_xml(x: &mut Xml, g: &Geometry, s: Size) -> Result<(), PptxError> {
    match g {
        Geometry::Rectangle | Geometry::Ellipse | Geometry::RoundRectangle { .. } => {
            x.raw("<a:prstGeom")?;
            x.attr(
                "prst",
                match g {
                    Geometry::Rectangle => "rect",
                    Geometry::Ellipse => "ellipse",
                    _ => "roundRect",
                },
            )?;
            x.raw("><a:avLst>")?;
            if let Geometry::RoundRectangle { radius } = g {
                let short = s.width.get().min(s.height.get());
                let numerator = i128::from(radius.get()) * 100000;
                if short <= 0 || radius.get() > short / 2 || numerator % i128::from(short) != 0 {
                    return Err(value(
                        "roundRectangle.radius",
                        "radius cannot be represented exactly by a native adjustment",
                    ));
                }
                x.raw("<a:gd name=\"adj\"")?;
                x.attr("fmla", format!("val {}", numerator / i128::from(short)))?;
                x.raw("/>")?;
            }
            x.raw("</a:avLst></a:prstGeom>")?;
        }
        Geometry::Path { commands, viewport } => {
            x.raw("<a:custGeom><a:avLst/><a:gdLst/><a:ahLst/><a:cxnLst/><a:rect l=\"0\" t=\"0\" r=\"r\" b=\"b\"/><a:pathLst><a:path")?;
            x.attr("w", coordinate(viewport.width.get())?)?;
            x.attr("h", coordinate(viewport.height.get())?)?;
            x.raw(">")?;
            for command in commands {
                let (tag, points) = match command {
                    PathCommand::Move { to } => ("moveTo", vec![*to]),
                    PathCommand::Line { to } => ("lnTo", vec![*to]),
                    PathCommand::Quadratic { control, to } => ("quadBezTo", vec![*control, *to]),
                    PathCommand::Cubic {
                        control1,
                        control2,
                        to,
                    } => ("cubicBezTo", vec![*control1, *control2, *to]),
                    PathCommand::Close => ("close", vec![]),
                };
                x.raw("<a:")?;
                x.raw(tag)?;
                x.raw(">")?;
                for p in points {
                    point(x, "pt", p)?;
                }
                x.raw("</a:")?;
                x.raw(tag)?;
                x.raw(">")?;
            }
            x.raw("</a:path></a:pathLst></a:custGeom>")?;
        }
    }
    Ok(())
}
