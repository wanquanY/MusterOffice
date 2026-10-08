use super::{bindings::*, blank_error, encode_callbacks::Callbacks, ok};
use crate::{
    EncodeConfig, EncodeLimits, EncodeProfile, EncodeStats, EncodeVideoFrame, ErrorKind,
    MediaError, WriteAt,
};
use std::{ffi::CStr, marker::PhantomData, mem::MaybeUninit, ptr::NonNull, rc::Rc};

pub(crate) struct EncodeContext<'a> {
    raw: Option<NonNull<Encode>>,
    callbacks: Box<Callbacks<'a>>,
    config: EncodeConfig,
    _thread: PhantomData<Rc<()>>,
}
fn integer(value: u32) -> Result<i32, MediaError> {
    i32::try_from(value)
        .map_err(|_| MediaError::new(ErrorKind::Invalid, "encoder integer overflow"))
}
fn version() -> Result<(), MediaError> {
    // SAFETY: process-lifetime metadata from the checked native wrapper.
    let valid = unsafe {
        let v = mo_media_upstream_version();
        mo_encode_abi_version() == 1
            && !v.is_null()
            && CStr::from_ptr(v).to_bytes() == env!("MO_MEDIA_EXPECTED_VERSION").as_bytes()
    };
    if valid {
        Ok(())
    } else {
        Err(MediaError::new(
            ErrorKind::Abi,
            "loaded encoder ABI/version mismatch",
        ))
    }
}
pub(crate) fn profiles() -> Result<Vec<EncodeProfile>, MediaError> {
    version()?;
    // SAFETY: read-only codec inventory; no device/session or host capabilities.
    let mask = unsafe { mo_encode_profiles() };
    if mask & !3 != 0 {
        return Err(MediaError::new(
            ErrorKind::Abi,
            "unknown encoder profile bit",
        ));
    }
    Ok([
        (1, EncodeProfile::H264VideoToolboxAacMp4),
        (2, EncodeProfile::Ffv1PcmMp4),
    ]
    .into_iter()
    .filter_map(|(bit, p)| (mask & bit != 0).then_some(p))
    .collect())
}
impl<'a> EncodeContext<'a> {
    pub fn open(
        sink: impl WriteAt + 'a,
        config: EncodeConfig,
        limits: EncodeLimits,
        cancel: impl FnMut() -> bool + 'a,
    ) -> Result<Self, MediaError> {
        version()?;
        let raw_config = EncoderConfig {
            profile: match config.profile {
                EncodeProfile::H264VideoToolboxAacMp4 => 1,
                EncodeProfile::Ffv1PcmMp4 => 2,
                EncodeProfile::H264WebCodecsAacMp4 => {
                    return Err(MediaError::new(
                        ErrorKind::Unsupported,
                        "WebCodecs profile requires a browser encoder",
                    ));
                }
            },
            width: integer(config.width)?,
            height: integer(config.height)?,
            video_time_scale: integer(config.video_time_scale)?,
            frame_rate_num: integer(config.frame_rate_num)?,
            frame_rate_den: integer(config.frame_rate_den)?,
            video_duration: config.video_duration,
            video_frames: i64::from(config.video_frames),
            video_bit_rate: i64::from(config.video_bit_rate),
            audio_sample_rate: integer(config.audio_sample_rate)?,
            audio_channels: integer(config.audio_channels)?,
            audio_bit_rate: integer(config.audio_bit_rate)?,
            gop_frames: integer(config.gop_frames)?,
            audio_samples: config.audio_samples,
        };
        let raw_limits = EncoderLimits {
            max_file_bytes: limits.max_file_bytes,
            max_write_bytes: limits.max_write_bytes,
            max_pending_bytes: limits.max_pending_bytes,
            max_frame_bytes: limits.max_frame_bytes,
            max_pixels: limits.max_pixels,
            max_packets: limits.max_packets,
            max_packet_bytes: limits.max_packet_bytes,
            max_pending_packets: limits.max_pending_packets,
            max_steps: limits.max_steps,
            max_write_calls: limits.max_write_calls,
        };
        let mut context = Self {
            raw: None,
            callbacks: Callbacks::new(sink, limits.max_file_bytes, cancel),
            config,
            _thread: PhantomData,
        };
        context.callbacks.check()?;
        let io = context.callbacks.io();
        let mut raw = std::ptr::null_mut();
        let mut error = blank_error();
        // SAFETY: live records and boxed callback owner. Native calls are serial;
        // the guard immediately takes ownership before any fallible check.
        let status = unsafe { mo_encode_open(&io, &raw_config, &raw_limits, &mut raw, &mut error) };
        context.raw = NonNull::new(raw);
        context.end(ok(status, error))?;
        if context.raw.is_none() {
            return Err(MediaError::new(
                ErrorKind::Abi,
                "encoder open returned null",
            ));
        }
        Ok(context)
    }
    fn begin(&mut self) -> Result<*mut Encode, MediaError> {
        let raw = self
            .raw
            .ok_or_else(|| MediaError::new(ErrorKind::State, "encoder is closed"))?;
        if let Err(e) = self.callbacks.check() {
            self.close();
            return Err(e);
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
    pub fn video(&mut self, frame: EncodeVideoFrame<'_>) -> Result<(), MediaError> {
        let raw = self.begin()?;
        let value = EncoderVideo {
            pts: frame.pts,
            duration: frame.duration,
            planes: frame.planes.map(|p| p.as_ptr()),
            plane_bytes: frame.planes.map(|p| p.len() as u64),
            strides: frame.strides.map(|n| n as u64),
        };
        let mut error = blank_error();
        // SAFETY: borrowed planes stay alive for the entire call. Their exact
        // lengths/strides are supplied and validated before C accesses rows.
        let status = unsafe { mo_encode_video(raw, &value, &mut error) };
        self.end(ok(status, error))
    }
    pub fn audio(&mut self, first: i64, samples: &[f32]) -> Result<(), MediaError> {
        let raw = self.begin()?;
        let channels = self.config.audio_channels as usize;
        if channels == 0
            || !samples.len().is_multiple_of(channels)
            || samples.len() / channels > 65536
        {
            return self.end(Err(MediaError::new(
                ErrorKind::Invalid,
                "encoder PCM shape",
            )));
        }
        let mut error = blank_error();
        // SAFETY: C reads exactly frames*configured_channels f32 values; the
        // checked slice is borrowed until this serial call returns.
        let status = unsafe {
            mo_encode_audio(
                raw,
                first,
                samples.as_ptr(),
                (samples.len() / channels) as u32,
                &mut error,
            )
        };
        self.end(ok(status, error))
    }
    pub fn finish(&mut self) -> Result<EncodeStats, MediaError> {
        let raw = self.begin()?;
        let mut output = MaybeUninit::<EncoderStats>::uninit();
        let mut error = blank_error();
        // SAFETY: C initializes every output field only on success.
        let status = unsafe { mo_encode_finish(raw, output.as_mut_ptr(), &mut error) };
        self.end(ok(status, error))?;
        let v = unsafe { output.assume_init() };
        self.close();
        Ok(EncodeStats {
            byte_length: v.byte_length,
            write_bytes: v.write_bytes,
            video_frames: v.video_frames,
            audio_samples: v.audio_samples,
            video_packets: v.video_packets,
            audio_packets: v.audio_packets,
            peak_pending_bytes: v.peak_pending_bytes,
        })
    }
    pub fn is_closed(&self) -> bool {
        self.raw.is_none()
    }
    fn close(&mut self) {
        if let Some(raw) = self.raw.take() {
            // SAFETY: unique allocation, callbacks outlive close. Close never
            // finalizes or publishes an incomplete candidate.
            unsafe {
                mo_encode_close(raw.as_ptr());
            }
        }
    }
}
impl Drop for EncodeContext<'_> {
    fn drop(&mut self) {
        self.close();
    }
}
