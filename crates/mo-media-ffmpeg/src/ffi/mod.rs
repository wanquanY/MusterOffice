//! All native ownership, pointer access and callbacks live behind this module.
mod bindings;
mod callbacks;
mod metadata;
use crate::{DecodeLimits, DecodedFrame, ErrorKind, FramePlane, MediaError, ReadAt, StreamInfo};
use bindings::*;
use callbacks::Callbacks;
use std::{ffi::CStr, marker::PhantomData, mem::MaybeUninit, ptr::NonNull, rc::Rc};

pub(crate) struct Context<'a> {
    raw: Option<NonNull<Media>>,
    callbacks: Box<Callbacks<'a>>,
    limits: DecodeLimits,
    // Explicitly !Send and !Sync, independently of callback implementations.
    _thread: PhantomData<Rc<()>>,
}
impl<'a> Context<'a> {
    pub fn open(
        source: impl ReadAt + 'a,
        length: u64,
        limits: DecodeLimits,
        cancel: impl FnMut() -> bool + 'a,
    ) -> Result<Self, MediaError> {
        // SAFETY: fixed ABI functions return a process-lifetime static version.
        let version_ok = unsafe {
            let version = mo_media_upstream_version();
            mo_media_abi_version() == 1
                && !version.is_null()
                && CStr::from_ptr(version).to_bytes()
                    == env!("MO_MEDIA_EXPECTED_VERSION").as_bytes()
        };
        if !version_ok {
            return Err(MediaError::new(
                ErrorKind::Abi,
                "loaded media component version mismatch",
            ));
        }
        let mut context = Self {
            raw: None,
            callbacks: Callbacks::new(source, length, cancel),
            limits,
            _thread: PhantomData,
        };
        context.callbacks.check()?;
        let io = context.callbacks.io();
        let limits = Limits {
            max_input_bytes: limits.max_input_bytes,
            max_read_bytes: limits.max_read_bytes,
            max_frame_bytes: limits.max_frame_bytes,
            max_streams: limits.max_streams,
            max_pixels: limits.max_pixels,
            max_audio_samples: limits.max_audio_samples,
            max_channels: limits.max_channels,
            max_decode_steps: limits.max_decode_steps,
        };
        let mut raw = std::ptr::null_mut();
        let mut error = blank_error();
        // SAFETY: synchronous open borrows valid records; Box-stable callback
        // owner lives through close. `raw` starts null, and successful ownership
        // is immediately installed in the drop guard before any fallible work.
        let status = unsafe { mo_media_open(&io, &limits, &mut raw, &mut error) };
        context.raw = NonNull::new(raw);
        context.end(ok(status, error))?;
        if context.raw.is_none() {
            return Err(MediaError::new(
                ErrorKind::Abi,
                "successful open returned null",
            ));
        }
        Ok(context)
    }
    fn begin(&mut self) -> Result<*mut Media, MediaError> {
        let raw = self.raw.ok_or_else(|| {
            MediaError::new(ErrorKind::State, "media session is closed after failure")
        })?;
        if let Err(error) = self.callbacks.check() {
            self.close();
            return Err(error);
        }
        Ok(raw.as_ptr())
    }
    fn end<T>(&mut self, result: Result<T, MediaError>) -> Result<T, MediaError> {
        let result = self.callbacks.check().and(result);
        if result.is_err() {
            self.close();
        }
        result
    }
    pub fn streams(&mut self) -> Result<Vec<StreamInfo>, MediaError> {
        let raw = self.begin()?;
        // SAFETY: live unique session, immutable stream metadata output only.
        let count = unsafe { mo_media_stream_count(raw) };
        let result = (|| {
            if count > self.limits.max_streams {
                return Err(MediaError::new(ErrorKind::Limit, "stream count budget"));
            }
            let mut streams = Vec::new();
            streams
                .try_reserve_exact(count as usize)
                .map_err(|_| MediaError::new(ErrorKind::Limit, "stream allocation"))?;
            for index in 0..count {
                let mut output = MaybeUninit::<Stream>::uninit();
                let mut error = blank_error();
                // SAFETY: C initializes all fields on OK. No data is read on error.
                let status =
                    unsafe { mo_media_stream(raw, index, output.as_mut_ptr(), &mut error) };
                ok(status, error)?;
                let info = metadata::stream(unsafe { output.assume_init() })?;
                if info.index != index {
                    return Err(MediaError::new(ErrorKind::Abi, "stream index mismatch"));
                }
                streams.push(info);
            }
            Ok(streams)
        })();
        self.end(result)
    }
    pub fn select(&mut self, index: u32) -> Result<(), MediaError> {
        let raw = self.begin()?;
        let mut error = blank_error();
        // SAFETY: live exclusive context; public typestate permits one selection.
        let status = unsafe { mo_media_select(raw, index, &mut error) };
        self.end(ok(status, error))
    }
    pub fn seek(&mut self, ticks: i64) -> Result<(), MediaError> {
        let raw = self.begin()?;
        let mut error = blank_error();
        // SAFETY: selected live exclusive context; no frame pointers escape.
        let status = unsafe { mo_media_seek(raw, ticks, &mut error) };
        self.end(ok(status, error))
    }
    pub fn next_frame(&mut self) -> Result<Option<DecodedFrame>, MediaError> {
        let raw = self.begin()?;
        let mut output = MaybeUninit::<Frame>::uninit();
        let mut error = blank_error();
        // SAFETY: selected exclusive context. Output initialized only on FRAME.
        let status = unsafe { mo_media_next(raw, output.as_mut_ptr(), &mut error) };
        let result = (|| {
            if status == 2 {
                return Ok(None);
            }
            if status != 1 {
                return Err(failure(status, error));
            }
            let value = unsafe { output.assume_init() };
            if value.planes <= 0 || value.planes > 64 {
                return Err(MediaError::new(ErrorKind::Abi, "invalid plane count"));
            }
            let count = value.planes as u32;
            let info = metadata::frame(value)?;
            let mut total = 0u64;
            let mut planes = Vec::new();
            planes
                .try_reserve_exact(count as usize)
                .map_err(|_| MediaError::new(ErrorKind::Limit, "plane allocation"))?;
            for index in 0..count {
                let mut layout = MaybeUninit::<Plane>::uninit();
                let mut error = blank_error();
                // SAFETY: current frame remains valid until next/seek/close,
                // none of which can run concurrently with this exclusive call.
                let status = unsafe { mo_media_plane(raw, index, layout.as_mut_ptr(), &mut error) };
                ok(status, error)?;
                let layout = unsafe { layout.assume_init() };
                let bytes = layout
                    .rows
                    .checked_mul(layout.row_bytes)
                    .filter(|n| *n > 0 && *n <= self.limits.max_frame_bytes.saturating_sub(total))
                    .ok_or_else(|| {
                        MediaError::new(ErrorKind::Limit, "aggregate frame byte budget")
                    })?;
                total += bytes;
                let length = usize::try_from(bytes)
                    .map_err(|_| MediaError::new(ErrorKind::Limit, "addressable frame bytes"))?;
                let mut bytes = Vec::new();
                bytes
                    .try_reserve_exact(length)
                    .map_err(|_| MediaError::new(ErrorKind::Limit, "frame allocation"))?;
                bytes.resize(length, 0);
                let mut error = blank_error();
                // SAFETY: initialized writable allocation with exact active-row
                // capacity. On failure the entire frame, including partial bytes,
                // is dropped before it can become a successful public result.
                let status = unsafe {
                    mo_media_copy_plane(
                        raw,
                        index,
                        bytes.as_mut_ptr(),
                        length as u64,
                        layout.row_bytes,
                        &mut error,
                    )
                };
                ok(status, error)?;
                planes.push(FramePlane {
                    row_bytes: layout.row_bytes as usize,
                    rows: layout.rows as usize,
                    bytes,
                });
            }
            Ok(Some(DecodedFrame { info, planes }))
        })();
        self.end(result)
    }
    pub fn is_closed(&self) -> bool {
        self.raw.is_none()
    }
    fn close(&mut self) {
        if let Some(raw) = self.raw.take() {
            // SAFETY: exactly one close per successful native allocation;
            // callbacks remain alive during close and are dropped afterwards.
            unsafe {
                mo_media_close(raw.as_ptr());
            }
        }
    }
}
impl Drop for Context<'_> {
    fn drop(&mut self) {
        self.close();
    }
}
fn blank_error() -> RawError {
    RawError {
        status: 0,
        upstream_code: 0,
        message: [0; 192],
    }
}
fn ok(status: i32, error: RawError) -> Result<(), MediaError> {
    if status == 0 {
        Ok(())
    } else {
        Err(failure(status, error))
    }
}
fn failure(status: i32, error: RawError) -> MediaError {
    let kind = match status {
        -1 => ErrorKind::Invalid,
        -2 => ErrorKind::Io,
        -3 => ErrorKind::Cancelled,
        -4 => ErrorKind::Limit,
        -5 => ErrorKind::Unsupported,
        -6 => ErrorKind::Decode,
        -7 => ErrorKind::State,
        _ => ErrorKind::Abi,
    };
    MediaError {
        kind,
        upstream_code: (error.upstream_code != 0).then_some(error.upstream_code),
        message: metadata::string(&error.message),
    }
}
