use mo_raster::{
    BackendReply, MAX_CLIP_FRAME_WORDS, MAX_COMPOSITE_FRAME_WORDS,
    MAX_ELLIPTIC_GRADIENT_FRAME_WORDS, MAX_FRAME_WORDS, MAX_GRADIENT_PLANE_FRAME_WORDS,
    MAX_IMAGE_FRAME_WORDS, MAX_OPACITY_GROUP_FRAME_WORDS, MAX_PIXEL_BYTES,
    MAX_RECT_GRADIENT_FRAME_WORDS, RasterError,
};
use std::{ffi::c_void, ptr, slice};
unsafe extern "C" {
    fn mo_skia_abi() -> u32;
    fn mo_skia_pick_abi() -> u32;
    fn mo_skia_pick(
        request: *const u32,
        words: u32,
        output: *mut *mut u32,
        output_words: *mut u32,
    ) -> i32;
    fn mo_skia_clips_abi() -> u32;
    fn mo_skia_gradient_planes_abi() -> u32;
    fn mo_skia_office_gradients_abi() -> u32;
    fn mo_skia_rect_gradients_abi() -> u32;
    fn mo_skia_elliptic_gradients_abi() -> u32;
    fn mo_skia_opacity_groups_abi() -> u32;
    fn mo_skia_snapshot_scopes_abi() -> u32;
    fn mo_skia_compositing_abi() -> u32;
    fn mo_skia_raster(
        request: *const u32,
        words: u32,
        output: *mut *mut u8,
        bytes: *mut u32,
    ) -> i32;
    fn mo_skia_images_abi() -> u32;
    fn mo_skia_raster_images(
        request: *const u32,
        words: u32,
        images: *const u8,
        image_bytes: u32,
        output: *mut *mut u8,
        bytes: *mut u32,
    ) -> i32;
    fn mo_skia_free(pixels: *mut c_void);
    fn mo_image_decode_abi() -> u32;
    fn mo_image_decode_sized_abi() -> u32;
    fn mo_image_decode_sized(
        encoded: *const u8,
        length: u32,
        min_width: u32,
        min_height: u32,
        pixels: *mut *mut u8,
        info: *mut u32,
    ) -> i32;
}
pub(super) fn decode(
    encoded: &[u8],
    size: Option<mo_image::DecodeSize>,
) -> Result<mo_image::DecoderReply, mo_image::ImageError> {
    use mo_image::{DecoderReply, ImageError, MAX_ENCODED_BYTES, MAX_PIXEL_BYTES};
    if encoded.len() > MAX_ENCODED_BYTES {
        return Err(ImageError::Limit("native encoded bytes"));
    }
    // SAFETY: pure ABI query; the caller serializes this pinned component.
    if unsafe { mo_image_decode_abi() } != 1 || unsafe { mo_image_decode_sized_abi() } != 1 {
        return Err(ImageError::Host("decode ABI mismatch"));
    }
    let mut pointer = ptr::null_mut();
    let mut words = [0; 9];
    // SAFETY: bounded immutable bytes and distinct valid output slots. No input
    // references survive the synchronous call. Output uses the shared allocator.
    let status = unsafe {
        mo_image_decode_sized(
            encoded.as_ptr(),
            encoded.len() as u32,
            size.map_or(0, |s| s.width),
            size.map_or(0, |s| s.height),
            &mut pointer,
            words.as_mut_ptr(),
        )
    };
    let _owned = Output(pointer);
    if !(0..=5).contains(&status)
        || status == 4
        || (status != 0 && (!pointer.is_null() || words != [0; 9]))
    {
        return Err(ImageError::ComponentInvalid("native decode ownership"));
    }
    let bytes = words[8] as usize;
    if status != 0 {
        return Ok(DecoderReply {
            status: status as u32,
            words,
            pixels: vec![],
        });
    }
    if pointer.is_null() || bytes == 0 || bytes > MAX_PIXEL_BYTES {
        return Err(ImageError::ComponentInvalid("native decode allocation"));
    }
    // SAFETY: a successful pinned ABI allocation contains exactly words[8]
    // initialized bytes, held by the guard throughout this copy.
    let data = unsafe { slice::from_raw_parts(pointer, bytes) };
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(bytes)
        .map_err(|_| ImageError::Host("native decode allocation"))?;
    pixels.extend_from_slice(data);
    Ok(DecoderReply {
        status: 0,
        words,
        pixels,
    })
}
struct Output(*mut u8);
impl Drop for Output {
    fn drop(&mut self) {
        // SAFETY: owned output from the pinned component; null is accepted.
        unsafe {
            mo_skia_free(self.0.cast());
        }
    }
}
pub(super) fn raster(frame: &[u32], images: Option<&[u8]>) -> Result<BackendReply, RasterError> {
    if frame.len()
        > if frame.get(1) == Some(&14) {
            mo_raster::MAX_SNAPSHOT_SCOPE_FRAME_WORDS
        } else if frame.get(1) == Some(&13) {
            MAX_OPACITY_GROUP_FRAME_WORDS
        } else if frame.get(1) == Some(&12) {
            MAX_ELLIPTIC_GRADIENT_FRAME_WORDS
        } else if frame.get(1) == Some(&11) {
            MAX_RECT_GRADIENT_FRAME_WORDS
        } else if matches!(frame.get(1), Some(9 | 10)) {
            MAX_GRADIENT_PLANE_FRAME_WORDS
        } else if frame.get(1) == Some(&8) {
            MAX_COMPOSITE_FRAME_WORDS
        } else if frame.get(1) == Some(&7) {
            MAX_CLIP_FRAME_WORDS
        } else if images.is_some() {
            MAX_IMAGE_FRAME_WORDS
        } else {
            MAX_FRAME_WORDS
        }
    {
        return Err(RasterError::Limit("native frame words"));
    }
    // SAFETY: pure no-argument ABI query.
    if unsafe { mo_skia_abi() } != 4 {
        return Err(RasterError::Host("raster ABI mismatch"));
    }
    // SAFETY: pure no-argument query on the pinned component.
    if frame.get(1) == Some(&14) && unsafe { mo_skia_snapshot_scopes_abi() } != 1 {
        return Err(RasterError::Host("raster snapshot scope ABI mismatch"));
    }
    // SAFETY: pure no-argument query on the pinned component.
    if matches!(frame.get(1), Some(13 | 14)) && unsafe { mo_skia_opacity_groups_abi() } != 1 {
        return Err(RasterError::Host("raster opacity group ABI mismatch"));
    }
    // SAFETY: pure no-argument query on the pinned component.
    if frame.get(1) == Some(&7) && unsafe { mo_skia_clips_abi() } != 1 {
        return Err(RasterError::Host("raster clip ABI mismatch"));
    }
    // SAFETY: pure no-argument query on the pinned component.
    if frame.get(1) == Some(&8) && unsafe { mo_skia_compositing_abi() } != 1 {
        return Err(RasterError::Host("raster compositing ABI mismatch"));
    }
    // SAFETY: pure no-argument query on the pinned component.
    if frame.get(1) == Some(&12) && unsafe { mo_skia_elliptic_gradients_abi() } != 1 {
        return Err(RasterError::Host("raster elliptic gradient ABI mismatch"));
    }
    // SAFETY: pure no-argument query on the pinned component.
    if frame.get(1) == Some(&11) && unsafe { mo_skia_rect_gradients_abi() } != 1 {
        return Err(RasterError::Host(
            "raster rectangular gradient ABI mismatch",
        ));
    }
    // SAFETY: pure no-argument query on the pinned component.
    if frame.get(1) == Some(&10) && unsafe { mo_skia_office_gradients_abi() } != 1 {
        return Err(RasterError::Host(
            "raster Office gradient extension unavailable",
        ));
    }
    if frame.get(1) == Some(&9) && unsafe { mo_skia_gradient_planes_abi() } != 1 {
        return Err(RasterError::Host("raster gradient plane ABI mismatch"));
    }
    if let Some(images) = images {
        if images.len() > MAX_PIXEL_BYTES {
            return Err(RasterError::Limit("native image bytes"));
        }
        // SAFETY: pure no-argument extension query.
        if ![1, 2].contains(&unsafe { mo_skia_images_abi() }) {
            return Err(RasterError::Host("raster image ABI mismatch"));
        }
    }
    let mut pointer = ptr::null_mut();
    let mut bytes = 0;
    // SAFETY: aligned borrowed request, checked word count, distinct valid output
    // slots; the pinned component retains no input. Access is serialized above.
    let status = unsafe {
        match images {
            Some(images) => mo_skia_raster_images(
                frame.as_ptr(),
                frame.len() as u32,
                images.as_ptr(),
                images.len() as u32,
                &mut pointer,
                &mut bytes,
            ),
            None => mo_skia_raster(frame.as_ptr(), frame.len() as u32, &mut pointer, &mut bytes),
        }
    };
    let _owned = Output(pointer);
    if !(0..=4).contains(&status) || (status != 0 && (!pointer.is_null() || bytes != 0)) {
        return Err(RasterError::ComponentInvalid("native status or ownership"));
    }
    if status != 0 {
        return Ok(BackendReply {
            status: status as u32,
            pixels: vec![],
        });
    }
    if pointer.is_null() || bytes == 0 || bytes as usize > MAX_PIXEL_BYTES {
        return Err(RasterError::ComponentInvalid("native pixel bytes"));
    }
    // SAFETY: successful pinned ABI owns exactly `bytes` initialized pixel bytes;
    // the guard retains the allocation throughout the copy.
    let data = unsafe { slice::from_raw_parts(pointer, bytes as usize) };
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(data.len())
        .map_err(|_| RasterError::Host("native pixels allocation"))?;
    pixels.extend_from_slice(data);
    Ok(BackendReply { status: 0, pixels })
}

