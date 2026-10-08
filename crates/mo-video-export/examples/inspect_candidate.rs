//! Diagnostic host: inspect explicit local bytes against a supplied frozen plan.
//! This does not decode, create a durable receipt or publish an artifact.
use mo_media::EncodeConfig;
use mo_media_isobmff::{IndexLimits, IsoBmffIndex};
use std::io::{Read, Seek, SeekFrom};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: inspect_candidate CONFIG_JSON MP4".into());
    }
    let config: EncodeConfig = serde_json::from_slice(&std::fs::read(&args[0])?)?;
    let mut file = std::fs::File::open(&args[1])?;
    let length = file.metadata()?.len();
    let mut read = |at: u64, destination: &mut [u8]| {
        file.seek(SeekFrom::Start(at))?;
        file.read(destination)
    };
    let index = IsoBmffIndex::read(&mut read, length, IndexLimits::default(), &|| false)?;
    let result = mo_video_export::inspect_browser_candidate(&index, config, &|| false)?;
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
