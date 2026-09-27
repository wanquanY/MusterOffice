//! Host example: own files, invoke a retained SDK sampler, save verified frames.
//! No graphics libraries, database, Node, UI or live presentation clock here.
use mo_embedded_sdk::{common::RationalTime, delivery::Content, playback::*};
use serde_json::{Value, json};
use std::{fs, io::Read, path::Path, time::Duration};

fn read(path: &Path, limit: u64) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err("example input byte limit".into());
    }
    Ok(bytes)
}
fn json(path: &Path, value: &Value) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.write_all(b"\n")?;
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    if a.len() != 6 && a.len() != 8 {
        return Err("usage: playback-example worker sha256 author|source prepare.json samples.json new-output [source.pptx fonts.bin]".into());
    }
    let runtime = NativePlayback::new(
        Path::new(&a[0]).canonicalize()?,
        a[1].clone().try_into()?,
        Duration::from_secs(30),
    )?;
    let prepare = read(Path::new(&a[3]), 32 * 1024 * 1024)?;
    let samples: Vec<Value> = serde_json::from_slice(&read(Path::new(&a[4]), 1024 * 1024)?)?;
    if samples.is_empty() || samples.len() > 256 {
        return Err("example sample count".into());
    }
    let out = Path::new(&a[5]);
    let mut files = Vec::new();
    let mut save =
        |ordinal: usize, info: Value, pixels: Vec<u8>| -> Result<(), Box<dyn std::error::Error>> {
            use std::io::Write;
            let meta = format!("{ordinal:04}.json");
            let rgba = format!("{ordinal:04}.rgba");
            json(&out.join(&meta), &info)?;
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(out.join(&rgba))?
                .write_all(&pixels)?;
            files.push(json!({"metadata":meta,"pixels":rgba,"byteLength":pixels.len()}));
            Ok(())
        };
    let summary = match a[2].as_str() {
        "author" if a.len() == 6 => {
            let request: PlaybackPrepareRequest = serde_json::from_slice(&prepare)?;
            let deck = mo_embedded_sdk::Presentation::from_snapshot(request.snapshot)?;
            if request.binding.revision != deck.snapshot().revision {
                return Err("example playback revision conflict".into());
            }
            let mut owner = deck.prepare_author_playback(
                &runtime,
                mo_embedded_sdk::PlaybackOptions {
                    session: request.binding.session,
                    generation: request.binding.generation,
                    slide: request.slide,
                    viewport: request.viewport,
                    defaults: request.defaults,
                },
                &|| false,
            )?;
            fs::create_dir(out)?;
            let info = serde_json::to_value(owner.info())?;
            for (i, q) in samples.iter().enumerate() {
                let at: RationalTime = serde_json::from_value(q["at"].clone())?;
                let history =
                    serde_json::from_value(q.get("history").cloned().unwrap_or(Value::Null))?;
                let frame = owner.sample(at, history, &|| false)?;
                save(i, serde_json::to_value(frame.info)?, frame.pixels)?;
            }
            let timing = serde_json::to_value(owner.timing(&|| false)?)?;
            let generation = owner
                .info()
                .binding
                .generation
                .next()
                .ok_or("generation overflow")?;
            owner.advance(generation, &|| false)?;
            let advanced = serde_json::to_value(owner.info())?;
            owner.dispose(&|| false)?;
            json!({"prepared":info,"timing":timing,"advanced":advanced})
        }
        "source" if a.len() == 8 => {
            // These source/font buffers cease to exist before the first sample.
            let mut owner = {
                let source = read(Path::new(&a[6]), 128 * 1024 * 1024)?;
                let fonts = read(Path::new(&a[7]), 128 * 1024 * 1024)?;
                runtime.prepare_source(
                    serde_json::from_slice(&prepare)?,
                    Content {
                        reader: &source,
                        byte_length: source.len() as u64,
                    },
                    Content {
                        reader: &fonts,
                        byte_length: fonts.len() as u64,
                    },
                    &|| false,
                )?
            };
            fs::create_dir(out)?;
            let info = serde_json::to_value(owner.info())?;
            for (i, q) in samples.iter().enumerate() {
                let at = serde_json::from_value(q["at"].clone())?;
                let history =
                    serde_json::from_value(q.get("history").cloned().unwrap_or(Value::Null))?;
                let frame = owner.sample(at, history, &|| false)?;
                save(i, serde_json::to_value(frame.info)?, frame.pixels)?;
            }
            let timing = serde_json::to_value(owner.timing(&|| false)?)?;
            let generation = owner
                .info()
                .binding
                .generation
                .next()
                .ok_or("generation overflow")?;
            owner.advance(generation, &|| false)?;
            let advanced = serde_json::to_value(owner.info())?;
            owner.dispose(&|| false)?;
            json!({"prepared":info,"timing":timing,"advanced":advanced})
        }
        _ => return Err("mode/arguments mismatch".into()),
    };
    json(
        &out.join("result.json"),
        &json!({"status":"computed","files":files,"session":summary,"productCommitted":false}),
    )?;
    Ok(())
}
