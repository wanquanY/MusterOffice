use super::super::Budget;
use super::*;
use crate::{
    A,
    source::{SourceLimits, drawingml::*, line, malformed, paint, theme::SourceSupplementalFont},
};
use mo_xml::{Element, XmlError};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Style,
    Background,
    Region,
    Text,
    Fonts,
    FontRef,
    Leaf,
    Cell,
    Borders,
    Border(usize),
    Fill(bool),
    Effect,
    Color(bool),
}
struct Frame {
    kind: Kind,
    rank: u8,
}
enum Capture {
    Paint {
        background: bool,
        effects: bool,
        reader: Box<paint::Reader>,
    },
    Line {
        edge: usize,
        reader: Box<line::Reader>,
    },
}
impl Capture {
    fn depth(&self) -> usize {
        match self {
            Self::Paint { reader, .. } => reader.depth,
            Self::Line { reader, .. } => reader.depth,
        }
    }
}
pub(in crate::source) struct Reader {
    pub depth: usize,
    value: SourceTableStyle,
    frames: Vec<Frame>,
    region: Option<TableStyleRegion>,
    opaque: Option<usize>,
    capture: Option<Capture>,
}
pub(super) fn guid(value: &str) -> Result<String, XmlError> {
    let v = value.trim().as_bytes();
    if v.len() != 38
        || v[0] != b'{'
        || v[37] != b'}'
        || v[1..37].iter().enumerate().any(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                *b != b'-'
            } else {
                !b.is_ascii_hexdigit()
            }
        })
    {
        return Err(malformed("invalid native table style GUID"));
    }
    Ok(value.trim().to_ascii_uppercase())
}
impl Reader {
    pub fn new(
        e: &Element,
        depth: usize,
        ordinal: u32,
        budget: &mut Budget,
        limits: SourceLimits,
    ) -> Result<Self, XmlError> {
        budget.style(limits)?;
        if !(e.name.is(A, "tblStyle") || e.name.is(A, "tableStyle")) {
            return Err(malformed("invalid table style root"));
        }
        let style_id = required(e, "styleId")?;
        guid(style_id)?;
        let mut out = Self {
            depth,
            frames: vec![Frame {
                kind: Kind::Style,
                rank: 0,
            }],
            region: None,
            opaque: None,
            capture: None,
            value: SourceTableStyle {
                source_ordinal: ordinal,
                style_id: style_id.into(),
                name: required(e, "styleName")?.into(),
                background: None,
                parts: BTreeMap::new(),
                retained_ordinals: vec![],
                effect_nodes: BTreeMap::new(),
            },
        };
        out.attributes(e, ordinal, &["styleId", "styleName"]);
        Ok(out)
    }
    fn retain(&mut self, ordinal: u32) {
        let retained = if self.frames.iter().any(|f| f.kind == Kind::Region) {
            &mut self.part().retained_ordinals
        } else if self.frames.iter().any(|f| f.kind == Kind::Background) {
            &mut self
                .value
                .background
                .as_mut()
                .expect("background")
                .retained_ordinals
        } else {
            &mut self.value.retained_ordinals
        };
        if retained.last() != Some(&ordinal) {
            retained.push(ordinal);
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
    fn order(&mut self, rank: u8, repeated: bool) -> Result<(), XmlError> {
        let f = self.frames.last_mut().expect("style frame");
        if rank < f.rank || (rank == f.rank && !repeated) {
            return Err(malformed(
                "duplicate or out-of-order table style declaration",
            ));
        }
        f.rank = rank;
        Ok(())
    }
    fn part(&mut self) -> &mut SourceTablePartStyle {
        self.value
            .parts
            .get_mut(&self.region.expect("active style region"))
            .expect("region")
    }
    fn text_style(&mut self) -> &mut SourceTableTextStyle {
        self.part().text.as_mut().expect("text style")
    }
    fn cell(&mut self) -> &mut SourceTableCellStyle {
        self.part().cell.as_mut().expect("cell style")
    }
    fn color(&mut self, reference: bool) -> &mut SourceColor {
        if reference {
            let Some(SourceTableFontStyle::Reference { color, .. }) = &mut self.text_style().font
            else {
                unreachable!("font reference")
            };
            color.as_mut().expect("reference color")
        } else {
            self.text_style().color.as_mut().expect("text color")
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
        line_budget: &mut line::Budget,
        paint_budget: &mut paint::Budget,
        limits: SourceLimits,
    ) -> Result<(), XmlError> {
        budget.element(e, limits)?;
        if let Some(c) = &mut self.capture {
            return match c {
                Capture::Paint { reader, .. } => {
                    reader.start(e, depth, ordinal, extension, paint_budget, limits)
                }
                Capture::Line { reader, .. } => reader.start(
                    e,
                    depth,
                    ordinal,
                    extension,
                    line_budget,
                    paint_budget,
                    limits,
                ),
            };
        }
        if self.opaque.is_some() {
            return Ok(());
        }
        if depth != self.depth + self.frames.len() {
            return Err(malformed("invalid table style nesting"));
        }
        if extension || e.name.namespace != A {
            self.retain(ordinal);
            self.opaque = Some(depth);
            return Ok(());
        }
        let parent = self.frames.last().expect("style frame").kind;
        let local = e.name.local.as_str();
        let mut child = None;
        match (parent, local) {
            (Kind::Style, "tblBg") => {
                self.order(1, false)?;
                self.value.background = Some(SourceTableBackgroundStyle {
                    source_ordinal: ordinal,
                    fill: None,
                    effects: None,
                    retained_ordinals: vec![],
                });
                child = Some(Kind::Background);
            }
            (Kind::Style, _) if enumeration::<TableStyleRegion>(local).is_ok() => {
                let region: TableStyleRegion = enumeration(local)?;
                let rank = TableStyleRegion::ORDER
                    .iter()
                    .position(|v| *v == region)
                    .expect("region") as u8
                    + 2;
                self.order(rank, false)?;
                self.value.parts.insert(
                    region,
                    SourceTablePartStyle {
                        source_ordinal: ordinal,
                        text: None,
                        cell: None,
                        retained_ordinals: vec![],
                    },
                );
                self.region = Some(region);
                child = Some(Kind::Region);
            }
            (Kind::Region, "tcTxStyle") => {
                self.order(1, false)?;
                self.part().text = Some(SourceTableTextStyle {
                    source_ordinal: ordinal,
                    bold: e.attribute("b").map(enumeration).transpose()?,
                    italic: e.attribute("i").map(enumeration).transpose()?,
                    font: None,
                    color: None,
                });
                self.attributes(e, ordinal, &["b", "i"]);
                child = Some(Kind::Text);
            }
            (Kind::Region, "tcStyle") => {
                self.order(2, false)?;
                self.part().cell = Some(SourceTableCellStyle {
                    source_ordinal: ordinal,
                    borders: None,
                    fill: None,
                    cell_3d_ordinal: None,
                });
                child = Some(Kind::Cell);
            }
            (Kind::Text, "font") => {
                self.order(1, false)?;
                self.text_style().font = Some(SourceTableFontStyle::Collection {
                    source_ordinal: ordinal,
                    fonts: SourceFontCollection::default(),
                });
                child = Some(Kind::Fonts);
            }
            (Kind::Text, "fontRef") => {
                self.order(1, false)?;
                self.text_style().font = Some(SourceTableFontStyle::Reference {
                    source_ordinal: ordinal,
                    index: enumeration(required(e, "idx")?)?,
                    color: None,
                });
                self.attributes(e, ordinal, &["idx"]);
                child = Some(Kind::FontRef);
            }
            (Kind::Fonts, "latin" | "ea" | "cs" | "font") => {
                let rank = match local {
                    "latin" => 1,
                    "ea" => 2,
                    "cs" => 3,
                    _ => 4,
                };
                self.order(rank, local == "font")?;
                let Some(SourceTableFontStyle::Collection { fonts, .. }) =
                    &mut self.text_style().font
                else {
                    unreachable!("font collection")
                };
                match local {
                    "latin" => fonts.latin = Some(text_font(e)?),
                    "ea" => fonts.east_asian = Some(text_font(e)?),
                    "cs" => fonts.complex_script = Some(text_font(e)?),
                    _ => fonts.supplemental.push(SourceSupplementalFont {
                        script: required(e, "script")?.into(),
                        typeface: required(e, "typeface")?.into(),
                    }),
                }
                self.attributes(
                    e,
                    ordinal,
                    if local == "font" {
                        &["script", "typeface"]
                    } else {
                        &["typeface", "panose", "pitchFamily", "charset"]
                    },
                );
                child = Some(Kind::Leaf);
            }
            (Kind::Text | Kind::FontRef, _)
                if [
                    "srgbClr",
                    "scrgbClr",
                    "hslClr",
                    "sysClr",
                    "schemeClr",
                    "prstClr",
                ]
                .contains(&local) =>
            {
                self.order(if parent == Kind::Text { 2 } else { 1 }, false)?;
                let value = color(e, ordinal)?;
                if parent == Kind::Text {
                    self.text_style().color = Some(value);
                } else {
                    let Some(SourceTableFontStyle::Reference { color, .. }) =
                        &mut self.text_style().font
                    else {
                        unreachable!("font reference")
                    };
                    *color = Some(value);
                }
                self.attributes(
                    e,
                    ordinal,
                    match local {
                        "scrgbClr" => &["r", "g", "b"],
                        "hslClr" => &["hue", "sat", "lum"],
                        "sysClr" => &["val", "lastClr"],
                        _ => &["val"],
                    },
                );
                child = Some(Kind::Color(parent == Kind::FontRef));
            }
            (Kind::Color(reference), _) => match transform(e) {
                Ok(t) => {
                    self.color(reference).transforms.push(t);
                    self.attributes(
                        e,
                        ordinal,
                        if ["comp", "inv", "gray", "gamma", "invGamma"].contains(&local) {
                            &[]
                        } else {
                            &["val"]
                        },
                    );
                    child = Some(Kind::Leaf);
                }
                Err(XmlError::Compatibility(_)) => (),
                Err(e) => return Err(e),
            },
            (Kind::Cell, "tcBdr") => {
                self.order(1, false)?;
                self.cell().borders = Some(SourceTableBorderStyle {
                    source_ordinal: ordinal,
                    edges: std::array::from_fn(|_| None),
                });
                child = Some(Kind::Borders);
            }
            (Kind::Borders, _)
                if [
                    "left", "right", "top", "bottom", "insideH", "insideV", "tl2br", "tr2bl",
                ]
                .contains(&local) =>
            {
                let edge = [
                    "left", "right", "top", "bottom", "insideH", "insideV", "tl2br", "tr2bl",
                ]
                .iter()
                .position(|v| *v == local)
                .expect("edge");
                self.order(edge as u8 + 1, false)?;
                child = Some(Kind::Border(edge));
            }
            (Kind::Border(edge), "ln" | "lnRef") => {
                self.order(1, false)?;
                self.capture = Some(Capture::Line {
                    edge,
                    reader: Box::new(line::Reader::new(e, depth, ordinal, line_budget, limits)?),
                });
                return Ok(());
            }
            (Kind::Cell | Kind::Background, "fill" | "fillRef") => {
                let bg = parent == Kind::Background;
                self.order(if bg { 1 } else { 2 }, false)?;
                if local == "fill" {
                    child = Some(Kind::Fill(bg));
                } else {
                    self.capture = Some(Capture::Paint {
                        background: bg,
                        effects: false,
                        reader: Box::new(paint::Reader::new(
                            e,
                            depth,
                            ordinal,
                            paint_budget,
                            limits,
                        )?),
                    });
                    return Ok(());
                }
            }
            (
                Kind::Fill(bg),
                "noFill" | "solidFill" | "gradFill" | "blipFill" | "pattFill" | "grpFill",
            ) => {
                self.order(1, false)?;
                self.capture = Some(Capture::Paint {
                    background: bg,
                    effects: false,
                    reader: Box::new(paint::Reader::new(e, depth, ordinal, paint_budget, limits)?),
                });
                return Ok(());
            }
            (Kind::Background, "effect" | "effectRef") => {
                self.order(2, false)?;
                if local == "effect" {
                    child = Some(Kind::Effect);
                } else {
                    self.capture = Some(Capture::Paint {
                        background: true,
                        effects: true,
                        reader: Box::new(paint::Reader::new(
                            e,
                            depth,
                            ordinal,
                            paint_budget,
                            limits,
                        )?),
                    });
                    return Ok(());
                }
            }
            (Kind::Effect, "effectLst" | "effectDag") => {
                self.order(1, false)?;
                self.capture = Some(Capture::Paint {
                    background: true,
                    effects: true,
                    reader: Box::new(paint::Reader::new(e, depth, ordinal, paint_budget, limits)?),
                });
                return Ok(());
            }
            (Kind::Cell, "cell3D") => {
                self.order(3, false)?;
                self.cell().cell_3d_ordinal = Some(ordinal);
            }
            (_, "extLst") => {
                self.order(255, false)?;
            }
            _ => (),
        }
        if let Some(kind) = child {
            self.frames.push(Frame { kind, rank: 0 });
            if !matches!(
                kind,
                Kind::Text | Kind::FontRef | Kind::Leaf | Kind::Color(_)
            ) {
                self.attributes(e, ordinal, &[]);
            }
        } else {
            self.retain(ordinal);
            self.opaque = Some(depth);
        }
        Ok(())
    }
    pub fn text(&self, text: &str) -> Result<(), XmlError> {
        if let Some(c) = &self.capture {
            return match c {
                Capture::Paint { reader, .. } => reader.text(text),
                Capture::Line { reader, .. } => reader.text(text),
            };
        }
        if self.opaque.is_none() && !text.trim().is_empty() {
            return Err(malformed("unexpected table style text"));
        }
        Ok(())
    }
    pub fn end(&mut self, depth: usize) -> Result<(), XmlError> {
        if let Some(c) = &mut self.capture {
            match c {
                Capture::Paint { reader, .. } if reader.depth != depth => reader.end(depth)?,
                Capture::Line { reader, .. } => reader.end(depth)?,
                _ => (),
            }
            if c.depth() != depth {
                return Ok(());
            }
            match self.capture.take().expect("style capture") {
                Capture::Line { edge, reader } => {
                    let line = match reader.finish() {
                        line::Declaration::Line(line) => SourceTableStyleLine::Direct { line },
                        line::Declaration::Reference(reference) => {
                            SourceTableStyleLine::Reference { reference }
                        }
                    };
                    self.cell().borders.as_mut().expect("borders").edges[edge] = Some(line);
                }
                Capture::Paint {
                    background,
                    effects,
                    reader,
                } => {
                    let d = reader.finish()?.publish(&mut self.value.effect_nodes)?;
                    if effects {
                        let effects = match d {
                            paint::Declaration::Effects(effects) => {
                                SourceTableStyleEffects::Direct { effects }
                            }
                            paint::Declaration::EffectReference(reference) => {
                                SourceTableStyleEffects::Reference { reference }
                            }
                            _ => return Err(malformed("table effect declaration")),
                        };
                        self.value.background.as_mut().expect("background").effects = Some(effects);
                    } else {
                        let fill = match d {
                            paint::Declaration::Fill(fill) => SourceTableStyleFill::Direct {
                                fill: Box::new(fill),
                            },
                            paint::Declaration::Reference(reference) => {
                                SourceTableStyleFill::Reference { reference }
                            }
                            _ => return Err(malformed("table fill declaration")),
                        };
                        if background {
                            self.value.background.as_mut().expect("background").fill = Some(fill);
                        } else {
                            self.cell().fill = Some(fill);
                        }
                    }
                }
            }
            return Ok(());
        }
        if let Some(d) = self.opaque {
            if d == depth {
                self.opaque = None;
            }
            return Ok(());
        }
        if self.frames.is_empty() || self.depth + self.frames.len() - 1 != depth {
            return Err(malformed("invalid table style close"));
        }
        let frame = self.frames.pop().expect("style frame");
        if matches!(frame.kind, Kind::Border(_) | Kind::Fill(_) | Kind::Effect) && frame.rank == 0 {
            return Err(malformed("missing table style value"));
        }
        if frame.kind == Kind::Fonts {
            let Some(SourceTableFontStyle::Collection { fonts, .. }) = &self.text_style().font
            else {
                unreachable!("font collection")
            };
            if fonts.latin.is_none() || fonts.east_asian.is_none() || fonts.complex_script.is_none()
            {
                return Err(malformed("incomplete table style font collection"));
            }
        }
        Ok(())
    }
    pub fn finish(self) -> Result<SourceTableStyle, XmlError> {
        if !self.frames.is_empty() || self.capture.is_some() || self.opaque.is_some() {
            return Err(malformed("unclosed table style"));
        }
        Ok(self.value)
    }
}
