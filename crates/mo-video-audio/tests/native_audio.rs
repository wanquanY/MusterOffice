//! Explicit real-media evidence. Input PCM is decoded again in an isolated
//! pinned worker; no prerecorded output stands in for this run's implementation.
mod support;
use mo_common::{ByteLength, Digest as ContentDigest};
use mo_media::{DecodeLimits, MediaKind};
use mo_native_limits::ProcessLimits;
use mo_native_media::{NativeContainer, WorkerBundle, WorkerOptions};
use mo_time::{ExactRatio as R, Rounding};
use mo_video_audio::*;
use mo_video_compile::{CompileLimits, CompiledDocument};
use mo_video_model::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
    time::Duration,
};
use support::*;
fn options() -> WorkerOptions {
    WorkerOptions {
        decode: DecodeLimits {
            max_input_bytes: 1 << 30,
            max_read_bytes: 1 << 32,
            max_frame_bytes: 1 << 20,
            max_streams: 16,
            max_pixels: 1 << 24,
            max_audio_samples: 262144,
            max_channels: 8,
            max_decode_steps: 10000,
        },
        process: ProcessLimits {
            additional_address_bytes: 512 << 20,
            cpu_seconds: 60,
            open_files: 32,
        },
        open_timeout: Duration::from_secs(15),
        call_timeout: Duration::from_secs(15),
    }
}
#[test]
#[ignore = "requires a pinned worker, real AAC fixture and a fresh output directory"]
fn actual_worker_pcm_to_block_mixer() {
    let fixture = PathBuf::from(std::env::var_os("MO_AUDIO_FIXTURE").expect("fixture"));
    let output = PathBuf::from(std::env::var_os("MO_AUDIO_OUTPUT").expect("fresh output"));
    fs::create_dir(&output).unwrap();
    let bundle = WorkerBundle::open(
        PathBuf::from(std::env::var_os("MO_MEDIA_TEST_BUNDLE").expect("bundle")),
        std::env::var("MO_MEDIA_TEST_BUNDLE_SHA256").expect("pin"),
    )
    .unwrap();
    let mut file = File::open(&fixture).unwrap();
    let length = file.metadata().unwrap().len();
    let container = NativeContainer::open(
        &bundle,
        move |offset, out: &mut [u8]| {
            file.seek(SeekFrom::Start(offset))?;
            file.read(out)
        },
        length,
        options(),
        &|| false,
    )
    .unwrap();
    let index = container
        .streams()
        .iter()
        .find(|s| s.kind == MediaKind::Audio)
        .unwrap()
        .index;
    let mut decoder = container.select(index, &|| false).unwrap();
    let mut samples = Vec::new();
    let mut records = vec![];
    while let Some(frame) = decoder.next_frame(&|| false).unwrap() {
        let info = &frame.info;
        assert_eq!(info.sample_rate, Some(48000));
        let pts = R::new(
            (info.pts_ticks.expect("actual timestamp") as i128
                * i128::from(info.time_base.numerator)
                * 48000)
                .into(),
            info.time_base.denominator.into(),
            256,
        )
        .unwrap();
        let first = pts.round_integer(Rounding::Floor).unwrap();
        assert_eq!(pts.compare(&R::integer(first)), std::cmp::Ordering::Equal);
        assert_eq!(
            first,
            samples.len() as i64,
            "continuous trimmed PCM required"
        );
        let pcm = normalize_pcm(
            &frame,
            if cfg!(target_endian = "little") {
                ByteOrder::Little
            } else {
                ByteOrder::Big
            },
            ChannelLayout::from_codec_name(&info.channel_layout).unwrap(),
            first,
            PcmLimits::default(),
            &|| false,
        )
        .unwrap();
        records.push(json!({"first":first,"frames":pcm.frames(),"format":info.format}));
        samples.extend_from_slice(pcm.samples());
    }
    decoder.close(&|| false).unwrap();
    assert_eq!(samples.len(), 144000);
    let normalized: Vec<u8> = samples.iter().flat_map(|v| v.to_le_bytes()).collect();
    fs::write(output.join("normalized.f32"), &normalized).unwrap();
    let digest = ContentDigest::from_sha256(Sha256::digest(fs::read(&fixture).unwrap()).into());
    let mut cases = vec![];
    for (name, rate, duration, reverse, effects) in [
        ("forward", 48000, 24000, false, false),
        ("reverse", 48000, 24000, true, false),
        ("gain-fade", 48000, 24000, false, true),
        ("rate-44100", 44100, 24000, false, false),
        ("slow", 48000, 48000, false, false),
    ] {
        let (mut d, mut host) = document(samples.clone(), 48000);
        d.audio_sample_rate = rate;
        host.binding.digest = digest.clone();
        let resource = d.resources.get_mut(&host.binding.resource).unwrap();
        resource.digest = digest.clone();
        resource.byte_length = ByteLength::new(length);
        track(&mut d, "main", "audio");
        clip(
            &mut d,
            "audio",
            "voice",
            range(0, duration),
            media(
                &host,
                if reverse {
                    mapping(24000, 0, duration)
                } else {
                    mapping(0, 24000, duration)
                },
            ),
        );
        if effects {
            d.clips.get_mut(&cid("voice")).unwrap().effects = vec![
                Effect::Gain {
                    milli_decibels: -6000,
                },
                Effect::Fade {
                    fade_in: t(4000),
                    fade_out: t(8000),
                    envelope: range(0, 24000),
                },
            ];
        }
        let c = CompiledDocument::new(d, CompileLimits::default(), &|| false).unwrap();
        let total = c.audio_sample_count().unwrap();
        let mut mixer = AudioMixer::new(&c, ChannelLayout::mono(), MixLimits::default()).unwrap();
        let mut file = File::create(output.join(format!("{name}.f32"))).unwrap();
        let mut first = 0;
        let mut peak = 0;
        let mut blocks = 0;
        while first < total {
            let n = (total - first).min([128, 997, 4096, 257][blocks % 4]) as u32;
            let block = mixer.render(&mut host, first, n, &|| false).unwrap();
            peak = peak.max(block.stats.peak_pcm_bytes);
            for value in block.pcm.samples() {
                file.write_all(&value.to_le_bytes()).unwrap();
            }
            first += i64::from(n);
            blocks += 1;
        }
        let bytes = fs::read(output.join(format!("{name}.f32"))).unwrap();
        cases.push(json!({"name":name,"sampleRate":rate,"frames":total,"blocks":blocks,"peakCorePcmBytes":peak,"sha256":format!("{:x}",Sha256::digest(&bytes))}));
    }
    fs::write(output.join("report.json"),serde_json::to_vec_pretty(&json!({"profile":PROFILE,"fixtureSha256":digest,"normalizedSha256":format!("{:x}",Sha256::digest(normalized)),"decodedFrames":records,"cases":cases})).unwrap()).unwrap();
}
