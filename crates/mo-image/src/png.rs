//! Streaming, deterministic PNG output for the renderer's premultiplied sRGB
//! RGBA8 pixels. PNG stores unassociated alpha (W3C PNG 3, sections 6.2/11.2).
use flate2::{Compression, write::ZlibEncoder};
use mo_common::Digest;
use sha2::{Digest as _, Sha256};
use std::io::{self, Write};

const CHUNK: usize = 64 * 1024;
#[derive(Debug, thiserror::Error)]
pub enum PngError {
    #[error("invalid PNG input: {0}")]
    Invalid(&'static str),
    #[error("PNG encoding limit: {0}")]
    Limit(&'static str),
    #[error("PNG encoding cancelled")]
    Cancelled,
    #[error("PNG output: {0}")]
    Io(io::Error),
}
impl From<io::Error> for PngError {
    fn from(error: io::Error) -> Self {
        match error.get_ref().and_then(|e| e.downcast_ref::<Abort>()) {
            Some(Abort::Cancelled) => Self::Cancelled,
            Some(Abort::Limit) => Self::Limit("encoded bytes"),
            _ => Self::Io(error),
        }
    }
}
#[derive(Debug, thiserror::Error)]
enum Abort {
    #[error("PNG encoding cancelled")]
    Cancelled,
    #[error("PNG encoded bytes limit")]
    Limit,
}
#[derive(Debug, Clone)]
pub struct PngReceipt {
    pub sha256: Digest,
    pub byte_length: u64,
}

struct Output<'a, W> {
    writer: &'a mut W,
    check: &'a dyn Fn() -> bool,
    max: u64,
    written: u64,
    hash: Sha256,
    failed: bool,
}
impl<W: Write> Write for Output<'_, W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.failed {
            return Err(io::Error::other("PNG output already failed"));
        }
        let result = (|| {
            if (self.check)() {
                return Err(io::Error::other(Abort::Cancelled));
            }
            if self
                .written
                .checked_add(bytes.len() as u64)
                .is_none_or(|n| n > self.max)
            {
                return Err(io::Error::other(Abort::Limit));
            }
            let n = self.writer.write(bytes)?;
            if n > bytes.len() {
                return Err(io::Error::other("invalid PNG output write count"));
            }
            self.hash.update(&bytes[..n]);
            self.written += n as u64;
            Ok(n)
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    fn flush(&mut self) -> io::Result<()> {
        if self.failed {
            return Err(io::Error::other("PNG output already failed"));
        }
        if (self.check)() {
            self.failed = true;
            return Err(io::Error::other(Abort::Cancelled));
        }
        let result = self.writer.flush();
        if result.is_err() {
            self.failed = true;
        }
        result
    }
}
fn chunk(out: &mut impl Write, kind: &[u8; 4], data: &[u8]) -> io::Result<()> {
    let length = u32::try_from(data.len()).map_err(io::Error::other)?;
    let mut crc = crc32fast::Hasher::new();
    crc.update(kind);
    crc.update(data);
    out.write_all(&length.to_be_bytes())?;
    out.write_all(kind)?;
    out.write_all(data)?;
    out.write_all(&crc.finalize().to_be_bytes())
}
struct Idat<'a, W> {
    out: &'a mut W,
    pending: Vec<u8>,
}
impl<W: Write> Idat<'_, W> {
    fn emit(&mut self) -> io::Result<()> {
        if !self.pending.is_empty() {
            chunk(self.out, b"IDAT", &self.pending)?;
            self.pending.clear();
        }
        Ok(())
    }
}
impl<W: Write> Write for Idat<'_, W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let n = bytes.len().min(CHUNK - self.pending.len());
        self.pending.extend_from_slice(&bytes[..n]);
        if self.pending.len() == CHUNK {
            self.emit()?;
        }
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.emit()?;
        self.out.flush()
    }
}

/// Output includes an sRGB declaration, 8-bit RGBA, no interlace and a Sub
/// filter. Memory beyond input pixels is one row, 64 KiB IDAT and deflate state.
/// Transparent RGB is canonical zero. Other channels use nearest integer
/// unpremultiplication; this preserves each premultiplied integer on re-premul.
/// A receipt proves writes only. The caller must seal and verify final storage.
pub fn encode_to<W: Write>(
    pixels: &[u8],
    width: u32,
    height: u32,
    out: &mut W,
    max_bytes: u64,
    check: &dyn Fn() -> bool,
) -> Result<PngReceipt, PngError> {
    if check() {
        return Err(PngError::Cancelled);
    }
    let length = u64::from(width) * u64::from(height) * 4;
    if width == 0
        || height == 0
        || width > 8192
        || height > 8192
        || length > super::MAX_PIXEL_BYTES as u64
    {
        return Err(PngError::Limit("pixel dimensions or bytes"));
    }
    if length != pixels.len() as u64 {
        return Err(PngError::Invalid("pixel length"));
    }
    // Validate before emitting a signature; never silently clamp malformed RGBA.
    for block in pixels.chunks(16384) {
        if check() {
            return Err(PngError::Cancelled);
        }
        if block
            .chunks_exact(4)
            .any(|p| p[..3].iter().any(|c| *c > p[3]))
        {
            return Err(PngError::Invalid("premultiplied channels"));
        }
    }
    let mut out = Output {
        writer: out,
        check,
        max: max_bytes,
        written: 0,
        hash: Sha256::new(),
        failed: false,
    };
    out.write_all(b"\x89PNG\r\n\x1a\n")?;
    let mut header = [0; 13];
    header[..4].copy_from_slice(&width.to_be_bytes());
    header[4..8].copy_from_slice(&height.to_be_bytes());
    header[8] = 8;
    header[9] = 6;
    chunk(&mut out, b"IHDR", &header)?;
    chunk(&mut out, b"sRGB", &[0])?;
    let mut row = vec![0; width as usize * 4 + 1];
    row[0] = 1;
    let mut encoded = ZlibEncoder::new(
        Idat {
            out: &mut out,
            pending: Vec::with_capacity(CHUNK),
        },
        Compression::fast(),
    );
    for pixels in pixels.chunks_exact(width as usize * 4) {
        if check() {
            return Err(PngError::Cancelled);
        }
        let mut left = [0; 4];
        for (input, output) in pixels.chunks_exact(4).zip(row[1..].chunks_exact_mut(4)) {
            let a = u32::from(input[3]);
            let mut straight = [0, 0, 0, input[3]];
            if a != 0 {
                for i in 0..3 {
                    straight[i] = ((u32::from(input[i]) * 255 + a / 2) / a) as u8;
                }
            }
            for i in 0..4 {
                output[i] = straight[i].wrapping_sub(left[i]);
            }
            left = straight;
        }
        encoded.write_all(&row)?;
    }
    encoded.finish()?.emit()?;
    chunk(&mut out, b"IEND", &[])?;
    out.flush()?;
    if check() {
        return Err(PngError::Cancelled);
    }
    Ok(PngReceipt {
        sha256: Digest::from_sha256(out.hash.finalize().into()),
        byte_length: out.written,
    })
}
