use super::VisualSourceLimits;
use crate::render::{RenderError, Result, VisualFrame};
use std::collections::{BTreeMap, BTreeSet};
struct Entry {
    age: u64,
    bytes: usize,
    frame: VisualFrame,
}
/// Two ordered indexes keep reads/eviction logarithmic with no stale age queue.
#[derive(Default)]
pub(super) struct Cache {
    frames: BTreeMap<i64, Entry>,
    ages: BTreeSet<(u64, i64)>,
    clock: u64,
    pub bytes: usize,
}
impl Cache {
    fn tick(&mut self) -> Result<u64> {
        self.clock = self.clock.checked_add(1).ok_or(RenderError::Budget)?;
        Ok(self.clock)
    }
    pub fn get(&mut self, pts: i64) -> Result<Option<VisualFrame>> {
        let age = self.tick()?;
        let Some(entry) = self.frames.get_mut(&pts) else {
            return Ok(None);
        };
        self.ages.remove(&(entry.age, pts));
        entry.age = age;
        self.ages.insert((age, pts));
        Ok(Some(entry.frame.clone()))
    }
    pub fn reserve(&mut self, bytes: usize, limits: VisualSourceLimits) -> Result<()> {
        if bytes > limits.max_retained_bytes {
            return Err(RenderError::Budget);
        }
        while self.bytes > limits.max_retained_bytes - bytes
            || self.frames.len() >= limits.max_retained_frames
        {
            let Some((_, pts)) = self.ages.pop_first() else {
                return Err(RenderError::Budget);
            };
            let entry = self
                .frames
                .remove(&pts)
                .ok_or(RenderError::Invalid("color cache index"))?;
            self.bytes -= entry.bytes;
        }
        Ok(())
    }
    pub fn insert(&mut self, pts: i64, bytes: usize, frame: VisualFrame) -> Result<()> {
        let age = self.tick()?;
        if self.frames.contains_key(&pts) {
            return Err(RenderError::Invalid("duplicate color cache entry"));
        }
        self.bytes = self.bytes.checked_add(bytes).ok_or(RenderError::Budget)?;
        self.frames.insert(pts, Entry { age, bytes, frame });
        self.ages.insert((age, pts));
        Ok(())
    }
}
