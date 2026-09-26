use super::{frame::*, *};
use crate::source::{SourceLimits, malformed};
use mo_xml::{Element, XmlError};

#[derive(Default)]
pub(in crate::source) struct Budget {
    elements: usize,
    attribute_bytes: usize,
}
impl Budget {
    fn element(&mut self, element: &Element, limits: SourceLimits) -> Result<(), XmlError> {
        self.elements = self
            .elements
            .checked_add(1)
            .ok_or(XmlError::Limit("paint elements"))?;
        for attribute in &element.attributes {
            self.attribute_bytes = self
                .attribute_bytes
                .checked_add(attribute.value.len())
                .ok_or(XmlError::Limit("paint attribute bytes"))?;
        }
        if self.elements > limits.max_paint_elements {
            return Err(XmlError::Limit("paint elements"));
        }
        if self.attribute_bytes > limits.max_paint_attribute_bytes {
            return Err(XmlError::Limit("paint attribute bytes"));
        }
        Ok(())
    }
}
pub(in crate::source) enum Declaration {
    Fill(SourceFill),
    Reference(SourceFillReference),
    Background(SourceBackground),
    Effects(SourceEffectProperties),
    EffectStyle(SourceEffectStyle),
    EffectReference(SourceEffectReference),
}
pub(in crate::source) struct Parsed {
    pub declaration: Declaration,
    pub nodes: BTreeMap<u32, SourceEffectNode>,
}
impl Parsed {
    pub fn publish(
        self,
        target: &mut BTreeMap<u32, SourceEffectNode>,
    ) -> Result<Declaration, XmlError> {
        for (id, node) in self.nodes {
            if target.insert(id, node).is_some() {
                return Err(malformed("duplicate part effect binding"));
            }
        }
        Ok(self.declaration)
    }
}
pub(in crate::source) struct Reader {
    nodes: BTreeMap<u32, SourceEffectNode>,
    pub depth: usize,
    frames: Vec<Frame>,
    retained: Vec<u32>,
    opaque_depth: Option<usize>,
}
impl Reader {
    pub fn new(
        element: &Element,
        depth: usize,
        ordinal: u32,
        budget: &mut Budget,
        limits: SourceLimits,
    ) -> Result<Self, XmlError> {
        budget.element(element, limits)?;
        if !is_fill(&element.name)
            && !element.name.is(crate::P, "blipFill")
            && !element.name.is(crate::A, "fillRef")
            && !element.name.is(crate::P, "bg")
            && !is_effect_properties(&element.name)
            && !element.name.is(crate::A, "effectStyle")
            && !element.name.is(crate::A, "effectRef")
        {
            return Err(malformed("invalid fill root"));
        }
        let mut retained = Vec::new();
        let frame = Frame::new(
            element,
            ordinal,
            &mut retained,
            element.name.is(crate::A, "effectDag"),
        )?;
        Ok(Self {
            nodes: BTreeMap::new(),
            depth,
            frames: vec![frame],
            retained,
            opaque_depth: None,
        })
    }
    pub fn start(
        &mut self,
        element: &Element,
        depth: usize,
        ordinal: u32,
        extension: bool,
        budget: &mut Budget,
        limits: SourceLimits,
    ) -> Result<(), XmlError> {
        budget.element(element, limits)?;
        if self.opaque_depth.is_some() {
            return Ok(());
        }
        if depth != self.depth + self.frames.len() {
            return Err(malformed("invalid fill nesting"));
        }
        let parent = self.frames.last_mut().expect("root frame");
        // Recognized effects/extLst obey their native sequence even when MCE
        // marks extension content. Other extension nodes remain opaque.
        if extension && !parent.recognized_opaque(&element.name) {
            retain(&mut self.retained, ordinal);
            self.opaque_depth = Some(depth);
            return Ok(());
        }
        let child = parent.child(&element.name)?;
        if matches!(child, Child::Opaque) {
            retain(&mut self.retained, ordinal);
            self.opaque_depth = Some(depth);
        } else {
            self.frames.push(Frame::new(
                element,
                ordinal,
                &mut self.retained,
                matches!(child, Child::Effect),
            )?);
        }
        Ok(())
    }
    pub fn text(&self, text: &str) -> Result<(), XmlError> {
        if self.opaque_depth.is_none() && !text.trim().is_empty() {
            return Err(malformed("unexpected fill character data"));
        }
        Ok(())
    }
    pub fn end(&mut self, depth: usize) -> Result<(), XmlError> {
        if let Some(opaque) = self.opaque_depth {
            if opaque == depth {
                self.opaque_depth = None;
            }
            return Ok(());
        }
        if depth != self.depth + self.frames.len() - 1 || self.frames.len() < 2 {
            return Err(malformed("invalid fill closing depth"));
        }
        let frame = self
            .frames
            .pop()
            .expect("checked frame")
            .close(&mut self.retained, &mut self.nodes)?;
        self.frames.last_mut().expect("parent").attach(frame)
    }
    pub fn finish(mut self) -> Result<Parsed, XmlError> {
        if self.frames.len() != 1 || self.opaque_depth.is_some() {
            return Err(malformed("unclosed fill declaration"));
        }
        let frame = self
            .frames
            .pop()
            .expect("root frame")
            .close(&mut self.retained, &mut self.nodes)?;
        let declaration = match frame.value {
            Node::Effects(value) => Ok(Declaration::Effects(value)),
            Node::FinishedEffectStyle(value) => Ok(Declaration::EffectStyle(value)),
            Node::Reference(value) if frame.name == "effectRef" => {
                Ok(Declaration::EffectReference(value.into()))
            }
            Node::Fill(value) => Ok(Declaration::Fill(value)),
            Node::Reference(value) => Ok(Declaration::Reference(value)),
            Node::Background {
                black_white_mode,
                definition: Some(definition),
            } => Ok(Declaration::Background(SourceBackground {
                source_ordinal: frame.ordinal,
                black_white_mode,
                definition,
                retained_ordinals: self.retained,
            })),
            _ => Err(malformed("invalid paint declaration root")),
        }?;
        Ok(Parsed {
            declaration,
            nodes: self.nodes,
        })
    }
}
pub(super) fn retain(retained: &mut Vec<u32>, ordinal: u32) {
    if retained.last() != Some(&ordinal) {
        retained.push(ordinal);
    }
}
