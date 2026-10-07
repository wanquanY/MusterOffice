//! PTS lookup and LRU eviction are both logarithmic; no stale queue records.
use mo_media::DecodedFrame;
use std::{collections::BTreeMap, sync::Arc};

struct Entry {
    age: u64,
    bytes: usize,
    frame: Arc<DecodedFrame>,
}
#[derive(Default)]
pub(super) struct Cache {
    by_pts: BTreeMap<i64, Entry>,
    by_age: BTreeMap<u64, i64>,
    clock: u64,
    pub bytes: usize,
}
impl Cache {
    pub fn len(&self) -> usize {
        self.by_pts.len()
    }
    pub fn get(&mut self, pts: i64) -> Result<Option<Arc<DecodedFrame>>, ()> {
        let Some(entry) = self.by_pts.get_mut(&pts) else {
            return Ok(None);
        };
        let age = self.clock.checked_add(1).ok_or(())?;
        self.clock = age;
        self.by_age.remove(&entry.age);
        entry.age = age;
        self.by_age.insert(age, pts);
        Ok(Some(entry.frame.clone()))
    }
    pub fn evict(&mut self) -> bool {
        let Some((_, pts)) = self.by_age.pop_first() else {
            return false;
        };
        if let Some(entry) = self.by_pts.remove(&pts) {
            self.bytes -= entry.bytes;
        }
        true
    }
    pub fn insert(&mut self, pts: i64, bytes: usize, frame: Arc<DecodedFrame>) -> bool {
        let Some(age) = self.clock.checked_add(1) else {
            return false;
        };
        let prior_bytes = self.by_pts.get(&pts).map_or(0, |e| e.bytes);
        let Some(total) = self
            .bytes
            .checked_sub(prior_bytes)
            .and_then(|n| n.checked_add(bytes))
        else {
            return false;
        };
        if let Some(prior) = self.by_pts.remove(&pts) {
            self.by_age.remove(&prior.age);
        }
        self.clock = age;
        self.bytes = total;
        self.by_age.insert(age, pts);
        self.by_pts.insert(pts, Entry { age, bytes, frame });
        true
    }
}
