//! One editor owner per isolated process. Only prepare consumes material/fonts.
use std::io::{self, Read, Write};
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    let mut session = mo_kernel_api::EditorPageSession::default();
    let mut raster = mo_skia_sys::NativeRaster;
    let mut decoder = mo_skia_sys::NativeRaster;
    let mut text = mo_harfbuzz_sys::NativeShaper::default();
    loop {
        let mut header = [0; 12];
        if input.read(&mut header[..1])? == 0 {
            break;
        }
        input.read_exact(&mut header[1..])?;
        let mut sizes = [0usize; 3];
        for (n, word) in sizes.iter_mut().zip(header.chunks_exact(4)) {
            *n = u32::from_le_bytes(word.try_into().unwrap()) as usize;
        }
        if sizes[0] > mo_kernel_api::MAX_REQUEST_BYTES
            || sizes[1] > mo_kernel_api::MAX_INLINE_RESOURCE_BYTES
            || sizes[2] > mo_kernel_api::MAX_INLINE_FONT_BYTES
        {
            return Err("editor page input limit".into());
        }
        let mut request = vec![0; sizes[0]];
        let mut material = vec![0; sizes[1]];
        let mut fonts = vec![0; sizes[2]];
        input.read_exact(&mut request)?;
        input.read_exact(&mut material)?;
        input.read_exact(&mut fonts)?;
        let (metadata, pixels) = session.dispatch_json(
            std::str::from_utf8(&request)?,
            &material,
            &fonts,
            Some(mo_kernel_api::EditorPageComponents::All(
                mo_kernel_api::EditorPageBackends {
                    decoder: &mut decoder,
                    text: &mut text,
                    raster: &mut raster,
                },
            )),
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
