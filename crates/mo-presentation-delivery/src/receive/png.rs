//! Validate the exact PNG profile emitted by mo_image::png, with one scanline
//! plus a 64 KiB chunk buffer. This is not a general image import decoder.
use super::*;
#[cfg(test)]
#[path = "png_tests.rs"]
mod tests;
use flate2::bufread::ZlibDecoder;
use sha2::{Digest as _, Sha256};
use std::{
    cell::Cell,
    io::{self, BufReader, Read},
};

struct Ranges<'a> {
    source: &'a dyn mo_opc::ReaderAt,
    ranges: Vec<(u64, u64)>,
    index: usize,
    used: u64,
    check: &'a dyn Fn() -> bool,
    aborted: &'a Cell<bool>,
    source_failed: &'a Cell<bool>,
}
impl Read for Ranges<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if (self.check)() {
            self.aborted.set(true);
            return Err(io::Error::other("delivery PNG cancelled"));
        }
        while let Some(&(offset, length)) = self.ranges.get(self.index) {
            if self.used == length {
                self.index += 1;
                self.used = 0;
                continue;
            }
            let n = (length - self.used).min(buffer.len() as u64) as usize;
            if let Err(error) = self
                .source
                .read_exact_at(&mut buffer[..n], offset + self.used)
            {
                self.source_failed.set(true);
                return Err(error);
            }
            self.used += n as u64;
            return Ok(n);
        }
        Ok(0)
    }
}

pub(super) fn verify(
    content: &Content<'_>,
    width: u32,
    height: u32,
    expected_pixels: &Digest,
    check: &dyn Fn() -> bool,
) -> Result<(), DeliveryError> {
    let invalid = || DeliveryError::Invalid("delivery PNG profile or pixel digest");
    if width == 0
        || height == 0
        || width > 8192
        || height > 8192
        || u64::from(width) * u64::from(height) * 4 > mo_raster::MAX_PIXEL_BYTES as u64
        || content.byte_length < 8
    {
        return Err(invalid());
    }
    let mut signature = [0; 8];
    content.reader.read_exact_at(&mut signature, 0)?;
    if signature != *b"\x89PNG\r\n\x1a\n" {
        return Err(invalid());
    }
    let mut offset = 8;
    let mut stage = 0;
    let mut ranges = Vec::new();
    let mut compressed = 0;
    let mut buffer = vec![0; 65536];
    loop {
        cancel(check)?;
        if content.byte_length - offset < 12 {
            return Err(invalid());
        }
        let mut header = [0; 8];
        content.reader.read_exact_at(&mut header, offset)?;
        let len = u32::from_be_bytes(header[..4].try_into().expect("four bytes")) as usize;
        let kind = &header[4..];
        if len > buffer.len() || len as u64 + 12 > content.byte_length - offset {
            return Err(invalid());
        }
        content
            .reader
            .read_exact_at(&mut buffer[..len], offset + 8)?;
        let mut crc = crc32fast::Hasher::new();
        crc.update(kind);
        crc.update(&buffer[..len]);
        let mut stored_crc = [0; 4];
        content
            .reader
            .read_exact_at(&mut stored_crc, offset + 8 + len as u64)?;
        if crc.finalize() != u32::from_be_bytes(stored_crc) {
            return Err(invalid());
        }
        match (stage, kind) {
            (0, b"IHDR") => {
                if len != 13
                    || buffer[..4] != width.to_be_bytes()
                    || buffer[4..8] != height.to_be_bytes()
                    || buffer[8..13] != [8, 6, 0, 0, 0]
                {
                    return Err(invalid());
                }
                stage = 1;
            }
            (1, b"sRGB") => {
                if len != 1 || buffer[0] != 0 {
                    return Err(invalid());
                }
                stage = 2;
            }
            (2, b"IDAT") => {
                if ranges.len() == 4096 {
                    return Err(DeliveryError::Limit("delivery PNG chunks"));
                }
                ranges.push((offset + 8, len as u64));
                compressed += len as u64;
            }
            (2, b"IEND") if len == 0 && !ranges.is_empty() => {
                if offset + 12 != content.byte_length {
                    return Err(invalid());
                }
                break;
            }
            _ => return Err(invalid()),
        }
        offset += len as u64 + 12;
    }
    drop(buffer);
    let aborted = Cell::new(false);
    let source_failed = Cell::new(false);
    let reader = Ranges {
        source: content.reader,
        ranges,
        index: 0,
        used: 0,
        check,
        aborted: &aborted,
        source_failed: &source_failed,
    };
    let mut decoder = ZlibDecoder::new(BufReader::with_capacity(65536, reader));
    let mut row = vec![0; width as usize * 4 + 1];
    let mut hash = Sha256::new();
    let result = (|| {
        for _ in 0..height {
            cancel(check)?;
            decoder.read_exact(&mut row)?;
            if row[0] != 1 {
                return Err(invalid());
            }
            let mut left = [0u8; 4];
            for pixel in row[1..].chunks_exact_mut(4) {
                for i in 0..4 {
                    pixel[i] = pixel[i].wrapping_add(left[i]);
                }
                left.copy_from_slice(pixel);
                let alpha = u32::from(pixel[3]);
                if alpha == 0 && pixel[..3] != [0, 0, 0] {
                    return Err(invalid());
                }
                for channel in &mut pixel[..3] {
                    *channel = ((u32::from(*channel) * alpha + 127) / 255) as u8;
                }
            }
            hash.update(&row[1..]);
        }
        let mut extra = [0];
        if decoder.read(&mut extra)? != 0 || decoder.total_in() != compressed {
            return Err(invalid());
        }
        if Digest::from_sha256(hash.finalize().into()) != *expected_pixels {
            return Err(invalid());
        }
        cancel(check)
    })();
    if aborted.get() {
        return Err(DeliveryError::Cancelled);
    }
    match result {
        Err(DeliveryError::Io(_)) if !source_failed.get() => {
            Err(DeliveryError::Invalid("delivery PNG deflate data"))
        }
        other => other,
    }
}
