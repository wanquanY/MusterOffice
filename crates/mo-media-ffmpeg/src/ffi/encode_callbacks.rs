use super::bindings::EncodeIo;
use crate::{ErrorKind, MediaError, WriteAt};
use std::{
    cell::{Cell, RefCell},
    ffi::c_void,
    panic::{AssertUnwindSafe, catch_unwind},
};

#[derive(Clone, Copy)]
enum Failure {
    WritePanic,
    CancelPanic,
    Io,
    Contract,
}
/// Box-stable address. C calls synchronously on the owning thread; each closure
/// has its own dynamic borrow guard so accidental recursion fails closed.
pub(super) struct Callbacks<'a> {
    sink: RefCell<Box<dyn WriteAt + 'a>>,
    cancel: RefCell<Box<dyn FnMut() -> bool + 'a>>,
    maximum: u64,
    failure: Cell<Option<Failure>>,
    cancelled: Cell<bool>,
}
impl<'a> Callbacks<'a> {
    pub fn new(
        sink: impl WriteAt + 'a,
        maximum: u64,
        cancel: impl FnMut() -> bool + 'a,
    ) -> Box<Self> {
        Box::new(Self {
            sink: RefCell::new(Box::new(sink)),
            cancel: RefCell::new(Box::new(cancel)),
            maximum,
            failure: Cell::new(None),
            cancelled: Cell::new(false),
        })
    }
    pub fn io(&mut self) -> EncodeIo {
        EncodeIo {
            opaque: (self as *mut Self).cast(),
            write_at,
            cancelled,
        }
    }
    fn fail(&self, failure: Failure) {
        if self.failure.get().is_none() {
            self.failure.set(Some(failure));
        }
    }
    pub fn check(&self) -> Result<(), MediaError> {
        // Same guarded implementation for explicit Rust boundary checks and C
        // interrupt callbacks; successful EOF cannot outrun cancellation.
        self.poll();
        if let Some(failure) = self.failure.get() {
            let (kind, message) = match failure {
                Failure::WritePanic => (ErrorKind::CallbackPanic, "write_at callback panicked"),
                Failure::CancelPanic => {
                    (ErrorKind::CallbackPanic, "cancellation callback panicked")
                }
                Failure::Io => (ErrorKind::Io, "write_at callback failed"),
                Failure::Contract => (
                    ErrorKind::CallbackContract,
                    "write_at callback returned an invalid byte count",
                ),
            };
            Err(MediaError::new(kind, message))
        } else if self.cancelled.get() {
            Err(MediaError::new(
                ErrorKind::Cancelled,
                "media operation cancelled",
            ))
        } else {
            Ok(())
        }
    }
    fn poll(&self) -> bool {
        if self.failure.get().is_some() || self.cancelled.get() {
            return true;
        }
        match catch_unwind(AssertUnwindSafe(|| (self.cancel.borrow_mut())())) {
            Ok(value) => {
                self.cancelled.set(value);
                value
            }
            Err(payload) => {
                // Panic payloads may themselves panic in Drop. At most one such
                // payload is retained per poisoned session; never unwind in C.
                std::mem::forget(payload);
                self.fail(Failure::CancelPanic);
                true
            }
        }
    }
}
unsafe extern "C" fn cancelled(opaque: *mut c_void) -> i32 {
    // SAFETY: Context owns a Box<Callbacks> until mo_encode_close finishes;
    // this ABI never calls callbacks concurrently or retains them afterwards.
    let state = unsafe { &*opaque.cast::<Callbacks<'_>>() };
    i32::from(state.poll())
}
unsafe extern "C" fn write_at(
    opaque: *mut c_void,
    offset: u64,
    buffer: *const u8,
    capacity: i32,
) -> i32 {
    // SAFETY: same owner/lifetime invariant as cancelled. C provides an initialized read-only
    // allocation of `capacity` bytes used only for the duration of this call.
    let state = unsafe { &*opaque.cast::<Callbacks<'_>>() };
    if state.poll() {
        return -1;
    }
    if capacity <= 0
        || buffer.is_null()
        || offset > state.maximum
        || capacity as u64 > state.maximum - offset
    {
        state.fail(Failure::Contract);
        return -1;
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: C lends initialized immutable output bytes for this call.
        let bytes = unsafe { std::slice::from_raw_parts(buffer, capacity as usize) };
        match state.sink.borrow_mut().write_at(offset, bytes) {
            Ok(n) if n > 0 && n <= bytes.len() => n as i32,
            Ok(_) => {
                state.fail(Failure::Contract);
                -1
            }
            Err(_) => {
                state.fail(Failure::Io);
                -1
            }
        }
    }));
    match result {
        Ok(value) => value,
        Err(payload) => {
            std::mem::forget(payload);
            state.fail(Failure::WritePanic);
            -1
        }
    }
}
