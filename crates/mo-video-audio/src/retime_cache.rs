//! Bounded indexed LRU. Large projects must not linearly scan every retained
//! clip for each audio span. Host-held requests do not pin cache membership.
use crate::{AudioError, Result, RetimePlan, retime_plan::CompiledRetime};
use mo_video_model::ClipId;
use std::{collections::BTreeMap, sync::Arc};

struct Entry {
    age: u64,
    value: CompiledRetime,
}
#[derive(Default)]
pub(crate) struct RetimeCache {
    entries: BTreeMap<ClipId, Entry>,
    order: BTreeMap<u64, ClipId>,
    clock: u64,
    bytes: usize,
}
impl RetimeCache {
    pub fn retained_bytes(&self) -> usize {
        self.bytes
    }
    fn tick(&mut self) -> Result<u64> {
        self.clock = self.clock.checked_add(1).ok_or(AudioError::Budget)?;
        Ok(self.clock)
    }
    /// Outer None is a cache miss; inner None is a known run without DSP.
    pub fn lookup(&mut self, clip: &ClipId, first: i64) -> Result<Option<Option<Arc<RetimePlan>>>> {
        let age = self.tick()?;
        let Some(entry) = self.entries.get_mut(clip) else {
            return Ok(None);
        };
        self.order.remove(&entry.age);
        entry.age = age;
        self.order.insert(age, clip.clone());
        let right = entry
            .value
            .plans
            .partition_point(|p| p.first_sample() <= first);
        Ok(Some(
            right
                .checked_sub(1)
                .and_then(|i| entry.value.plans.get(i))
                .filter(|p| first < p.first_sample() + p.output_frames())
                .cloned(),
        ))
    }
    /// Admission occurs before schedule allocation, even when a host still
    /// retains Arc requests whose independent memory is outside this cache.
    pub fn admit(&mut self, charge: usize, maximum: usize) -> Result<()> {
        if charge > maximum {
            return Err(AudioError::Budget);
        }
        while self.bytes.checked_add(charge).is_none_or(|n| n > maximum) {
            let (_, id) = self.order.pop_first().ok_or(AudioError::Budget)?;
            let entry = self
                .entries
                .remove(&id)
                .ok_or(AudioError::Invalid("retime cache index"))?;
            self.bytes -= entry.value.charge;
        }
        Ok(())
    }
    pub fn insert(&mut self, id: ClipId, value: CompiledRetime) -> Result<()> {
        let age = self.tick()?;
        if self.entries.contains_key(&id) {
            return Err(AudioError::Invalid("duplicate retime cache entry"));
        }
        self.bytes = self
            .bytes
            .checked_add(value.charge)
            .ok_or(AudioError::Budget)?;
        self.order.insert(age, id.clone());
        self.entries.insert(id, Entry { age, value });
        Ok(())
    }
}
