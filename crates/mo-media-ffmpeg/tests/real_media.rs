//! The verification runner generates real B-frame/VFR/AAC media and compares
//! these actual Rust outputs with an independent command-line decoder.
use mo_media_ffmpeg::{Container, DecodeLimits, DecodedFrame, Decoder, MediaKind};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

fn decoder(path: &Path, kind: MediaKind) -> Decoder<'static> {
    let mut file = File::open(path).unwrap();
    let length = file.metadata().unwrap().len();
    let limits = DecodeLimits {
        max_input_bytes: 1 << 28,
        max_read_bytes: 1 << 30,
        max_frame_bytes: 1 << 24,
        max_streams: 16,
        max_pixels: 1 << 24,
        max_audio_samples: 262144,
        max_channels: 8,
        max_decode_steps: 10000,
    };
    let container = Container::open(
        move |offset, out: &mut [u8]| {
            file.seek(SeekFrom::Start(offset))?;
            file.read(out)
        },
        length,
        limits,
        || false,
    )
    .unwrap();
    let stream = container
        .streams()
        .iter()
        .find(|s| s.kind == kind)
        .unwrap()
        .index;
    container.select(stream).unwrap()
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
fn drain(decoder: &mut Decoder<'_>, mut output: Option<&mut File>) -> Vec<Value> {
    let mut records = Vec::new();
    while let Some(frame) = decoder.next_frame().unwrap() {
        if let Some(output) = output.as_mut() {
            for plane in &frame.planes {
                output.write_all(&plane.bytes).unwrap();
            }
        }
        records.push(record(&frame));
    }
    assert!(decoder.next_frame().unwrap().is_none());
    records
}
#[test]
#[ignore = "requires runner-generated real media and an explicit output directory"]
fn actual_rust_media_decode() {
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
    video.seek(time_base.denominator as i64).unwrap();
    let forward = drain(&mut video, None);
    video.seek(0).unwrap();
    let backward = drain(&mut video, None);
    let mut audio = decoder(Path::new(&fixture), MediaKind::Audio);
    let audio = drain(
        &mut audio,
        Some(&mut File::create(output.join("audio.f32")).unwrap()),
    );
    let report =
        json!({"linear": linear, "forward": forward, "backward": backward, "audio": audio});
    fs::write(
        output.join("frames.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}
