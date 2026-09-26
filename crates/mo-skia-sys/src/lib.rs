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
        use mo_image::ImageError;
        let _guard = INSTANCE
            .lock()
            .map_err(|_| ImageError::Host("native component lock poisoned"))?;
        if self.is_invalid() {
            return Err(ImageError::Host("native component invalidated"));
        }
        let result = ffi::decode(encoded);
        if result.is_err() || result.as_ref().is_ok_and(|r| r.status == 2) {
            RasterBackend::invalidate(self);
        }
        result
    }
    fn invalidate(&mut self) {
        RasterBackend::invalidate(self);
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
