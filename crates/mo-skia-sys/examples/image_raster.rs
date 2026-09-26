//! Verification host: explicit bounded inputs, one isolated process per request.
//! stdin: u32 JSON length, JSON {raster, images}, then contiguous RGBA resources.
//! stdout: u32 metadata length, metadata JSON, then completed RGBA frame.
use mo_raster::{ImageResource, PathRasterRequest, PreparedImages, RasterBackend};
use serde::Deserialize;
use std::io::{Read, Write};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    raster: PathRasterRequest,
    images: Vec<ImageResource>,
}
#[derive(Default)]
struct Backend {
    inner: mo_skia_sys::NativeRaster,
    calls: u32,
}
impl RasterBackend for Backend {
    fn raster(&mut self, frame: &[u32]) -> Result<mo_raster::BackendReply, mo_raster::RasterError> {
        self.calls += 1;
        self.inner.raster(frame)
    }
    fn raster_images(
        &mut self,
        frame: &[u32],
        images: &[u8],
    ) -> Result<mo_raster::BackendReply, mo_raster::RasterError> {
        self.calls += 1;
        self.inner.raster_images(frame, images)
    }
    fn invalidate(&mut self) {
        self.inner.invalidate();
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = Vec::new();
    std::io::stdin()
        .take(72 * 1024 * 1024 + 1)
        .read_to_end(&mut input)?;
    if input.len() < 4 || input.len() > 72 * 1024 * 1024 {
        return Err("input size".into());
    }
    let size = u32::from_le_bytes(input[..4].try_into()?) as usize;
    if size > 8 * 1024 * 1024 || size > input.len() - 4 {
        return Err("JSON size".into());
    }
    let request: Request = serde_json::from_slice(&input[4..4 + size])?;
    let mut backend = Backend::default();
    let result = (|| {
        let images = PreparedImages::new(&request.images, &input[4 + size..], &|| false)?;
        let compiled = mo_raster::compile_images(&request.raster, &images, &|| false)?;
        let frame = compiled.frame().to_vec();
        let output = mo_raster::render_compiled_images(compiled, &mut backend, &|| false)?;
        Ok::<_, mo_raster::RasterError>((output, frame))
    })();
    let (metadata, pixels) = match result {
        Ok((output, frame)) => (
            serde_json::json!({"raster": output.raster.info,
            "images": output.work, "resourcesSha256": output.resources_sha256, "frame": frame,
            "calls": backend.calls, "invalid": backend.inner.is_invalid()}),
            output.raster.pixels,
        ),
        Err(error) => (
            serde_json::json!({"error": error.to_string(), "calls": backend.calls,
            "invalid": backend.inner.is_invalid()}),
            vec![],
        ),
    };
    let bytes = serde_json::to_vec(&metadata)?;
    let mut out = std::io::stdout().lock();
    out.write_all(&(bytes.len() as u32).to_le_bytes())?;
    out.write_all(&bytes)?;
    out.write_all(&pixels)?;
    Ok(())
}
