//! Explicit integration with the real stepped C++ component and frozen SDK frames.
//! Paths/processes belong to this verification host, never to the computation core.
use mo_kernel_api::*;
use mo_raster::BackendReply;
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn save(path: impl AsRef<Path>, bytes: &[u8]) {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap()
        .write_all(bytes)
        .unwrap();
}
fn step(
    words: &[u32],
    images: Option<&[u8]>,
    budget: u32,
    probe: &Path,
    output: &Path,
) -> BackendReply {
    let mut child = Command::new(probe)
        .args([u32::from(images.is_some()).to_string(), budget.to_string()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut data = Vec::new();
    data.extend_from_slice(&(words.len() as u32).to_le_bytes());
    for word in words {
        data.extend_from_slice(&word.to_le_bytes());
    }
    if let Some(images) = images {
        data.extend_from_slice(&(images.len() as u32).to_le_bytes());
        data.extend_from_slice(images);
    }
    child.stdin.take().unwrap().write_all(&data).unwrap();
    let result = child.wait_with_output().unwrap();
    save(output.with_extension("input"), &data);
    save(output.with_extension("out"), &result.stdout);
    save(output.with_extension("log"), &result.stderr);
    assert!(result.status.success());
    let status = u32::from_le_bytes(result.stdout[..4].try_into().unwrap());
    let count = u32::from_le_bytes(result.stdout[4..8].try_into().unwrap()) as usize;
    assert_eq!(count + 8, result.stdout.len());
    assert_eq!(status, 0);
    let metrics: Value = serde_json::from_slice(&result.stderr).unwrap();
    assert!(metrics["steps"].as_u64().unwrap() > 0);
    BackendReply {
        status,
        pixels: result.stdout[8..].to_vec(),
    }
}
fn compare(
    reference: &Path,
    output: &Path,
    index: usize,
    budget: u32,
    info: Value,
    pixels: Vec<u8>,
) {
    let old = reference.join("output").join(format!("{index:04}"));
    assert_eq!(info, read(old.with_extension("json")));
    assert_eq!(pixels, fs::read(old.with_extension("rgba")).unwrap());
    save(
        output.join(format!("{index:04}-{budget}.json")),
        &serde_json::to_vec_pretty(&info).unwrap(),
    );
}

#[test]
#[ignore = "requires explicit frozen MO_PLAYBACK_REFERENCE, MO_SKIA_STEP_PROBE and new MO_PREPARED_OUTPUT"]
fn prepared_frames_match_frozen_native_sdk_pixels_and_metadata() {
    let root = std::path::PathBuf::from(std::env::var_os("MO_PLAYBACK_REFERENCE").unwrap());
    let probe = std::path::PathBuf::from(std::env::var_os("MO_SKIA_STEP_PROBE").unwrap());
    let output = std::path::PathBuf::from(std::env::var_os("MO_PREPARED_OUTPUT").unwrap());
    fs::create_dir(&output).unwrap();
    let report = read(root.join("report.json"));
    let mut count = 0;
    for group in report["observations"].as_array().unwrap() {
        let kind = group["kind"].as_str().unwrap();
        let directory = format!("{kind}-{:03}", group["group"].as_u64().unwrap());
        let reference = root.join(&directory);
        let destination = output.join(&directory);
        fs::create_dir(&destination).unwrap();
        let prepare = read(reference.join("prepare.json"));
        let samples = read(reference.join("samples.json"));
        if kind == "author" {
            let mut owner = PlaybackSession::default();
            let request = PlaybackSessionRequest::Prepare {
                request: Box::new(serde_json::from_value(prepare.clone()).unwrap()),
            };
            assert!(matches!(
                owner.dispatch(request, None, &|| false).0,
                PlaybackSessionResponse::Prepared { .. }
            ));
            for (index, sample) in samples.as_array().unwrap().iter().enumerate() {
                for budget in [1, 7, 4096] {
                    let q = json!({"binding":prepare["binding"],"at":sample["at"],"history":sample["history"]});
                    let pending = owner
                        .prepare_render(serde_json::from_value(q).unwrap(), &|| false)
                        .unwrap();
                    let reply = step(
                        pending.words(),
                        None,
                        budget,
                        &probe,
                        &destination.join(format!("{index:04}-{budget}")),
                    );
                    let image = owner.complete_render(pending, reply, &|| false).unwrap();
                    compare(
                        &reference,
                        &destination,
                        index,
                        budget,
                        serde_json::to_value(image.info).unwrap(),
                        image.pixels,
                    );
                    count += 1;
                }
            }
        } else {
            assert_eq!(kind, "source");
            let mut owner = PptxPlaybackSession::default();
            {
                let source = fs::read(reference.join("source.bin")).unwrap();
                let fonts = fs::read(reference.join("fonts.bin")).unwrap();
                let request = PptxPlaybackSessionRequest::Prepare {
                    request: Box::new(serde_json::from_value(prepare.clone()).unwrap()),
                };
                let reply = owner
                    .dispatch(
                        request,
                        &source,
                        &fonts,
                        Some(PptxPlaybackResources {
                            decoder: &mut mo_skia_sys::NativeRaster,
                            text: Some(&mut mo_harfbuzz_sys::NativeShaper::default()),
                        }),
                        None,
                        &|| false,
                    )
                    .0;
                assert!(
                    matches!(reply, PptxPlaybackSessionResponse::Prepared { .. }),
                    "{reply:?}"
                );
            } // No source/font/component owner is borrowed by prepared frames.
            for (index, sample) in samples.as_array().unwrap().iter().enumerate() {
                for budget in [1, 7, 4096] {
                    let q = json!({"binding":prepare["binding"],"at":sample["at"],"history":sample["history"]});
                    let pending = owner
                        .prepare_render(serde_json::from_value(q).unwrap(), &|| false)
                        .unwrap();
                    let reply = step(
                        pending.words(),
                        Some(pending.images()),
                        budget,
                        &probe,
                        &destination.join(format!("{index:04}-{budget}")),
                    );
                    let (info, pixels) = owner.complete_render(pending, reply, &|| false).unwrap();
                    compare(
                        &reference,
                        &destination,
                        index,
                        budget,
                        serde_json::to_value(info).unwrap(),
                        pixels,
                    );
                    count += 1;
                }
            }
        }
    }
    assert_eq!(count, report["frameCount"].as_u64().unwrap() * 3);
    save(output.join("report.json"), &serde_json::to_vec_pretty(&json!({"status":"passed","comparisons":count,"distinctFrames":report["frameCount"],"budgets":[1,7,4096]})).unwrap());
}
