//! The verification runner generates real B-frame/VFR/AAC media and compares
//! these actual Rust outputs with an independent command-line decoder.
mod support;
use mo_media::{DecodedFrame, MediaKind};
use mo_native_media::{NativeContainer, NativeDecoder, WorkerError, WorkerOptions};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

fn decoder(path: &Path, kind: MediaKind) -> NativeDecoder<'static> {
    decoder_with_options(path, kind, support::options())
}
fn decoder_with_options(
    path: &Path,
    kind: MediaKind,
    options: WorkerOptions,
) -> NativeDecoder<'static> {
    let mut file = File::open(path).unwrap();
    let length = file.metadata().unwrap().len();
    let container = NativeContainer::open(
        &support::bundle(),
        move |offset, out: &mut [u8]| {
            assert!(out.len() <= mo_media_wire::MAX_READ_BYTES as usize);
            file.seek(SeekFrom::Start(offset))?;
            file.read(out)
        },
        length,
        options,
        &|| false,
    )
    .unwrap();
    let stream = container
        .streams()
        .iter()
        .find(|s| s.kind == kind)
        .unwrap()
        .index;
    container.select(stream, &|| false).unwrap()
}
fn record(frame: &DecodedFrame) -> Value {
    let info = &frame.info;
    let mut digest = Sha256::new();
    for plane in &frame.planes {
        digest.update(&plane.bytes);
    }
    json!({ "generation": info.generation, "pts": info.pts_ticks, "duration": info.duration_ticks,
        "timeBase": [info.time_base.numerator, info.time_base.denominator],
        "format": info.format, "width": info.width, "height": info.height, "samples": info.samples,
        "colors": [info.colors.primaries, info.colors.transfer, info.colors.matrix, info.colors.range],
        "bytes": frame.planes.iter().map(|p| p.bytes.len()).sum::<usize>(), "sha256": format!("{:x}", digest.finalize()) })
}
fn drain(decoder: &mut NativeDecoder<'_>, mut output: Option<&mut File>) -> Vec<Value> {
    let mut records = Vec::new();
    while let Some(frame) = decoder.next_frame(&|| false).unwrap() {
        if let Some(output) = output.as_mut() {
            for plane in &frame.planes {
                output.write_all(&plane.bytes).unwrap();
            }
        }
        records.push(record(&frame));
    }
    assert!(decoder.next_frame(&|| false).unwrap().is_none());
    records
}
#[test]
#[ignore = "requires runner-generated real media and an explicit output directory"]
fn actual_worker_media_decode() {
    let fixture = std::env::var_os("MO_MEDIA_TEST_FIXTURE").expect("real fixture required");
    let output = std::env::var_os("MO_MEDIA_TEST_OUTPUT").expect("output directory required");
    let output = Path::new(&output);
    fs::create_dir(output).unwrap();
    let mut video = decoder(Path::new(&fixture), MediaKind::Video);
    let linear = drain(
        &mut video,
        Some(&mut File::create(output.join("video.yuv")).unwrap()),
    );
    assert_eq!(linear.len(), 77);
    let time_base = video.stream().time_base.unwrap();
    assert_eq!(time_base.numerator, 1);
    video.seek(time_base.denominator as i64, &|| false).unwrap();
    let forward = drain(&mut video, None);
    video.seek(0, &|| false).unwrap();
    let backward = drain(&mut video, None);
    video.close(&|| false).unwrap();
    let mut audio = decoder(Path::new(&fixture), MediaKind::Audio);
    let audio_records = drain(
        &mut audio,
        Some(&mut File::create(output.join("audio.f32")).unwrap()),
    );
    audio.close(&|| false).unwrap();
    let large = std::env::var_os("MO_MEDIA_LARGE_TEST_FIXTURE").expect("large packet fixture");
    let mut big = decoder(Path::new(&large), MediaKind::Video);
    let big_frame = big.next_frame(&|| false).unwrap().unwrap();
    let mut file = File::create(output.join("large-frame.yuv")).unwrap();
    for plane in &big_frame.planes {
        file.write_all(&plane.bytes).unwrap();
    }
    assert_eq!(
        big_frame
            .planes
            .iter()
            .map(|p| p.bytes.len())
            .sum::<usize>(),
        1024 * 1024 * 3 / 2
    );
    assert!(big.next_frame(&|| false).unwrap().is_none());
    big.close(&|| false).unwrap();
    let mut limits = support::options();
    limits.decode.max_frame_bytes = 1 << 20;
    let mut limited = decoder_with_options(Path::new(&large), MediaKind::Video, limits);
    let pid = limited.process_id().unwrap();
    assert!(
        matches!(limited.next_frame(&|| false), Err(WorkerError::Media(error)) if error.kind == mo_media::ErrorKind::Limit)
    );
    assert!(limited.is_stopped());
    support::assert_reaped(pid);
    let report = json!({"linear": linear, "forward": forward, "backward": backward, "audio": audio_records, "largeFrame": record(&big_frame)});
    fs::write(
        output.join("frames.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}