/// The caller holds the sole native component lock for this complete call.
pub(super) fn pick(frame: &[u32]) -> Result<mo_raster::picking::PickingReply, RasterError> {
    use mo_raster::picking::*;
    if frame.len() > MAX_PICK_FRAME_WORDS {
        return Err(RasterError::Limit("native picking frame"));
    }
    // SAFETY: pure capability query on the serialized, pinned instance.
    if unsafe { mo_skia_pick_abi() } != 1 {
        return Err(RasterError::Host("picking ABI mismatch"));
    }
    let mut pointer = ptr::null_mut();
    let mut words = 0;
    // SAFETY: input is immutable for this synchronous call; outputs are distinct
    // initialized slots. Successful allocation is owned by the shared guard.
    let status =
        unsafe { mo_skia_pick(frame.as_ptr(), frame.len() as u32, &mut pointer, &mut words) };
    let _owned = Output(pointer.cast());
    if !(0..=4).contains(&status) || (status != 0 && (!pointer.is_null() || words != 0)) {
        return Err(RasterError::ComponentInvalid("native picking ownership"));
    }
    if status != 0 {
        return Ok(PickingReply {
            status: status as u32,
            words: vec![],
        });
    }
    if pointer.is_null() || !(7..=MAX_PICK_REPLY_WORDS).contains(&(words as usize)) {
        return Err(RasterError::ComponentInvalid("native picking allocation"));
    }
    // SAFETY: the successful pinned ABI reports exactly this many initialized
    // u32s, and the guard retains the allocation while they are copied.
    let data = unsafe { slice::from_raw_parts(pointer, words as usize) };
    let mut words = Vec::new();
    words
        .try_reserve_exact(data.len())
        .map_err(|_| RasterError::Host("native picking allocation"))?;
    words.extend_from_slice(data);
    Ok(PickingReply { status: 0, words })
}
