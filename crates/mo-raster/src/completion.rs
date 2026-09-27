//! Consume one private compiled batch after an external component execution.
use crate::{
    BackendReply, CompiledRaster, RasterError, RasterImage, RasterInfo, cancel, profile_for_frame,
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
        reply: BackendReply,
        check: &dyn Fn() -> bool,
    ) -> Result<RasterImage, RasterError> {
        cancel(check)?;
        reply.check_status()?;
        if reply.pixels.len() != self.pixel_bytes() {
            return Err(RasterError::ComponentInvalid("pixel length"));
        }
        let mut hash = Sha256::new();
        for chunk in reply.pixels.chunks(16384) {
            cancel(check)?;
            if chunk
                .chunks_exact(4)
                .any(|p| p[..3].iter().any(|v| *v > p[3]))
            {
                return Err(RasterError::ComponentInvalid("premultiplied channels"));
            }
            hash.update(chunk);
        }
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
                sha256: Digest::from_sha256(hash.finalize().into()),
                frame_sha256: Digest::from_sha256(frame_hash.finalize().into()),
                work: self.work,
            },
            pixels: reply.pixels,
        })
    }
}
