//! Consume one private compiled batch after an external component execution.
use crate::{
    CompiledRaster, RasterCompletionReply, RasterError, RasterImage, RasterInfo, cancel,
    profile_for_frame,
};
use mo_common::{ByteLength, Digest};
use sha2::{Digest as _, Sha256};

impl CompiledRaster {
    /// Validates the complete reply against this exact compiled batch. The host
    /// owns execution/correlation and must quarantine the component when the
    /// returned error's invalidates_backend() is true. No partial pixels escape.
    /// This consumes the batch, preventing a second completion in safe Rust.
    pub fn complete(
        self,
        reply: impl Into<RasterCompletionReply>,
        check: &dyn Fn() -> bool,
    ) -> Result<RasterImage, RasterError> {
        cancel(check)?;
        let reply = reply.into();
        reply.check_status()?;
        if reply.pixel_bytes() != self.pixel_bytes() {
            return Err(RasterError::ComponentInvalid("pixel length"));
        }
        let reply = reply.validate(check)?;
        cancel(check)?;
        let mut frame_hash = Sha256::new();
        for words in self.frame().chunks(4096) {
            cancel(check)?;
            for word in words {
                frame_hash.update(word.to_le_bytes());
            }
        }
        cancel(check)?;
        Ok(RasterImage {
            info: RasterInfo {
                profile: profile_for_frame(self.frame()[1])
                    .ok_or(RasterError::ComponentInvalid("compiled frame version"))?
                    .into(),
                width: self.width(),
                height: self.height(),
                byte_length: ByteLength::new(reply.pixels.len() as u64),
                sha256: reply.sha256,
                frame_sha256: Digest::from_sha256(frame_hash.finalize().into()),
                work: self.work,
            },
            pixels: reply.pixels,
        })
    }
}
