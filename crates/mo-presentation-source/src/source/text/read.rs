use super::{grammar::*, *};
use crate::{
    A,
    source::{SourceLimits, malformed, paint},
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
            .ok_or(XmlError::Limit("text style elements"))?;
        for a in &e.attributes {
            self.attribute_bytes = self
                .attribute_bytes
                .checked_add(a.value.len())
                .ok_or(XmlError::Limit("text style attribute bytes"))?;
        }
        if self.elements > limits.max_text_style_elements {
            return Err(XmlError::Limit("text style elements"));
        }
        if self.attribute_bytes > limits.max_text_style_attribute_bytes {
            return Err(XmlError::Limit("text style attribute bytes"));
        }
        Ok(())
    }
}
struct Frame {
    ordinal: u32,
    node: SourceTextNode,
    rank: u8,
}
enum Delegate {
    Paint(paint::Reader),
    Line(Box<super::super::line::Reader>),
    Color {
        color: SourceColor,
        opaque: Option<usize>,
    },
}
struct Capture {
    depth: usize,
    ordinal: u32,
    node: SourceTextNode,
    reader: Delegate,
}
pub(in crate::source) struct Reader {
    pub depth: usize,
    catalog: SourceTextCatalog,
    frames: Vec<Frame>,
    opaque: Option<usize>,
    capture: Option<Capture>,
}
impl Reader {
    pub fn cell_body(
        e: &Element,
        depth: usize,
        ordinal: u32,
        owner: u32,
        cell: crate::source::table::SourceCellAddress,
        budget: &mut Budget,
        limits: SourceLimits,
    ) -> Result<Self, XmlError> {
        if !e.name.is(A, "txBody") {
            return Err(malformed("invalid table text body root"));
        }
        let mut reader = Self::new(e, depth, ordinal, Some(owner), budget, limits)?;
        reader.catalog.roots[0].cell = Some(cell);
        Ok(reader)
    }
    pub fn new(
        e: &Element,
        depth: usize,
        ordinal: u32,
        owner: Option<u32>,
        budget: &mut Budget,
        limits: SourceLimits,
    ) -> Result<Self, XmlError> {
        budget.element(e, limits)?;
        let node = node(e, ordinal, None)?;
        if !matches!(
            node.element,
            NativeTextElement::TxBody
                | NativeTextElement::TxStyles
                | NativeTextElement::DefaultTextStyle
                | NativeTextElement::FontRef
                | NativeTextElement::BodyPr
                | NativeTextElement::LstStyle
        ) {
            return Err(malformed("invalid text style root"));
        }
        Ok(Self {
            depth,
            catalog: SourceTextCatalog {
                roots: vec![SourceTextRoot {
                    cell: None,
                    source_ordinal: ordinal,
                    owner,
                }],
                ..Default::default()
            },
            frames: vec![Frame {
                ordinal,
                node,
                rank: 0,
            }],
            opaque: None,
            capture: None,
        })
    }
    #[allow(clippy::too_many_arguments)]
    pub fn start(
        &mut self,
        e: &Element,
        depth: usize,
        ordinal: u32,
        extension: bool,
        budget: &mut Budget,
        line_budget: &mut super::super::line::Budget,
        paint_budget: &mut paint::Budget,
        limits: SourceLimits,
    ) -> Result<(), XmlError> {
        budget.element(e, limits)?;
        if let Some(c) = &mut self.capture {
            return match &mut c.reader {
                Delegate::Paint(r) => r.start(e, depth, ordinal, extension, paint_budget, limits),
                Delegate::Line(r) => r.start(
                    e,
                    depth,
                    ordinal,
                    extension,
                    line_budget,
                    paint_budget,
                    limits,
                ),
                Delegate::Color { color, opaque } => {
                    if opaque.is_some() {
                        return Ok(());
                    }
                    if extension {
                        retain(&mut c.node.retained_ordinals, ordinal);
                        *opaque = Some(depth);
                        return Ok(());
                    }
                    if depth != c.depth + 1 {
                        return Err(malformed("nested text color transform"));
                    }
                    let t = transform(e)?;
                    retain_attributes(
                        e,
                        if matches!(
                            e.name.local.as_str(),
                            "comp" | "inv" | "gray" | "gamma" | "invGamma"
                        ) {
                            &[]
                        } else {
                            &["val"]
                        },
                        ordinal,
                        &mut c.node.retained_ordinals,
                    );
                    color.transforms.push(t);
                    Ok(())
                }
            };
        }
        if self.opaque.is_some() {
            return Ok(());
        }
        if depth != self.depth + self.frames.len() {
            return Err(malformed("native text nesting"));
        }
        let parent = self.frames.last_mut().expect("text root");
        let spec = child(parent.node.element, e)?;
        if spec.rank != 0 {
            if spec.rank < parent.rank || (spec.rank == parent.rank && !spec.repeated) {
                return Err(malformed("duplicate or out-of-order native text child"));
            }
            parent.rank = spec.rank;
        }
        if spec.opaque || extension {
            retain(&mut parent.node.retained_ordinals, ordinal);
            self.opaque = Some(depth);
            return Ok(());
        }
        let element: NativeTextElement = enumeration(&e.name.local)?;
        let delegate = if e.name.is(A, "ln") || e.name.is(A, "uLn") {
            Some(Delegate::Line(Box::new(super::super::line::Reader::new(
                e,
                depth,
                ordinal,
                line_budget,
                limits,
            )?)))
        } else if super::super::fill::is_fill(&e.name) || paint::is_effect_properties(&e.name) {
            Some(Delegate::Paint(paint::Reader::new(
                e,
                depth,
                ordinal,
                paint_budget,
                limits,
            )?))
        } else if matches!(
            element,
            NativeTextElement::SrgbClr
                | NativeTextElement::ScrgbClr
                | NativeTextElement::HslClr
                | NativeTextElement::SysClr
                | NativeTextElement::SchemeClr
                | NativeTextElement::PrstClr
        ) {
            Some(Delegate::Color {
                color: color(e, ordinal)?,
                opaque: None,
            })
        } else {
            None
        };
        let mut n = if delegate.is_some() {
            SourceTextNode {
                element,
                parent: Some(parent.ordinal),
                children: vec![],
                value: SourceTextValue::Container {},
                retained_ordinals: vec![],
            }
        } else {
            node(e, ordinal, Some(parent.ordinal))?
        };
        parent.node.children.push(ordinal);
        if let Some(reader) = delegate {
            if matches!(reader, Delegate::Color { .. }) {
                retain_attributes(
                    e,
                    match element {
                        NativeTextElement::ScrgbClr => &["r", "g", "b"],
                        NativeTextElement::HslClr => &["hue", "sat", "lum"],
                        NativeTextElement::SysClr => &["val", "lastClr"],
                        _ => &["val"],
                    },
                    ordinal,
                    &mut n.retained_ordinals,
                );
            }
            self.capture = Some(Capture {
                depth,
                ordinal,
                node: n,
                reader,
            });
        } else {
            self.frames.push(Frame {
                ordinal,
                node: n,
                rank: 0,
            });
        }
        Ok(())
    }
    pub fn text(&self, text: &str) -> Result<(), XmlError> {
        if let Some(c) = &self.capture {
            return match &c.reader {
                Delegate::Paint(r) => r.text(text),
                Delegate::Line(r) => r.text(text),
                Delegate::Color { opaque, .. } => {
                    if opaque.is_some() || text.trim().is_empty() {
                        Ok(())
                    } else {
                        Err(malformed("unexpected text color content"))
                    }
                }
            };
        }
        if self.opaque.is_none()
            && self
                .frames
                .last()
                .is_none_or(|f| f.node.element != NativeTextElement::T)
            && !text.trim().is_empty()
        {
            return Err(malformed("text outside native text leaf"));
        }
        Ok(())
    }
    pub fn end(&mut self, depth: usize) -> Result<(), XmlError> {
        if self.capture.as_ref().is_some_and(|c| c.depth == depth) {
            let mut c = self.capture.take().expect("checked capture");
            c.node.value = match c.reader {
                Delegate::Paint(r) => match r.finish()?.publish(&mut self.catalog.effect_nodes)? {
                    paint::Declaration::Fill(fill) => SourceTextValue::Fill { fill: fill.into() },
                    paint::Declaration::Effects(effects) => SourceTextValue::Effects { effects },
                    _ => return Err(malformed("invalid text paint declaration")),
                },
                Delegate::Line(r) => match r.finish() {
                    super::super::line::Declaration::Line(line) => {
                        SourceTextValue::Line { line: line.into() }
                    }
                    _ => return Err(malformed("invalid text line declaration")),
                },
                Delegate::Color {
                    color,
                    opaque: None,
                } => SourceTextValue::Color {
                    color: color.into(),
                },
                _ => return Err(malformed("unclosed text color")),
            };
            if self.catalog.nodes.insert(c.ordinal, c.node).is_some() {
                return Err(malformed("duplicate text source ordinal"));
            }
            return Ok(());
        }
        if let Some(c) = &mut self.capture {
            return match &mut c.reader {
                Delegate::Paint(r) => r.end(depth),
                Delegate::Line(r) => r.end(depth),
                Delegate::Color { opaque, .. } => {
                    if *opaque == Some(depth) {
                        *opaque = None;
                    }
                    Ok(())
                }
            };
        }
        if let Some(d) = self.opaque {
            if d == depth {
                self.opaque = None;
            }
            return Ok(());
        }
        if self.frames.is_empty() || depth != self.depth + self.frames.len() - 1 {
            return Err(malformed("invalid text closing depth"));
        }
        let f = self.frames.pop().expect("text frame");
        close(&f.node, &self.catalog.nodes)?;
        if self.catalog.nodes.insert(f.ordinal, f.node).is_some() {
            return Err(malformed("duplicate text source ordinal"));
        }
        Ok(())
    }
    pub fn finish(self, target: &mut SourceTextCatalog) -> Result<(), XmlError> {
        let mut roots = CatalogRoots::existing(target)?;
        self.finish_indexed(target, &mut roots)
    }
    pub fn finish_indexed(
        self,
        target: &mut SourceTextCatalog,
        roots: &mut CatalogRoots,
    ) -> Result<(), XmlError> {
        if !self.frames.is_empty() || self.capture.is_some() || self.opaque.is_some() {
            return Err(malformed("unclosed native text declarations"));
        }
        roots.append(target, self.catalog)
    }
}
