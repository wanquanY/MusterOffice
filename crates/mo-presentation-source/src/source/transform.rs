//! Native transform grammar shared by objects and the shape-tree declaration.
//! Unknown semantics retain physical locations; placement cannot silently use
//! an incomplete object transform. The package XML limits bound all input work.
use super::{SourceTransform, boolean, coordinate, integer, malformed};
use crate::A;
use mo_presentation_model::{Point, Size};
use mo_xml::{Element, XmlError};
pub(super) struct Reader {
    pub depth: usize,
    pub ordinal: u32,
    pub in_alternate: bool,
    value: SourceTransform,
    group: bool,
    rank: u8,
    retained_depth: Option<usize>,
}
impl Reader {
    pub fn new(
        e: &Element,
        depth: usize,
        ordinal: u32,
        group: bool,
        in_alternate: bool,
    ) -> Result<Self, XmlError> {
        let mut reader = Self {
            depth,
            ordinal,
            in_alternate,
            group,
            rank: 0,
            retained_depth: None,
            value: SourceTransform {
                rotation: e
                    .attribute("rot")
                    .map(|v| integer(Some(v), "rotation"))
                    .transpose()?,
                flip_horizontal: e.attribute("flipH").map(boolean).transpose()?,
                flip_vertical: e.attribute("flipV").map(boolean).transpose()?,
                ..Default::default()
            },
        };
        reader.attributes(e, ordinal, &["rot", "flipH", "flipV"]);
        Ok(reader)
    }
    fn retain(&mut self, ordinal: u32) {
        if self.value.retained_ordinals.last() != Some(&ordinal) {
            self.value.retained_ordinals.push(ordinal);
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
    ) -> Result<(), XmlError> {
        if self.retained_depth.is_some() {
            return Ok(());
        }
        if extension
            || e.name.namespace != A
            || !["off", "ext", "chOff", "chExt"].contains(&e.name.local.as_str())
        {
            self.retain(ordinal);
            self.retained_depth = Some(depth);
            return Ok(());
        }
        if depth != self.depth + 1 {
            return Err(malformed("native transform child must be a leaf"));
        }
        let (rank, child) = match e.name.local.as_str() {
            "off" => (1, false),
            "ext" => (2, false),
            "chOff" => (3, true),
            "chExt" => (4, true),
            _ => unreachable!(),
        };
        if rank <= self.rank || (child && !self.group) {
            return Err(malformed("invalid transform property order or context"));
        }
        self.rank = rank;
        if rank % 2 == 1 {
            self.attributes(e, ordinal, &["x", "y"]);
            let point = Point {
                x: coordinate(e.attribute("x"), "x")?,
                y: coordinate(e.attribute("y"), "y")?,
            };
            if child {
                self.value.child_origin = Some(point);
            } else {
                self.value.origin = Some(point);
            }
        } else {
            self.attributes(e, ordinal, &["cx", "cy"]);
            let size = Size {
                width: coordinate(e.attribute("cx"), "cx")?,
                height: coordinate(e.attribute("cy"), "cy")?,
            };
            if size.width.get() < 0 || size.height.get() < 0 {
                return Err(malformed("negative native extent"));
            }
            if child {
                self.value.child_size = Some(size);
            } else {
                self.value.size = Some(size);
            }
        }
        Ok(())
    }
    pub fn text(&self, text: &str) -> Result<(), XmlError> {
        if self.retained_depth.is_none()
            && !text.chars().all(|c| matches!(c, ' ' | '\t' | '\n' | '\r'))
        {
            return Err(malformed("text inside native transform"));
        }
        Ok(())
    }
    pub fn end(&mut self, depth: usize) {
        if self.retained_depth == Some(depth) {
            self.retained_depth = None;
        }
    }
    pub fn finish(self) -> SourceTransform {
        self.value
    }
}
