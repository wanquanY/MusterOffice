use super::{ImageAlpha, ImageResource};
use crate::{MAX_PIXEL_BYTES, RasterError, cancel};
use mo_common::Digest;
use sha2::{Digest as _, Sha256};
use std::borrow::Cow;

/// Owns validated metadata, borrows immutable pixel bytes. Reusable across
/// multiple compilations/renders without rehashing or copying each placement.
pub struct PreparedImages<'a> {
    pub(crate) descriptors: Vec<[u32; 4]>,
    bytes: Cow<'a, [u8]>,
    sha256: Digest,
}
impl<'a> PreparedImages<'a> {
    pub fn new(
        manifest: &[ImageResource],
        bytes: &'a [u8],
        check: &dyn Fn() -> bool,
    ) -> Result<Self, RasterError> {
        cancel(check)?;
        if manifest.len() > 4096 || bytes.len() > MAX_PIXEL_BYTES {
            return Err(RasterError::Limit("image resources"));
        }
        let mut end = 0usize;
        let mut descriptors = Vec::with_capacity(manifest.len());
        for r in manifest {
            cancel(check)?;
            if r.width == 0 || r.height == 0 {
                return Err(RasterError::Invalid("zero image size"));
            }
            if r.width > 8192 || r.height > 8192 {
                return Err(RasterError::Limit("image dimensions"));
            }
            let size = r.width as usize * r.height as usize * 4;
            if size > MAX_PIXEL_BYTES || end > MAX_PIXEL_BYTES - size {
                return Err(RasterError::Limit("image pixels"));
            }
            descriptors.push([
                end as u32,
                r.width,
                r.height,
                u32::from(r.alpha == ImageAlpha::Premultiplied),
            ]);
            end += size;
        }
        if end != bytes.len() {
            return Err(RasterError::Invalid("image bundle length"));
        }
        let mut bundle_hash = Sha256::new();
        for (r, d) in manifest.iter().zip(&descriptors) {
            let start = d[0] as usize;
            let size = r.width as usize * r.height as usize * 4;
            let mut hash = Sha256::new();
            for chunk in bytes[start..start + size].chunks(16384) {
                cancel(check)?;
                if r.alpha == ImageAlpha::Premultiplied
                    && chunk
                        .chunks_exact(4)
                        .any(|p| p[..3].iter().any(|v| *v > p[3]))
                {
                    return Err(RasterError::Invalid("premultiplied image channels"));
                }
                hash.update(chunk);
                bundle_hash.update(chunk);
            }
            if Digest::from_sha256(hash.finalize().into()) != r.sha256 {
                return Err(RasterError::Invalid("image resource digest"));
            }
        }
        cancel(check)?;
        Ok(Self {
            descriptors,
            bytes: Cow::Borrowed(bytes),
            sha256: Digest::from_sha256(bundle_hash.finalize().into()),
        })
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn sha256(&self) -> &Digest {
        &self.sha256
    }
    pub fn len(&self) -> usize {
        self.descriptors.len()
    }
    pub fn is_empty(&self) -> bool {
        self.descriptors.is_empty()
    }
}
impl PreparedImages<'static> {
    /// Validate once, then take ownership without cloning the pixel bundle.
    /// Private fields prevent replacing pixels behind the verified descriptor.
    pub fn owned(
        manifest: &[ImageResource],
        bytes: Vec<u8>,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, RasterError> {
        let verified = PreparedImages::new(manifest, &bytes, check)?;
        let descriptors = verified.descriptors;
        let sha256 = verified.sha256;
        Ok(Self {
            descriptors,
            bytes: Cow::Owned(bytes),
            sha256,
        })
    }
}
