//! Safe, serial ownership of the pinned native pitch-preserving DSP component.
#![allow(unsafe_code)]
use mo_video_audio::{AudioError, Result, StretchEngine, StretchInfo};
use std::{ffi::c_void, ptr::NonNull};

unsafe extern "C" {
    fn mo_stretch_abi() -> u32;
    fn mo_stretch_create(
        channels: u32,
        block: u32,
        interval: u32,
        bytes: u32,
        out: *mut *mut c_void,
    ) -> u32;
    fn mo_stretch_destroy(state: *mut c_void);
    fn mo_stretch_latency(state: *const c_void, input: *mut u32, output: *mut u32) -> u32;
    fn mo_stretch_memory(state: *const c_void, live: *mut u32, peak: *mut u32) -> u32;
    fn mo_stretch_begin(state: *mut c_void, input: *const f32, frames: u32, rate: f64) -> u32;
    fn mo_stretch_process(
        state: *mut c_void,
        input: *const f32,
        input_frames: u32,
        output: *mut f32,
        output_frames: u32,
    ) -> u32;
}
fn status(value: u32) -> Result<()> {
    match value {
        0 => Ok(()),
        1 => Err(AudioError::Invalid("stretch component input")),
        2 => Err(AudioError::Budget),
        3 => Err(AudioError::Invalid("poisoned stretch component")),
        _ => Err(AudioError::Invalid("stretch component failure")),
    }
}
pub struct NativeStretch {
    state: NonNull<c_void>,
    info: StretchInfo,
}
impl NativeStretch {
    /// The reviewed default window is 120 ms with a 30 ms synthesis interval.
    /// Budget counts the instance/engine and all vector capacities; host buffers,
    /// allocator bookkeeping, C++ exception runtime and process RSS are separate.
    pub fn new(sample_rate: u32, channels: usize, max_bytes: u32) -> Result<Self> {
        // SAFETY: no arguments or externally owned storage.
        if unsafe { mo_stretch_abi() } != 1 {
            return Err(AudioError::Invalid("stretch component ABI"));
        }
        if !(8000..=384000).contains(&sample_rate) || !(1..=64).contains(&channels) {
            return Err(AudioError::Invalid("stretch native configuration"));
        }
        let mut pointer = std::ptr::null_mut();
        // SAFETY: scalar validated configuration and writable local output slot.
        status(unsafe {
            mo_stretch_create(
                channels as u32,
                sample_rate * 12 / 100,
                sample_rate * 3 / 100,
                max_bytes,
                &mut pointer,
            )
        })?;
        let state = NonNull::new(pointer).ok_or(AudioError::Invalid("null stretch instance"))?;
        let mut result = Self {
            state,
            info: StretchInfo {
                sample_rate,
                channels,
                input_latency: 0,
                output_latency: 0,
            },
        };
        // SAFETY: live owned instance; distinct writable integer slots.
        status(unsafe {
            mo_stretch_latency(
                result.state.as_ptr(),
                &mut result.info.input_latency,
                &mut result.info.output_latency,
            )
        })?;
        Ok(result)
    }
    pub fn memory(&self) -> Result<(u32, u32)> {
        let (mut live, mut peak) = (0, 0);
        // SAFETY: immutable live instance and writable local output slots.
        status(unsafe { mo_stretch_memory(self.state.as_ptr(), &mut live, &mut peak) })?;
        Ok((live, peak))
    }
}
impl StretchEngine for NativeStretch {
    fn info(&self) -> StretchInfo {
        self.info
    }
    fn begin(&mut self, input: &[f32], rate: f64) -> Result<()> {
        if input.len() != self.info.input_latency as usize * self.info.channels {
            return Err(AudioError::Invalid("stretch preroll length"));
        }
        // SAFETY: instance uniquely borrowed; input length matches channels/frames.
        status(unsafe {
            mo_stretch_begin(
                self.state.as_ptr(),
                input.as_ptr(),
                self.info.input_latency,
                rate,
            )
        })
    }
    fn process(&mut self, input: &[f32], output: &mut [f32]) -> Result<()> {
        let channels = self.info.channels;
        if !input.len().is_multiple_of(channels)
            || !output.len().is_multiple_of(channels)
            || input.len() / channels > 65536
            || output.is_empty()
            || output.len() / channels > 4096
        {
            return Err(AudioError::Invalid("stretch process lengths"));
        }
        // SAFETY: unique instance, valid distinct slice regions; f32 pointer
        // alignment and scalar lengths agree. No pointer survives this call.
        status(unsafe {
            mo_stretch_process(
                self.state.as_ptr(),
                input.as_ptr(),
                (input.len() / channels) as u32,
                output.as_mut_ptr(),
                (output.len() / channels) as u32,
            )
        })
    }
}
impl Drop for NativeStretch {
    fn drop(&mut self) {
        // SAFETY: exactly one owner; C++ drops the DSP before its budget object.
        unsafe { mo_stretch_destroy(self.state.as_ptr()) }
    }
}
