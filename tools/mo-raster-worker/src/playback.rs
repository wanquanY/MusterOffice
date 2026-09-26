//! One owner per isolated worker. Parent controls timeout, termination and publication.
use std::io::{self, Read, Write};
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    let mut session = mo_kernel_api::PlaybackSession::default();
    let mut raster = mo_skia_sys::NativeRaster;
    loop {
        let mut header = [0; 4];
        if input.read(&mut header[..1])? == 0 {
            break;
        }
        input.read_exact(&mut header[1..])?;
        let size = u32::from_le_bytes(header) as usize;
        if size > mo_kernel_api::MAX_REQUEST_BYTES {
            return Err("playback session request limit".into());
        }
        let mut bytes = vec![0; size];
        input.read_exact(&mut bytes)?;
        let (metadata, pixels) =
            session.dispatch_json(std::str::from_utf8(&bytes)?, Some(&mut raster), &|| false);
        output.write_all(&u32::try_from(metadata.len())?.to_le_bytes())?;
        output.write_all(&u32::try_from(pixels.len())?.to_le_bytes())?;
        output.write_all(metadata.as_bytes())?;
        output.write_all(&pixels)?;
        output.flush()?;
        if raster.is_invalid() {
            break;
        }
    }
    Ok(())
}
