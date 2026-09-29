//! Bounded local geometry owned by one immutable author playback plan. This is
//! derived computation state, never a document store or a cache of device frames.
use crate::{CompileError, shape_paths::Outline};
use mo_common::{Digest, SlideId};
use mo_geometry::Point;
use mo_presentation_model::Size;
use std::{collections::BTreeMap, mem::size_of, ops::Deref};

const MAX_ENTRIES: usize = 1024;
const MAX_PAYLOAD_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Key {
    /// Stable preorder position within the plan's one document and slide.
    pub object: u32,
    pub size: Size,
    pub anchor: Point,
    pub segments: u32,
}
struct Entry {
    key: Key,
    outline: Outline,
    bytes: usize,
}
pub(crate) enum LocalOutline<'a> {
    Retained(&'a Outline),
    Transient(Outline),
}
impl Deref for LocalOutline<'_> {
    type Target = Outline;
    fn deref(&self) -> &Outline {
        match self {
            Self::Retained(outline) => outline,
            Self::Transient(outline) => outline,
        }
    }
}
pub(crate) struct PageGeometry {
    owner: Digest,
    slide: SlideId,
    entries: BTreeMap<u32, Entry>,
    payload_bytes: usize,
}
impl PageGeometry {
    pub(crate) fn new(owner: Digest, slide: SlideId) -> Self {
        Self {
            owner,
            slide,
            entries: BTreeMap::new(),
            payload_bytes: 0,
        }
    }
    pub(crate) fn verify_owner(&self, owner: &Digest, slide: &SlideId) -> Result<(), CompileError> {
        if &self.owner != owner || &self.slide != slide {
            return Err(CompileError::Invalid("retained geometry document identity"));
        }
        Ok(())
    }
    pub(crate) fn outline(
        &mut self,
        key: Key,
        check: &dyn Fn() -> bool,
        build: impl FnOnce() -> Result<Outline, CompileError>,
    ) -> Result<LocalOutline<'_>, CompileError> {
        crate::cancel(check)?;
        if self
            .entries
            .get(&key.object)
            .is_some_and(|entry| entry.key == key)
        {
            return Ok(LocalOutline::Retained(&self.entries[&key.object].outline));
        }
        let outline = build()?;
        crate::cancel(check)?;
        let bytes = outline
            .commands
            .capacity()
            .checked_mul(size_of::<mo_geometry::PathCommand>())
            .and_then(|n| n.checked_add(size_of::<Entry>() + size_of::<u32>()));
        let prior = self.entries.get(&key.object).map_or(0, |entry| entry.bytes);
        let Some(total) = bytes.and_then(|n| (self.payload_bytes - prior).checked_add(n)) else {
            return Ok(LocalOutline::Transient(outline));
        };
        // One retained precision per object. When full, keep admitted objects
        // useful on later frames instead of cycling every page through an LRU.
        // Oversized or non-admitted geometry is still computed at full quality.
        if total > MAX_PAYLOAD_BYTES || prior == 0 && self.entries.len() == MAX_ENTRIES {
            return Ok(LocalOutline::Transient(outline));
        }
        self.entries.insert(
            key.object,
            Entry {
                key,
                outline,
                bytes: bytes.expect("checked above"),
            },
        );
        self.payload_bytes = total;
        Ok(LocalOutline::Retained(&self.entries[&key.object].outline))
    }
}

#[cfg(test)]
mod tests;
