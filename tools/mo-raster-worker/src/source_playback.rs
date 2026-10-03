//! One source owner per isolated process; framed source bytes at prepare/resize, fonts only at prepare.
use std::io::{self, Read, Write};
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    let mut session = mo_kernel_api::PptxPlaybackSession::default();
    let mut raster = mo_skia_sys::NativeRaster;
    let mut decoder = mo_skia_sys::NativeRaster;
    let mut text = mo_harfbuzz_sys::NativeShaper::default();
    loop {
        let mut header = [0; 12];
        if input.read(&mut header[..1])? == 0 {
            break;
        }
        input.read_exact(&mut header[1..])?;
        let sizes: Vec<_> = header
            .chunks_exact(4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize)
            .collect();
        if sizes[0] > mo_kernel_api::MAX_REQUEST_BYTES
            || sizes[1] > mo_kernel_api::MAX_INLINE_RESOURCE_BYTES
            || sizes[2] > mo_kernel_api::MAX_INLINE_FONT_BYTES
        {
            return Err("source session input limit".into());
        }
        let mut request = vec![0; sizes[0]];
        let mut source = vec![0; sizes[1]];
        let mut fonts = vec![0; sizes[2]];
        input.read_exact(&mut request)?;
        input.read_exact(&mut source)?;
        input.read_exact(&mut fonts)?;
        let (metadata, pixels) = session.dispatch_json(
            std::str::from_utf8(&request)?,
            &source,
            &fonts,
            Some(mo_kernel_api::PptxPlaybackResources {
                decoder: &mut decoder,
                text: Some(&mut text),
            }),
            Some(&mut raster),
            &|| false,
        );
        output.write_all(&u32::try_from(metadata.len())?.to_le_bytes())?;
        output.write_all(&u32::try_from(pixels.len())?.to_le_bytes())?;
        output.write_all(metadata.as_bytes())?;
        output.write_all(&pixels)?;
        output.flush()?;
        if raster.is_invalid() || decoder.is_invalid() || text.is_invalid() {
            break;
        }
    }
    Ok(())
}
