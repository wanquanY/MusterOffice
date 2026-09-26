//! Explicit encoded bytes to normalized pixels; no file, network or OS authority.
use mo_common::Digest;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use thiserror::Error;
pub mod png;
mod resolution;
pub use resolution::{
    Density, ImageResolution, PhysicalPixelSize, PixelExtent, ResolutionDeclaration,
    ResolutionSource, ResolutionUnit,
};

pub const MAX_ENCODED_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_PIXEL_BYTES: usize = 64 * 1024 * 1024;
pub const PROFILE: &str = "skia-8d6d37b-png-jpeg-oriented-srgb-premul-rgba8-v1-draft";
#[derive(Debug, Error)]
pub enum ImageError {
    #[error("invalid encoded image: {0}")]
    Invalid(&'static str),
    #[error("image decode limit: {0}")]
    Limit(&'static str),
    #[error("image decode cancelled")]
    Cancelled,
    #[error("image decoder status {0}")]
    Component(u32),
    #[error("invalid image decoder reply: {0}")]
    ComponentInvalid(&'static str),
    #[error("image decoder host: {0}")]
    Host(&'static str),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ImageFormat {
    Png,
    Jpeg,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SourceColor {
    AssumedSrgb,
    Icc,
    PngColor,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DecodedImageInfo {
    pub profile: String,
    pub source_sha256: Digest,
    pub pixels_sha256: Digest,
    pub width: u32,
    pub height: u32,
    pub encoded_width: u32,
    pub encoded_height: u32,
    pub orientation: u32,
    pub format: ImageFormat,
    pub encoded_bit_depth: u32,
    pub source_color: SourceColor,
    pub byte_length: u32,
    pub resolution: ImageResolution,
}
pub struct DecodedImage {
    info: DecodedImageInfo,
    pixels: Vec<u8>,
}
impl DecodedImage {
    pub fn info(&self) -> &DecodedImageInfo {
        &self.info
    }
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
    pub fn into_parts(self) -> (DecodedImageInfo, Vec<u8>) {
        (self.info, self.pixels)
    }
}
pub struct DecoderReply {
    pub status: u32,
    /// ABI metadata: dimensions, orientation, format, depth, color, byte length.
    pub words: [u32; 9],
    pub pixels: Vec<u8>,
}
pub trait ImageDecoder {
    fn decode(&mut self, encoded: &[u8]) -> Result<DecoderReply, ImageError>;
    fn invalidate(&mut self);
}
fn cancel(check: &dyn Fn() -> bool) -> Result<(), ImageError> {
    if check() {
        Err(ImageError::Cancelled)
    } else {
        Ok(())
    }
}
fn hash(bytes: &[u8], check: &dyn Fn() -> bool) -> Result<Digest, ImageError> {
    let mut hash = Sha256::new();
    for chunk in bytes.chunks(16384) {
        cancel(check)?;
        hash.update(chunk);
    }
    Ok(Digest::from_sha256(hash.finalize().into()))
}
/// Decoded pixels become available only after full result validation. The digest
/// binds the caller's resource identity to the actual encoded bytes.
pub fn decode(
    encoded: &[u8],
    source_sha256: &Digest,
    backend: &mut dyn ImageDecoder,
    check: &dyn Fn() -> bool,
) -> Result<DecodedImage, ImageError> {
    cancel(check)?;
    if encoded.is_empty() {
        return Err(ImageError::Invalid("empty input"));
    }
    if encoded.len() > MAX_ENCODED_BYTES {
        return Err(ImageError::Limit("encoded bytes"));
    }
    if &hash(encoded, check)? != source_sha256 {
        return Err(ImageError::Invalid("source digest"));
    }
    cancel(check)?;
    let result = (|| {
        let reply = backend.decode(encoded)?;
        cancel(check)?;
        if reply.status > 5
            || reply.status == 4
            || (reply.status != 0 && (!reply.pixels.is_empty() || reply.words != [0; 9]))
        {
            return Err(ImageError::ComponentInvalid("status or ownership"));
        }
        if reply.status != 0 {
            return Err(ImageError::Component(reply.status));
        }
        let [
            width,
            height,
            ew,
            eh,
            orientation,
            format,
            depth,
            color,
            byte_length,
        ] = reply.words;
        if width == 0
            || height == 0
            || width > 8192
            || height > 8192
            || !(1..=8).contains(&orientation)
            || u64::from(width) * u64::from(height) * 4 != u64::from(byte_length)
            || byte_length as usize > MAX_PIXEL_BYTES
            || reply.pixels.len() != byte_length as usize
            || (if orientation >= 5 { (eh, ew) } else { (ew, eh) }) != (width, height)
        {
            return Err(ImageError::ComponentInvalid("dimensions or orientation"));
        }
        let format = match format {
            1 if [1, 2, 4, 8, 16].contains(&depth) && encoded.starts_with(b"\x89PNG\r\n\x1a\n") => {
                ImageFormat::Png
            }
            2 if depth == 8 && encoded.starts_with(&[0xff, 0xd8]) => ImageFormat::Jpeg,
            _ => return Err(ImageError::ComponentInvalid("format or depth")),
        };
        let source_color = match color {
            0 => SourceColor::AssumedSrgb,
            1 => SourceColor::Icc,
            2 if format == ImageFormat::Png => SourceColor::PngColor,
            _ => return Err(ImageError::ComponentInvalid("color profile")),
        };
        for chunk in reply.pixels.chunks(16384) {
            cancel(check)?;
            if chunk
                .chunks_exact(4)
                .any(|p| p[..3].iter().any(|v| *v > p[3]))
            {
                return Err(ImageError::ComponentInvalid("premultiplied channels"));
            }
        }
        let pixels_sha256 = hash(&reply.pixels, check)?;
        let resolution = resolution::read(encoded, format, orientation, check)?;
        cancel(check)?;
        Ok(DecodedImage {
            info: DecodedImageInfo {
                profile: PROFILE.into(),
                source_sha256: source_sha256.clone(),
                pixels_sha256,
                width,
                height,
                encoded_width: ew,
                encoded_height: eh,
                orientation,
                format,
                encoded_bit_depth: depth,
                source_color,
                byte_length,
                resolution,
            },
            pixels: reply.pixels,
        })
    })();
    if matches!(
        result,
        Err(ImageError::Host(_)
            | ImageError::ComponentInvalid(_)
            | ImageError::Cancelled
            | ImageError::Component(2))
    ) {
        backend.invalidate();
    }
    result
}

#[cfg(test)]
mod tests;
