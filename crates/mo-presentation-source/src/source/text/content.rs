//! Shared physical run capture for shape and cell text bodies. Style parsing
//! stays in Reader; text is stored once and ordinary-leaf offsets remain private.
use crate::{
    A,
    source::{SourceLimits, SourceRun, SourceRunKind, SourceTextConstraint, malformed},
};
use mo_xml::{Element, XmlError};
use std::collections::{BTreeMap, BTreeSet};

struct Capture {
    paragraph: usize,
    run: usize,
    ordinal: usize,
    in_alternate: bool,
    physical_children: bool,
}
pub(in crate::source) struct ContentReader {
    pub depth: usize,
    paragraphs: Vec<Vec<SourceRun>>,
    seen: BTreeSet<(usize, usize)>,
    bindings: Vec<(usize, usize, usize)>,
    current_run: Option<(usize, usize)>,
    current_paragraph: Option<usize>,
    capture: Option<Capture>,
}
pub(in crate::source) struct ContentResult {
    pub paragraphs: Vec<Vec<SourceRun>>,
    /// Body-local paragraph/run and physical a:t ordinal.
    pub bindings: Vec<(usize, usize, usize)>,
}
impl ContentReader {
    pub fn new(depth: usize) -> Self {
        Self {
            depth,
            paragraphs: vec![],
            seen: BTreeSet::new(),
            bindings: vec![],
            current_run: None,
            current_paragraph: None,
            capture: None,
        }
    }
    pub fn physical_element(&mut self) {
        if let Some(c) = &mut self.capture {
            c.physical_children = true;
        }
    }
    pub fn start(
        &mut self,
        e: &Element,
        depth: usize,
        ordinal: usize,
        in_alternate: bool,
    ) -> Result<(), XmlError> {
        if self.capture.is_some() {
            return Err(malformed("native text leaf contains an element"));
        }
        if depth == self.depth + 1 && e.name.is(A, "p") {
            self.paragraphs.push(vec![]);
            self.current_paragraph = Some(self.paragraphs.len() - 1);
        }
        if depth == self.depth + 2
            && e.name.namespace == A
            && let Some(p) = self.current_paragraph
        {
            let kind = match e.name.local.as_str() {
                "r" => Some(SourceRunKind::Text),
                "br" => Some(SourceRunKind::Break),
                "fld" => Some(SourceRunKind::Field),
                _ => None,
            };
            if let Some(kind) = kind {
                let r = self.paragraphs[p].len();
                self.paragraphs[p].push(SourceRun {
                    kind,
                    text: String::new(),
                    editable: false,
                    edit_constraint: (kind == SourceRunKind::Field)
                        .then_some(SourceTextConstraint::DynamicField),
                });
                self.current_run = Some((p, r));
            }
        }
        if depth == self.depth + 3
            && e.name.is(A, "t")
            && let Some((p, r)) = self.current_run
        {
            if self.paragraphs[p][r].kind == SourceRunKind::Break {
                return Err(malformed("text inside break"));
            }
            if !self.seen.insert((p, r)) {
                return Err(malformed("duplicate native run text"));
            }
            self.capture = Some(Capture {
                paragraph: p,
                run: r,
                ordinal,
                in_alternate,
                physical_children: false,
            });
        }
        Ok(())
    }
    pub fn text(
        &mut self,
        text: &str,
        bytes: &mut usize,
        limits: SourceLimits,
    ) -> Result<(), XmlError> {
        if let Some(c) = &self.capture {
            *bytes = bytes
                .checked_add(text.len())
                .filter(|n| *n <= limits.max_text_bytes)
                .ok_or(XmlError::Limit("source text bytes"))?;
            self.paragraphs[c.paragraph][c.run].text.push_str(text);
        }
        Ok(())
    }
    pub fn end(&mut self, depth: usize) {
        if depth == self.depth + 3
            && let Some(c) = self.capture.take()
        {
            let run = &mut self.paragraphs[c.paragraph][c.run];
            if run.kind == SourceRunKind::Text {
                run.edit_constraint = if c.in_alternate {
                    Some(SourceTextConstraint::CompatibilityBranch)
                } else if c.physical_children {
                    Some(SourceTextConstraint::StructuredLeaf)
                } else {
                    None
                };
                run.editable = run.edit_constraint.is_none();
                self.bindings.push((c.paragraph, c.run, c.ordinal));
            }
        }
        if depth == self.depth + 2 {
            self.current_run = None;
        }
        if depth == self.depth + 1 {
            self.current_paragraph = None;
        }
    }
    pub fn finish(self) -> Result<ContentResult, XmlError> {
        if self.capture.is_some() || self.paragraphs.is_empty() {
            return Err(malformed("unclosed or empty text body"));
        }
        for (p, paragraph) in self.paragraphs.iter().enumerate() {
            for (r, run) in paragraph.iter().enumerate() {
                if run.kind == SourceRunKind::Text && !self.seen.contains(&(p, r)) {
                    return Err(malformed("regular text run has no text leaf"));
                }
            }
        }
        Ok(ContentResult {
            paragraphs: self.paragraphs,
            bindings: self.bindings,
        })
    }
}

impl ContentResult {
    pub fn publish(
        self,
        part: &str,
        object: &mut crate::source::SourceObject,
        bindings: &mut BTreeMap<crate::source::SourceTextTarget, usize>,
    ) -> Result<(), XmlError> {
        if !object.paragraphs.is_empty() {
            return Err(malformed("duplicate object text content"));
        }
        for (p, r, ordinal) in self.bindings {
            let target = crate::source::SourceTextTarget {
                part: part.to_owned(),
                object_id: object.native_id,
                paragraph: p
                    .try_into()
                    .map_err(|_| XmlError::Limit("paragraph index"))?,
                run: r.try_into().map_err(|_| XmlError::Limit("run index"))?,
            };
            if bindings.insert(target, ordinal).is_some() {
                return Err(malformed("duplicate source text binding"));
            }
        }
        object.paragraphs = self.paragraphs;
        Ok(())
    }
}
