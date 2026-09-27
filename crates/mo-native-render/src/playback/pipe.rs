use super::{Error, Frame, NativePlayback, SessionError};
use mo_common::Digest;
use mo_native_worker::{CHUNK_BYTES, Session};
use mo_presentation_delivery::Content;
use mo_raster::RasterInfo;
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest as _, Sha256};
use std::{
    io::Read,
    time::{Duration, Instant},
};

pub(super) struct Pipe<T> {
    session: Session<Frame<T>>,
    timeout: Duration,
    resources: bool,
    first_call: Option<Instant>,
    deadline: Instant,
}
impl<T: DeserializeOwned + Send + 'static> Pipe<T> {
    pub fn start(
        config: &NativePlayback,
        mode: &str,
        resources: bool,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, Error> {
        let start = Instant::now();
        let command = config.command(mode, check)?;
        Ok(Self {
            session: Session::start(command, read_frame::<T>)?,
            timeout: config.timeout,
            resources,
            first_call: Some(start),
            deadline: start + config.timeout,
        })
    }
    pub fn call(
        &mut self,
        request: &impl Serialize,
        source: Content<'_>,
        fonts: Content<'_>,
        check: &dyn Fn() -> bool,
    ) -> Result<Frame<T>, Error> {
        let start = self.first_call.take().unwrap_or_else(Instant::now);
        self.deadline = start + self.timeout;
        if self.session.is_stopped() {
            return Err(SessionError::Stopped.into());
        }
        if check() {
            self.session.stop();
            return Err(SessionError::Cancelled.into());
        }
        if source.byte_length > mo_kernel_api::MAX_INLINE_RESOURCE_BYTES as u64
            || fonts.byte_length > mo_kernel_api::MAX_INLINE_FONT_BYTES as u64
        {
            return Err(Error::Input("playback source/font byte limit".into()));
        }
        let json = super::super::request_json(request).map_err(|e| Error::Input(e.to_string()))?;
        let mut header = (json.len() as u32).to_le_bytes().to_vec();
        if self.resources {
            header.extend_from_slice(&(source.byte_length as u32).to_le_bytes());
            header.extend_from_slice(&(fonts.byte_length as u32).to_le_bytes());
        } else if source.byte_length != 0 || fonts.byte_length != 0 {
            return Err(Error::Input(
                "author sampler does not take source bytes".into(),
            ));
        }
        let parts = [
            Content {
                reader: &header,
                byte_length: header.len() as u64,
            },
            Content {
                reader: &json,
                byte_length: json.len() as u64,
            },
            source,
            fonts,
        ];
        let (mut part, mut offset) = (0, 0);
        self.session
            .call(self.timeout.saturating_sub(start.elapsed()), check, || {
                while part < parts.len() && offset == parts[part].byte_length {
                    part += 1;
                    offset = 0;
                }
                if part == parts.len() {
                    return Ok(None);
                }
                let n = (parts[part].byte_length - offset).min(CHUNK_BYTES as u64) as usize;
                let mut bytes = vec![0; n];
                parts[part]
                    .reader
                    .read_exact_at(&mut bytes, offset)
                    .map_err(|e| e.to_string())?;
                offset += n as u64;
                Ok(Some(bytes))
            })
            .map_err(Error::from)
    }
    pub fn control(
        &mut self,
        request: &impl Serialize,
        check: &dyn Fn() -> bool,
    ) -> Result<T, Error> {
        let frame = self.call(request, empty(), empty(), check)?;
        if !frame.pixels.is_empty() {
            return self.invalid("control response contains pixels");
        }
        Ok(frame.info)
    }
    pub fn invalid<R>(&mut self, message: &'static str) -> Result<R, Error> {
        self.session.stop();
        Err(Error::Response(message))
    }
    pub fn validate_pixels(
        &mut self,
        info: &RasterInfo,
        pixels: &[u8],
        size: (u32, u32),
        check: &dyn Fn() -> bool,
    ) -> Result<(), Error> {
        let Some(expected) = u64::from(size.0)
            .checked_mul(u64::from(size.1))
            .and_then(|n| n.checked_mul(4))
            .filter(|n| *n > 0 && *n <= mo_raster::MAX_PIXEL_BYTES as u64)
        else {
            return self.invalid("frame dimension limit");
        };
        if !mo_raster::accepts_profile(&info.profile, self.resources)
            || (info.width, info.height) != size
            || info.byte_length.get() != expected
            || pixels.len() as u64 != expected
        {
            return self.invalid("frame dimensions, profile or byte length differ");
        }
        let mut hash = Sha256::new();
        for chunk in pixels.chunks(CHUNK_BYTES) {
            self.checkpoint(check)?;
            if chunk
                .chunks_exact(4)
                .any(|p| p[0] > p[3] || p[1] > p[3] || p[2] > p[3])
            {
                return self.invalid("invalid premultiplied pixels");
            }
            hash.update(chunk);
        }
        if Digest::from_sha256(hash.finalize().into()) != info.sha256 {
            return self.invalid("frame pixel digest differs");
        }
        self.checkpoint(check)
    }
    fn checkpoint(&mut self, check: &dyn Fn() -> bool) -> Result<(), Error> {
        let error = if check() {
            Some(SessionError::Cancelled)
        } else if Instant::now() >= self.deadline {
            Some(SessionError::Deadline)
        } else {
            None
        };
        if let Some(error) = error {
            self.session.stop();
            return Err(error.into());
        }
        Ok(())
    }
    pub fn is_stopped(&self) -> bool {
        self.session.is_stopped()
    }
    pub fn finish(self, elapsed: Duration, check: &dyn Fn() -> bool) -> Result<(), Error> {
        self.session
            .finish(self.timeout.saturating_sub(elapsed), check)
            .map_err(Error::from)
    }
}
pub(super) fn empty() -> Content<'static> {
    static EMPTY: &[u8] = &[];
    Content {
        reader: &EMPTY,
        byte_length: 0,
    }
}
fn read_frame<T: DeserializeOwned>(
    out: &mut std::process::ChildStdout,
) -> Result<Frame<T>, String> {
    let mut lengths = [0; 8];
    out.read_exact(&mut lengths).map_err(|e| e.to_string())?;
    let metadata = u32::from_le_bytes(lengths[..4].try_into().unwrap()) as usize;
    let pixels = u32::from_le_bytes(lengths[4..].try_into().unwrap()) as usize;
    if metadata > 64 * 1024 * 1024 || pixels > mo_raster::MAX_PIXEL_BYTES {
        return Err("playback response byte limit".into());
    }
    fn bytes(out: &mut std::process::ChildStdout, n: usize) -> Result<Vec<u8>, String> {
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(n).map_err(|e| e.to_string())?;
        bytes.resize(n, 0);
        out.read_exact(&mut bytes).map_err(|e| e.to_string())?;
        Ok(bytes)
    }
    let metadata = bytes(out, metadata)?;
    let text = std::str::from_utf8(&metadata).map_err(|e| e.to_string())?;
    let info = mo_common::from_json_str(text).map_err(|e| e.to_string())?;
    Ok(Frame {
        info,
        pixels: bytes(out, pixels)?,
    })
}
#[cfg(all(test, unix))]
mod tests;
