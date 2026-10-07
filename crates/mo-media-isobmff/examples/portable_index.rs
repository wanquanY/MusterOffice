//! Verification host only: explicit fixture and request paths, no codec or UI.
use mo_media_isobmff::{IndexLimits, portable::PortableIndex};
use serde_json::{Value, json};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        return Err("usage: portable_index FIXTURE REQUESTS_JSON".into());
    }
    let mut source = File::open(&args[1])?;
    let length = source.metadata()?.len();
    let mut index = PortableIndex::open(
        &mut |offset, out: &mut [u8]| {
            source.seek(SeekFrom::Start(offset))?;
            source.read(out)
        },
        length,
        IndexLimits::default(),
        &|| false,
    )?;
    let requests: Vec<Value> = serde_json::from_slice(&std::fs::read(&args[2])?)?;
    let mut results = Vec::new();
    for request in requests {
        let body = request["body"].to_string();
        let result = match request["method"].as_str() {
            Some("summary") => index.summary(),
            Some("query") => index.query(&body),
            Some("videoTiming") => index.video_timing(
                request["body"].as_u64().ok_or("track")?.try_into()?,
                &|| false,
            ),
            Some("selectVideo") => index.select_video(&body, &|| false),
            Some("movie") => index.movie(&body, &|| false),
            _ => return Err("unknown verification method".into()),
        };
        results.push(match result {
            Ok(text) => json!({"ok":true,"value":serde_json::from_str::<Value>(&text)?}),
            Err(error) => json!({"ok":false,"error":error}),
        });
    }
    println!("{}", serde_json::to_string(&results)?);
    Ok(())
}
