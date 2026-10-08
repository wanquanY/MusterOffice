//! Bounded header-only scanning over immutable samples. Slice payloads are
//! skipped, not copied into WASM. Budgets are cumulative across access units.
use crate::{
    AvcInspector, MAX_NALS_PER_ACCESS_UNIT, MAX_PARAMETER_BYTES, Picture, Result, active,
    inspector::{AccessUnit, header, length},
    invalid, limit,
};
use mo_media::{ErrorKind, MediaError, ReadAt};
use std::panic::{AssertUnwindSafe, catch_unwind};

pub struct ReadBudget {
    bytes_left: u64,
    nals_left: u32,
}
impl ReadBudget {
    pub fn new(bytes: u64, nals: u32) -> Result<Self> {
        if bytes == 0 || bytes > 128 << 20 || nals == 0 || nals > 8_000_000 {
            return Err(limit("AVC scan limits"));
        }
        Ok(Self {
            bytes_left: bytes,
            nals_left: nals,
        })
    }
    pub fn remaining_bytes(&self) -> u64 {
        self.bytes_left
    }
    pub fn remaining_nals(&self) -> u32 {
        self.nals_left
    }
    fn read(
        &mut self,
        source: &mut dyn ReadAt,
        mut at: u64,
        mut out: &mut [u8],
        check: &dyn Fn() -> bool,
    ) -> Result<()> {
        self.bytes_left = self
            .bytes_left
            .checked_sub(out.len() as u64)
            .ok_or_else(|| limit("AVC header read budget"))?;
        while !out.is_empty() {
            active(check)?;
            let count = source
                .read_at(at, out)
                .map_err(|_| MediaError::new(ErrorKind::Io, "AVC header read"))?;
            if count == 0 || count > out.len() {
                return Err(MediaError::new(
                    ErrorKind::CallbackContract,
                    "AVC read size",
                ));
            }
            at += count as u64;
            out = &mut out[count..];
        }
        Ok(())
    }
}
impl AvcInspector {
    /// Failure preserves parameter state. Read/work charges are never refunded,
    /// including after callback failure, panic or cancellation.
    pub fn inspect_at(
        &mut self,
        source: &mut dyn ReadAt,
        offset: u64,
        size: u64,
        budget: &mut ReadBudget,
        check: &dyn Fn() -> bool,
    ) -> Result<Option<Picture>> {
        catch_unwind(AssertUnwindSafe(|| {
            self.read_unit(source, offset, size, budget, check)
        }))
        .unwrap_or_else(|_| {
            Err(MediaError::new(
                ErrorKind::CallbackPanic,
                "AVC callback panic",
            ))
        })
    }
    fn read_unit(
        &mut self,
        source: &mut dyn ReadAt,
        offset: u64,
        size: u64,
        budget: &mut ReadBudget,
        check: &dyn Fn() -> bool,
    ) -> Result<Option<Picture>> {
        if size == 0 || size > 64 << 20 {
            return Err(limit("AVC access unit bytes"));
        }
        let end = offset
            .checked_add(size)
            .ok_or_else(|| invalid("AVC source extent"))?;
        let mut at = offset;
        let mut next = self.clone();
        let mut unit = AccessUnit::default();
        let mut count = 0;
        while at < end {
            active(check)?;
            count += 1;
            if count > MAX_NALS_PER_ACCESS_UNIT {
                return Err(limit("access unit NAL count"));
            }
            budget.nals_left = budget
                .nals_left
                .checked_sub(1)
                .ok_or_else(|| limit("AVC NAL scan budget"))?;
            let mut size = [0; 4];
            let size = &mut size[..usize::from(self.length_size())];
            if end - at < size.len() as u64 {
                return Err(invalid("NAL length extent"));
            }
            budget.read(source, at, size, check)?;
            at += size.len() as u64;
            let size = length(size);
            if size == 0 || size as u64 > end - at {
                return Err(invalid("NAL payload extent"));
            }
            let mut byte = [0];
            budget.read(source, at, &mut byte, check)?;
            let kind = header(&byte)?.0;
            // This is an upper bound on the prefix parsed for the supported
            // picture key (all bounded Exp-Golomb fields combined < 64 bytes).
            let needed = match kind {
                7 | 8 => size,
                1 | 5 => size.min(64),
                _ => 1,
            };
            if needed > MAX_PARAMETER_BYTES {
                return Err(limit("parameter set bytes"));
            }
            let mut prefix = vec![0; needed];
            prefix[0] = byte[0];
            budget.read(source, at + 1, &mut prefix[1..], check)?;
            next.nal(&prefix, &mut unit)?;
            at += size as u64;
        }
        active(check)?;
        *self = next;
        Ok(unit.finish())
    }
}
