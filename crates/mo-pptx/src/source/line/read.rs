use super::*;
use crate::{
    A,
    source::{SourceLimits, drawingml::*, fill, integer, malformed, paint},
};
use mo_xml::{Element, XmlError};

#[derive(Default)]
pub(in crate::source) struct Budget {
    elements: usize,
    attribute_bytes: usize,
}
impl Budget {
    fn element(&mut self, e: &Element, limits: SourceLimits) -> Result<(), XmlError> {
        self.elements = self
            .elements
            .checked_add(1)
            .ok_or(XmlError::Limit("line elements"))?;
        for a in &e.attributes {
            self.attribute_bytes = self
                .attribute_bytes
                .checked_add(a.value.len())
                .ok_or(XmlError::Limit("line attribute bytes"))?;
        }
        if self.elements > limits.max_line_elements {
            return Err(XmlError::Limit("line elements"));
        }
        if self.attribute_bytes > limits.max_line_attribute_bytes {
            return Err(XmlError::Limit("line attribute bytes"));
        }
        Ok(())
    }
}
pub(in crate::source) enum Declaration {
    Line(SourceLine),
    Reference(SourceLineReference),
}
pub(in crate::source) struct Reader {
    pub depth: usize,
    value: Declaration,
    rank: u8,
    color_parent: Option<usize>,
    retained_depth: Option<usize>,
    fill_capture: Option<paint::Reader>,
}
impl Reader {
    pub fn new(
        e: &Element,
        depth: usize,
        ordinal: u32,
        budget: &mut Budget,
        limits: SourceLimits,
    ) -> Result<Self, XmlError> {
        budget.element(e, limits)?;
        let reference = e.name.is(A, "lnRef");
        let value = if reference {
            Declaration::Reference(SourceLineReference {
                source_ordinal: ordinal,
                index: integer(e.attribute("idx"), "line style index")?,
                color: None,
                retained_ordinals: Vec::new(),
            })
        } else {
            if !e.name.is(A, "ln") && !e.name.is(A, "uLn") {
                return Err(malformed("invalid line root"));
            }
            let width: Option<u32> = e
                .attribute("w")
                .map(|s| integer(Some(s), "line width"))
                .transpose()?;
            if width.is_some_and(|v| v > 20116800) {
                return Err(malformed("native line width outside range"));
            }
            Declaration::Line(SourceLine {
                source_ordinal: ordinal,
                width: width.map(|w| Emu::new(i64::from(w))),
                cap: e.attribute("cap").map(enumeration).transpose()?,
                compound: e.attribute("cmpd").map(enumeration).transpose()?,
                alignment: e.attribute("algn").map(enumeration).transpose()?,
                fill: None,
                dash: None,
                join: None,
                head: None,
                tail: None,
                retained_ordinals: Vec::new(),
            })
        };
        let mut reader = Self {
            depth,
            value,
            rank: 0,
            color_parent: reference.then_some(0),
            retained_depth: None,
            fill_capture: None,
        };
        reader.attributes(
            e,
            ordinal,
            if reference {
                &["idx"]
            } else {
                &["w", "cap", "cmpd", "algn"]
            },
        );
        Ok(reader)
    }
    fn retain(&mut self, ordinal: u32) {
        let entries = match &mut self.value {
            Declaration::Line(v) => &mut v.retained_ordinals,
            Declaration::Reference(v) => &mut v.retained_ordinals,
        };
        if entries.last() != Some(&ordinal) {
            entries.push(ordinal);
        }
    }
    fn attributes(&mut self, e: &Element, ordinal: u32, allowed: &[&str]) {
        if e.attributes
            .iter()
            .any(|a| !a.name.namespace.is_empty() || !allowed.contains(&a.name.local.as_str()))
        {
            self.retain(ordinal);
        }
    }
    fn color_slot(&mut self) -> &mut Option<SourceColor> {
        match &mut self.value {
            Declaration::Reference(v) => &mut v.color,
            Declaration::Line(_) => unreachable!("only line references use the line color reader"),
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn start(
        &mut self,
        e: &Element,
        depth: usize,
        ordinal: u32,
        extension: bool,
        budget: &mut Budget,
        paint_budget: &mut paint::Budget,
        limits: SourceLimits,
    ) -> Result<(), XmlError> {
        budget.element(e, limits)?;
        if let Some(reader) = &mut self.fill_capture {
            return reader.start(e, depth, ordinal, extension, paint_budget, limits);
        }
        let depth = depth - self.depth;
        if self.retained_depth.is_some() {
            return Ok(());
        }
        if extension {
            self.retain(ordinal);
            self.retained_depth = Some(depth);
            return Ok(());
        }
        if e.name.namespace != A {
            return Err(XmlError::Compatibility("unknown line namespace".into()));
        }
        let local = e.name.local.as_str();
        if let Some(parent) = self.color_parent {
            if depth == parent + 1 {
                if self.color_slot().is_some() {
                    return Err(malformed("multiple line colors"));
                }
                *self.color_slot() = Some(color(e, ordinal)?);
                let attrs: &[&str] = match local {
                    "scrgbClr" => &["r", "g", "b"],
                    "hslClr" => &["hue", "sat", "lum"],
                    "sysClr" => &["val", "lastClr"],
                    _ => &["val"],
                };
                self.attributes(e, ordinal, attrs);
            } else if depth == parent + 2 {
                self.color_slot()
                    .as_mut()
                    .ok_or_else(|| malformed("transform outside line color"))?
                    .transforms
                    .push(transform(e)?);
                self.attributes(
                    e,
                    ordinal,
                    if ["comp", "inv", "gray", "gamma", "invGamma"].contains(&local) {
                        &[]
                    } else {
                        &["val"]
                    },
                );
            } else {
                return Err(malformed("nested line color transform"));
            }
            return Ok(());
        }
        if depth == 2 {
            let Declaration::Line(line) = &mut self.value else {
                return Err(malformed("invalid line reference content"));
            };
            if let Some(SourceLineDash::Custom { stops, .. }) = &mut line.dash
                && self.rank == 2
                && local == "ds"
            {
                stops.push(SourceDashStop {
                    source_ordinal: ordinal,
                    dash: positive_percentage(required(e, "d")?)?,
                    space: positive_percentage(required(e, "sp")?)?,
                });
                self.attributes(e, ordinal, &["d", "sp"]);
                return Ok(());
            }
            return Err(malformed("unexpected line child content"));
        }
        if depth != 1 {
            return Err(malformed("nested line declaration"));
        }
        let rank = match local {
            "noFill" | "solidFill" | "gradFill" | "pattFill" => 1,
            "prstDash" | "custDash" => 2,
            "round" | "bevel" | "miter" => 3,
            "headEnd" => 4,
            "tailEnd" => 5,
            "extLst" => 6,
            _ => {
                return Err(XmlError::Compatibility(
                    "unknown native line declaration".into(),
                ));
            }
        };
        if rank <= self.rank {
            return Err(malformed("duplicate or out of order line property"));
        }
        self.rank = rank;
        let Declaration::Line(line) = &mut self.value else {
            return Err(malformed("line reference child is not a color"));
        };
        let attrs: &[&str] = match local {
            "noFill" | "solidFill" | "gradFill" | "pattFill" => {
                self.fill_capture = Some(paint::Reader::new(
                    e,
                    self.depth + depth,
                    ordinal,
                    paint_budget,
                    limits,
                )?);
                return Ok(());
            }
            "prstDash" => {
                line.dash = Some(SourceLineDash::Preset {
                    source_ordinal: ordinal,
                    value: e.attribute("val").map(enumeration).transpose()?,
                });
                &["val"]
            }
            "custDash" => {
                line.dash = Some(SourceLineDash::Custom {
                    source_ordinal: ordinal,
                    stops: Vec::new(),
                });
                &[]
            }
            "round" => {
                line.join = Some(SourceLineJoin::Round {
                    source_ordinal: ordinal,
                });
                &[]
            }
            "bevel" => {
                line.join = Some(SourceLineJoin::Bevel {
                    source_ordinal: ordinal,
                });
                &[]
            }
            "miter" => {
                line.join = Some(SourceLineJoin::Miter {
                    source_ordinal: ordinal,
                    limit: e.attribute("lim").map(positive_percentage).transpose()?,
                });
                &["lim"]
            }
            "headEnd" | "tailEnd" => {
                let value = Some(SourceLineEnd {
                    source_ordinal: ordinal,
                    kind: e.attribute("type").map(enumeration).transpose()?,
                    width: e.attribute("w").map(enumeration).transpose()?,
                    length: e.attribute("len").map(enumeration).transpose()?,
                });
                if local == "headEnd" {
                    line.head = value;
                } else {
                    line.tail = value;
                }
                &["type", "w", "len"]
            }
            "extLst" => {
                self.retain(ordinal);
                self.retained_depth = Some(depth);
                return Ok(());
            }
            _ => unreachable!("rank verified"),
        };
        self.attributes(e, ordinal, attrs);
        Ok(())
    }
    pub fn text(&self, text: &str) -> Result<(), XmlError> {
        if let Some(reader) = &self.fill_capture {
            return reader.text(text);
        }
        if self.retained_depth.is_none() && !text.trim().is_empty() {
            return Err(malformed("unexpected line character data"));
        }
        Ok(())
    }
    pub fn end(&mut self, depth: usize) -> Result<(), XmlError> {
        if self.fill_capture.as_ref().is_some_and(|r| r.depth == depth) {
            let parsed = self.fill_capture.take().expect("checked fill").finish()?;
            let paint::Declaration::Fill(fill) = parsed.declaration else {
                unreachable!("line fill root");
            };
            if !parsed.nodes.is_empty() {
                return Err(malformed("effect graph is not permitted in a line fill"));
            }
            let Declaration::Line(line) = &mut self.value else {
                unreachable!("line fill owner")
            };
            line.retained_ordinals.extend(fill.retained_ordinals);
            let source_ordinal = fill.source_ordinal;
            line.fill = Some(match fill.definition {
                fill::SourceFillDefinition::None {} => SourceLineFill::None { source_ordinal },
                fill::SourceFillDefinition::Solid { color } => SourceLineFill::Solid {
                    source_ordinal,
                    color,
                },
                fill::SourceFillDefinition::Gradient(gradient) => SourceLineFill::Gradient {
                    source_ordinal,
                    gradient: Box::new(gradient),
                },
                fill::SourceFillDefinition::Pattern(pattern) => SourceLineFill::Pattern {
                    source_ordinal,
                    pattern: Box::new(pattern),
                },
                _ => unreachable!("only native line fill types accepted"),
            });
        } else if let Some(reader) = &mut self.fill_capture {
            reader.end(depth)?;
        }
        let depth = depth - self.depth;
        if self.retained_depth == Some(depth) {
            self.retained_depth = None;
        }
        if self.color_parent == Some(depth) {
            self.color_parent = None;
        }
        Ok(())
    }
    pub fn finish(self) -> Declaration {
        self.value
    }
}
