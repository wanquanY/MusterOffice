//! Bounded streaming geometry grammar. Frames own only their unfinished native
//! record; completed children move into their parent without an XML DOM clone.
mod frame;
use super::*;
use crate::{
    A,
    source::{SourceLimits, malformed},
};
use frame::*;
use mo_xml::{Element, XmlError};

#[derive(Default)]
pub(in crate::source) struct Budget {
    elements: usize,
    bytes: usize,
}
impl Budget {
    fn element(&mut self, e: &Element, limits: SourceLimits) -> Result<(), XmlError> {
        self.elements = self
            .elements
            .checked_add(1)
            .ok_or(XmlError::Limit("geometry elements"))?;
        for a in &e.attributes {
            self.bytes = self
                .bytes
                .checked_add(a.value.len())
                .ok_or(XmlError::Limit("geometry attribute bytes"))?;
        }
        if self.elements > limits.max_geometry_elements {
            return Err(XmlError::Limit("geometry elements"));
        }
        if self.bytes > limits.max_geometry_attribute_bytes {
            return Err(XmlError::Limit("geometry attribute bytes"));
        }
        Ok(())
    }
}
#[derive(Default)]
struct Root {
    preset: Option<NativeShapeType>,
    adjustments: Option<SourceGeometryList<SourceGuide>>,
    guides: Option<SourceGeometryList<SourceGuide>>,
    handles: Option<SourceGeometryList<SourceAdjustHandle>>,
    connections: Option<SourceGeometryList<SourceConnectionSite>>,
    text_rect: Option<SourceGeometryRect>,
    paths: Option<SourceGeometryList<SourceGeometryPath>>,
}
pub(in crate::source) struct Reader {
    pub depth: usize,
    ordinal: u32,
    root: Root,
    rank: u8,
    frames: Vec<Frame>,
    retained: Vec<u32>,
    retained_depth: Option<usize>,
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
        let preset = if e.name.is(A, "prstGeom") {
            Some(
                required(e, "prst")?
                    .trim()
                    .to_owned()
                    .try_into()
                    .map_err(|_| malformed("invalid native preset geometry"))?,
            )
        } else {
            if !e.name.is(A, "custGeom") {
                return Err(malformed("invalid geometry root"));
            }
            None
        };
        let mut reader = Self {
            depth,
            ordinal,
            root: Root {
                preset,
                ..Default::default()
            },
            rank: 0,
            frames: Vec::new(),
            retained: Vec::new(),
            retained_depth: None,
        };
        reader.attributes(
            e,
            ordinal,
            if reader.root.preset.is_some() {
                &["prst"]
            } else {
                &[]
            },
        );
        Ok(reader)
    }
    fn retain(&mut self, ordinal: u32) {
        if self.retained.last() != Some(&ordinal) {
            self.retained.push(ordinal);
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
    pub fn start(
        &mut self,
        e: &Element,
        depth: usize,
        ordinal: u32,
        extension: bool,
        budget: &mut Budget,
        limits: SourceLimits,
    ) -> Result<(), XmlError> {
        budget.element(e, limits)?;
        if self.retained_depth.is_some() {
            return Ok(());
        }
        if extension {
            self.retain(ordinal);
            self.retained_depth = Some(depth);
            return Ok(());
        }
        if e.name.namespace != A {
            return Err(XmlError::Compatibility("unknown geometry namespace".into()));
        }
        if depth != self.depth + 1 + self.frames.len() {
            return Err(malformed("geometry nesting differs from grammar"));
        }
        let local = e.name.local.as_str();
        if let Some(parent) = self.frames.last() {
            if !parent.accepts(local) {
                return Err(malformed("invalid geometry child"));
            }
        } else {
            let rank = match local {
                "avLst" => 1,
                "gdLst" => 2,
                "ahLst" => 3,
                "cxnLst" => 4,
                "rect" => 5,
                "pathLst" => 6,
                _ => {
                    return Err(XmlError::Compatibility(
                        "unknown geometry declaration".into(),
                    ));
                }
            };
            if (self.root.preset.is_some() && rank != 1) || rank <= self.rank {
                return Err(malformed("duplicate or out of order geometry property"));
            }
            self.rank = rank;
        }
        let (frame, allowed) = Frame::new(e, ordinal)?;
        self.attributes(e, ordinal, allowed);
        self.frames.push(frame);
        Ok(())
    }
    pub fn text(&self, text: &str) -> Result<(), XmlError> {
        if self.retained_depth.is_none()
            && !text.chars().all(|c| matches!(c, ' ' | '\t' | '\n' | '\r'))
        {
            return Err(malformed("text inside native geometry"));
        }
        Ok(())
    }
    pub fn end(&mut self, depth: usize) -> Result<(), XmlError> {
        if let Some(retained) = self.retained_depth {
            if depth == retained {
                self.retained_depth = None;
            }
            return Ok(());
        }
        if depth != self.depth + self.frames.len() {
            return Err(malformed("geometry end differs from grammar"));
        }
        let completed = self
            .frames
            .pop()
            .ok_or_else(|| malformed("geometry frame missing"))?
            .finish()?;
        if let Some(parent) = self.frames.last_mut() {
            return parent.append(completed);
        }
        match completed {
            Complete::Guides(list, true) => self.root.adjustments = Some(list),
            Complete::Guides(list, false) => self.root.guides = Some(list),
            Complete::Handles(list) => self.root.handles = Some(list),
            Complete::Connections(list) => self.root.connections = Some(list),
            Complete::Rect(rect) => self.root.text_rect = Some(rect),
            Complete::Paths(list) => self.root.paths = Some(list),
            _ => return Err(malformed("invalid root geometry record")),
        }
        Ok(())
    }
    pub fn finish(self) -> Result<SourceGeometry, XmlError> {
        if !self.frames.is_empty() {
            return Err(malformed("unfinished geometry records"));
        }
        let definition = match self.root.preset {
            Some(preset) => SourceGeometryDefinition::Preset {
                preset,
                adjustments: self.root.adjustments,
            },
            None => SourceGeometryDefinition::Custom(Box::new(SourceCustomGeometry {
                adjustments: self.root.adjustments,
                guides: self.root.guides,
                handles: self.root.handles,
                connections: self.root.connections,
                text_rect: self.root.text_rect,
                paths: self
                    .root
                    .paths
                    .ok_or_else(|| malformed("custom geometry lacks path list"))?,
            })),
        };
        Ok(SourceGeometry {
            source_ordinal: self.ordinal,
            definition,
            retained_ordinals: self.retained,
        })
    }
}
