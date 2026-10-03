//! Only an isolated process should instantiate this native raster backend.
#[allow(unsafe_code)]
mod ffi;
use mo_raster::{BackendReply, RasterBackend, RasterError};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
static INSTANCE: Mutex<()> = Mutex::new(());
static INVALID: AtomicBool = AtomicBool::new(false);
#[derive(Default)]
pub struct NativeRaster;
impl NativeRaster {
    pub fn is_invalid(&self) -> bool {
        INVALID.load(Ordering::Acquire)
    }
}
impl NativeRaster {
    fn call(&mut self, frame: &[u32], images: Option<&[u8]>) -> Result<BackendReply, RasterError> {
        let _guard = INSTANCE
            .lock()
            .map_err(|_| RasterError::Host("native raster lock poisoned"))?;
        if self.is_invalid() {
            return Err(RasterError::Host("native raster instance invalidated"));
        }
        let result = ffi::raster(frame, images);
        if result.is_err()
            || result
                .as_ref()
                .is_ok_and(|r| r.status == 2 || r.status == 4)
        {
            self.invalidate();
        }
        result
    }
}
impl RasterBackend for NativeRaster {
    fn pick(&mut self, frame: &[u32]) -> Result<mo_raster::picking::PickingReply, RasterError> {
        let _guard = INSTANCE
            .lock()
            .map_err(|_| RasterError::Host("native raster lock poisoned"))?;
        if self.is_invalid() {
            return Err(RasterError::Host("native raster instance invalidated"));
        }
        let result = ffi::pick(frame);
        if result.is_err() || result.as_ref().is_ok_and(|r| matches!(r.status, 2 | 4)) {
            self.invalidate();
        }
        result
    }
    fn raster(&mut self, frame: &[u32]) -> Result<BackendReply, RasterError> {
        self.call(frame, None)
    }
    fn raster_images(&mut self, frame: &[u32], images: &[u8]) -> Result<BackendReply, RasterError> {
        self.call(frame, Some(images))
    }
    fn invalidate(&mut self) {
        INVALID.store(true, Ordering::Release);
    }
}
impl mo_image::ImageDecoder for NativeRaster {
    fn decode(&mut self, encoded: &[u8]) -> Result<mo_image::DecoderReply, mo_image::ImageError> {
        self.decode_inner(encoded, None)
    }
    fn decode_sized(
        &mut self,
        encoded: &[u8],
        size: mo_image::DecodeSize,
    ) -> Result<mo_image::DecoderReply, mo_image::ImageError> {
        self.decode_inner(encoded, Some(size))
    }
    fn invalidate(&mut self) {
        RasterBackend::invalidate(self);
    }
}
impl NativeRaster {
    fn decode_inner(
        &mut self,
        encoded: &[u8],
        size: Option<mo_image::DecodeSize>,
    ) -> Result<mo_image::DecoderReply, mo_image::ImageError> {
        use mo_image::ImageError;
        let _guard = INSTANCE
            .lock()
            .map_err(|_| ImageError::Host("native component lock poisoned"))?;
        if self.is_invalid() {
            return Err(ImageError::Host("native component invalidated"));
        }
        let result = ffi::decode(encoded, size);
        if result.is_err() || result.as_ref().is_ok_and(|r| r.status == 2) {
            RasterBackend::invalidate(self);
        }
        result
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalidation_cannot_be_cleared_by_a_new_wrapper() {
        NativeRaster.invalidate();
        assert!(NativeRaster.is_invalid());
        assert!(NativeRaster.raster(&[]).is_err());
    }
}
