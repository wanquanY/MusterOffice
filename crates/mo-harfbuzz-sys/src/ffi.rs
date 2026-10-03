use mo_text::{
    TextError,
    backend::{
        MAX_CARETS_RESULT_WORDS, MAX_METRICS_RESULT_WORDS, MAX_OUTLINES_RESULT_WORDS,
        MAX_RESULT_WORDS,
    },
};
use std::{ffi::c_void, ptr, slice};
unsafe extern "C" {
    fn mo_hb_shape(
        font: *const u8,
        font_length: u32,
        request: *const u32,
        request_words: u32,
        language: *const u8,
        language_length: u32,
        output: *mut *mut u32,
        output_words: *mut u32,
    ) -> i32;
    fn mo_hb_measure_font(
        font: *const u8,
        font_length: u32,
        request: *const u32,
        request_words: u32,
        output: *mut *mut u32,
        output_words: *mut u32,
    ) -> i32;
    fn mo_hb_outline_font(
        font: *const u8,
        font_length: u32,
        request: *const u32,
        request_words: u32,
        output: *mut *mut u32,
        output_words: *mut u32,
    ) -> i32;
    fn mo_hb_caret_font(
        font: *const u8,
        font_length: u32,
        request: *const u32,
        request_words: u32,
        output: *mut *mut u32,
        output_words: *mut u32,
    ) -> i32;
    fn mo_hb_free(pointer: *mut c_void);
    fn mo_hb_version() -> u32;
    fn mo_hb_invalidate();
    fn mo_hb_instance_invalid() -> i32;
}
pub(super) fn invalidate() {
    // SAFETY: the pinned allocator exposes an atomic, irreversible invalidation.
    unsafe { mo_hb_invalidate() }
}
pub(super) fn invalid() -> bool {
    // SAFETY: pure atomic read; no borrowed pointers or allocation.
    unsafe { mo_hb_instance_invalid() != 0 }
}
struct Output(*mut u32);
impl Drop for Output {
    fn drop(&mut self) {
        // SAFETY: only the pinned component's owned result pointer enters this guard.
        // Null is accepted by its free function; no Rust allocator touches the buffer.
        unsafe { mo_hb_free(self.0.cast()) };
    }
}
pub(super) fn version() -> u32 {
    // SAFETY: pure, no-argument version function of the linked pinned component.
    unsafe { mo_hb_version() }
}
#[derive(Clone, Copy)]
pub(super) enum Operation {
    Shape,
    Metrics,
    Outlines,
    Carets,
}
impl Operation {
    pub fn maximum(self) -> usize {
        match self {
            Self::Shape => MAX_RESULT_WORDS,
            Self::Metrics => MAX_METRICS_RESULT_WORDS,
            Self::Outlines => MAX_OUTLINES_RESULT_WORDS,
            Self::Carets => MAX_CARETS_RESULT_WORDS,
        }
    }
}
pub(super) fn compute(
    font: &[u8],
    request: &[u32],
    language: Option<&str>,
    operation: Operation,
) -> Result<(u32, Vec<u32>), TextError> {
    let n = u32::try_from(font.len()).map_err(|_| TextError::Limit("FFI font bytes"))?;
    let r = u32::try_from(request.len()).map_err(|_| TextError::Limit("FFI request words"))?;
    let l = u32::try_from(language.map_or(0, str::len))
        .map_err(|_| TextError::Limit("FFI language bytes"))?;
    let mut pointer = ptr::null_mut();
    let mut count = 0;
    // SAFETY: slices remain borrowed throughout this synchronous call, request is
    // u32-aligned, lengths fit the ABI, output slots are valid and non-overlapping.
    // The library holds no input references after return. Callers serialize access.
    let status = unsafe {
        if let Some(language) = language {
            mo_hb_shape(
                font.as_ptr(),
                n,
                request.as_ptr(),
                r,
                language.as_ptr(),
                l,
                &mut pointer,
                &mut count,
            )
        } else if matches!(operation, Operation::Outlines) {
            mo_hb_outline_font(
                font.as_ptr(),
                n,
                request.as_ptr(),
                r,
                &mut pointer,
                &mut count,
            )
        } else if matches!(operation, Operation::Carets) {
            mo_hb_caret_font(
                font.as_ptr(),
                n,
                request.as_ptr(),
                r,
                &mut pointer,
                &mut count,
            )
        } else {
            mo_hb_measure_font(
                font.as_ptr(),
                n,
                request.as_ptr(),
                r,
                &mut pointer,
                &mut count,
            )
        }
    };
    let output = Output(pointer);
    if !(0..=6).contains(&status) || (status != 0 && (!pointer.is_null() || count != 0)) {
        return Err(TextError::BackendInvalid(
            "native status or failure ownership",
        ));
    }
    if status != 0 {
        return Ok((status as u32, vec![]));
    }
    if pointer.is_null()
        || count < if language.is_some() { 8 } else { 6 }
        || count as usize > operation.maximum()
        || !(pointer as usize).is_multiple_of(align_of::<u32>())
    {
        return Err(TextError::BackendInvalid("native result pointer or length"));
    }
    // SAFETY: the pinned C ABI guarantees this successful allocation contains count
    // initialized u32s. Bounds/alignment were checked; it lives until guard drop.
    let words = unsafe { slice::from_raw_parts(output.0, count as usize) };
    let mut copied = Vec::new();
    copied
        .try_reserve_exact(words.len())
        .map_err(|_| TextError::Host("native result copy allocation"))?;
    copied.extend_from_slice(words);
    Ok((0, copied))
}
