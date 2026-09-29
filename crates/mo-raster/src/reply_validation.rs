//! Owned pixel validation. No unchecked bytes or caller-supplied digest can be
//! promoted to a validated reply; abandoning the cursor releases its pixels.
use crate::{BackendReply, MAX_PIXEL_BYTES, RasterError, cancel};
use mo_common::Digest;
use sha2::{Digest as _, Sha256};

pub const VALIDATION_BYTES_PER_UNIT: usize = 16_384;

pub struct ValidatedRasterReply {
    pub(crate) pixels: Vec<u8>,
    pub(crate) sha256: Digest,
}

/// A raw reply or an owned proof produced by the shared validator. The sealed
/// proof covers pixels only; completion still checks dimensions and frame/owner.
pub enum RasterCompletionReply {
    Unchecked(BackendReply),
    Validated(ValidatedRasterReply),
}
impl From<BackendReply> for RasterCompletionReply {
    fn from(reply: BackendReply) -> Self {
        Self::Unchecked(reply)
    }
}
impl From<ValidatedRasterReply> for RasterCompletionReply {
    fn from(reply: ValidatedRasterReply) -> Self {
        Self::Validated(reply)
    }
}
impl RasterCompletionReply {
    pub fn check_status(&self) -> Result<(), RasterError> {
        match self {
            Self::Unchecked(reply) => reply.check_status(),
            Self::Validated(_) => Ok(()),
        }
    }
    pub(crate) fn pixel_bytes(&self) -> usize {
        match self {
            Self::Unchecked(reply) => reply.pixels.len(),
            Self::Validated(reply) => reply.pixels.len(),
        }
    }
    pub(crate) fn validate(
        self,
        check: &dyn Fn() -> bool,
    ) -> Result<ValidatedRasterReply, RasterError> {
        match self {
            Self::Unchecked(reply) => {
                let mut validation = RasterReplyValidation::new(reply, check)?;
                while !validation.step(4096, check)? {}
                validation.take()
            }
            Self::Validated(reply) => Ok(reply),
        }
    }
}

pub struct RasterReplyValidation {
    pixels: Option<Vec<u8>>,
    cursor: usize,
    hash: Sha256,
}
impl RasterReplyValidation {
    pub fn new(reply: BackendReply, check: &dyn Fn() -> bool) -> Result<Self, RasterError> {
        cancel(check)?;
        reply.check_status()?;
        if reply.pixels.len() > MAX_PIXEL_BYTES || !reply.pixels.len().is_multiple_of(4) {
            return Err(RasterError::ComponentInvalid("pixel length"));
        }
        Ok(Self {
            pixels: Some(reply.pixels),
            cursor: 0,
            hash: Sha256::new(),
        })
    }
    /// Each unit checks premultiplication and hashes at most 16 KiB. This is a
    /// work bound, not a time guarantee. Failure is terminal; drop cancels safely.
    pub fn step(&mut self, work_units: u32, check: &dyn Fn() -> bool) -> Result<bool, RasterError> {
        if !(1..=4096).contains(&work_units) {
            return Err(RasterError::Invalid("validation work units"));
        }
        let result = self.advance(work_units, check);
        if result.is_err() {
            self.pixels = None;
        }
        result
    }
    fn advance(&mut self, work_units: u32, check: &dyn Fn() -> bool) -> Result<bool, RasterError> {
        cancel(check)?;
        let pixels = self
            .pixels
            .as_ref()
            .ok_or(RasterError::Host("validation closed"))?;
        for _ in 0..work_units {
            if self.cursor == pixels.len() {
                break;
            }
            cancel(check)?;
            let end = pixels.len().min(self.cursor + VALIDATION_BYTES_PER_UNIT);
            let chunk = &pixels[self.cursor..end];
            if chunk
                .chunks_exact(4)
                .any(|p| p[..3].iter().any(|v| *v > p[3]))
            {
                return Err(RasterError::ComponentInvalid("premultiplied channels"));
            }
            self.hash.update(chunk);
            self.cursor = end;
        }
        cancel(check)?;
        Ok(self.cursor == pixels.len())
    }
    /// Consumes the only copy of the proof. An incomplete/failed cursor cannot
    /// produce pixels, a digest, or a partial successful result.
    pub fn take(self) -> Result<ValidatedRasterReply, RasterError> {
        let pixels = self.pixels.ok_or(RasterError::Host("validation closed"))?;
        if self.cursor != pixels.len() {
            return Err(RasterError::Host("validation incomplete"));
        }
        Ok(ValidatedRasterReply {
            pixels,
            sha256: Digest::from_sha256(self.hash.finalize().into()),
        })
    }
}
