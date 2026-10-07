#![cfg(feature = "native")]
#[path = "native_visual/support.rs"]
mod actual;
mod support;
use mo_video_runtime::{
    authoring::model::*,
    compile::*,
    export::*,
    media::*,
    native::*,
    render::*,
    source::{native::*, *},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Write,
    path::PathBuf,
};
#[test]
#[ignore = "requires pinned worker, actual inputs and independent oracle; video-visual-check.py"]
fn actual_indexed_composition_matches_oracle_and_preview_matches_encoded_input() {
    let input = PathBuf::from(std::env::var_os("MO_VISUAL_INPUT").unwrap());
    let output = PathBuf::from(std::env::var_os("MO_VISUAL_OUTPUT").unwrap());
    fs::create_dir(&output).unwrap();
    let oracle: Value =
        serde_json::from_slice(&fs::read(std::env::var_os("MO_VISUAL_ORACLE").unwrap()).unwrap())
            .unwrap();
    let bundle = WorkerBundle::open(
        PathBuf::from(std::env::var_os("MO_MEDIA_TEST_BUNDLE").unwrap()),
        std::env::var("MO_MEDIA_TEST_BUNDLE_SHA256").unwrap(),
    )
    .unwrap();
    let (mut visual, binding, pids) = actual::source(&bundle, &input);
    let mut raw = NativeVideoDecoder::open_raw_track(
        &bundle,
        actual::reader(&input),
        input.metadata().unwrap().len(),
        actual::options(),
        oracle["trackId"].as_u64().unwrap() as u32,
        &|| false,
    )
    .unwrap();
    let mut normalized = BTreeMap::new();
    let mut raw_report = vec![];
    while let Some(frame) = raw.next(&|| false).unwrap() {
        let mut digest = Sha256::new();
        for plane in &frame.planes {
            digest.update(&plane.bytes);
        }
        let pts = frame.info.pts_ticks.unwrap();
        let want = oracle["raw"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["pts"].as_i64() == Some(pts))
            .unwrap();
        assert_eq!(
            format!("{:x}", digest.finalize()),
            want["sha256"].as_str().unwrap()
        );
        let image = normalize_frame(&frame, actual::color(), Default::default(), &|| false)
            .unwrap()
            .image;
        let matrix = SourceGeometry::from_media(&frame.info, raw.stream())
            .unwrap()
            .contain(
                (image.width(), image.height()),
                (image.width(), image.height()),
            )
            .unwrap();
        assert_eq!(
            matrix.coefficients(),
            [1., 0., 0., 1., 0., 0.],
            "fixtures explicitly require square identity presentation"
        );
        raw_report.push(json!({"pts":pts,"sha256":want["sha256"]}));
        normalized.insert(pts, image);
    }
    assert_eq!(raw_report.len(), oracle["raw"].as_array().unwrap().len());
    drop(raw);
    let mut reports = vec![];
    for reverse in [false, true] {
        let name = if reverse { "reverse" } else { "forward" };
        let doc = support::document(&binding, input.metadata().unwrap().len(), reverse);
        let rows = oracle[name].as_array().unwrap();
        assert_eq!(doc.frame_count().unwrap() as usize, rows.len());
        let mut expected = vec![Vec::new(); rows.len()];
        let mut stats = vec![];
        // Reverse scrub order before export exercises cached and uncached backward seeks.
        for row in rows.iter().rev() {
            let number = row["number"].as_i64().unwrap();
            let plan = doc
                .frame(number, Default::default(), &|| false)
                .unwrap()
                .value;
            let VisualSource::Media { mapping, .. } = &plan.layers[1].source else {
                panic!()
            };
            assert_eq!(
                mapping.sample.side,
                if reverse {
                    SampleSide::Before
                } else {
                    SampleSide::After
                }
            );
            let at: ExactRatio = serde_json::from_value(row["time"].clone()).unwrap();
            assert!(mapping.sample.time.compare(&at).is_eq());
            let rendered = render_frame(
                &plan,
                &mut visual,
                Sampling::Nearest,
                Default::default(),
                &|| false,
            )
            .unwrap();
            if let Some(pts) = row["pts"].as_i64() {
                assert_eq!(
                    rendered.image.pixels(),
                    normalized[&pts].pixels(),
                    "{name} {number}"
                );
            } else {
                let mut background = plan.clone();
                background.layers.pop();
                let clean = render_frame(
                    &background,
                    &mut visual,
                    Sampling::Nearest,
                    Default::default(),
                    &|| false,
                )
                .unwrap();
                assert_eq!(rendered.image.pixels(), clean.image.pixels());
                assert_eq!(visual.last_stats().source.decoded_frames, 0);
            }
            let image = rendered
                .image
                .to_i420(
                    I420Options {
                        background_srgb: [0; 3],
                        tone_map: None,
                        max_bytes: 1 << 20,
                        max_temporary_bytes: 1 << 20,
                    },
                    &|| false,
                )
                .unwrap();
            expected[number as usize] = image
                .planes()
                .iter()
                .flat_map(|p| p.iter().copied())
                .collect();
            let s = visual.last_stats();
            stats.push(json!({"number":number,"pts":row["pts"],"normalized":s.normalized,"colorCacheHit":s.normalized_cache_hit,"colorBytes":s.retained_bytes,"rawBytes":s.source.retained_bytes}));
        }
        let mut expected_file = File::create(output.join(format!("{name}.yuv"))).unwrap();
        for frame in &expected {
            expected_file.write_all(frame).unwrap();
        }
        drop(expected_file);
        let plan = ExportPlan::new(
            &doc,
            ExportOptions {
                profile: EncodeProfile::Ffv1PcmMp4,
                video_bit_rate: 2_000_000,
                gop_frames: 24,
                audio: ExportAudio::Omit,
                background_srgb: [0; 3],
                tone_map: None,
                sampling: Sampling::Nearest,
            },
            Default::default(),
            &|| false,
        )
        .unwrap();
        let config = plan.configuration();
        let encoder = NativeEncoder::open(
            &bundle,
            actual::Output(File::create(output.join(format!("{name}.mp4"))).unwrap()),
            config,
            EncoderOptions {
                encode: Default::default(),
                process: actual::options().process,
                open_timeout: actual::options().open_timeout,
                call_timeout: actual::options().call_timeout,
            },
            || false,
        )
        .unwrap();
        let mut export = ExportSession::new(
            plan,
            actual::Encoder {
                inner: encoder,
                expected,
                count: 0,
            },
        )
        .unwrap();
        let candidate = loop {
            match export
                .step(&mut visual, &mut actual::NoAudio, &|| false)
                .unwrap()
            {
                ExportStep::Advanced(_) => (),
                ExportStep::WaitingForAudio(_) => panic!("silent"),
                ExportStep::ReadyToVerify(candidate) => break candidate,
            }
        };
        assert_eq!(candidate.encoded.video_frames, rows.len() as u64);
        reports.push(json!({"direction":name,"frames":rows.len(),"config":config,"stats":stats}));
    }
    drop(visual);
    for pid in pids {
        actual::reaped(pid);
    }
    // A cancellation after color-cache population must also physically reap both processes.
    let (mut source, _, pids) = actual::source(&bundle, &input);
    let row = oracle["forward"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["pts"].is_i64())
        .unwrap();
    let sample = TimeSample {
        time: serde_json::from_value(row["time"].clone()).unwrap(),
        side: SampleSide::After,
    };
    source.media(&binding, &sample, &|| false).unwrap();
    assert!(matches!(
        source.media(&binding, &sample, &|| true),
        Err(RenderError::Cancelled)
    ));
    assert!(source.is_stopped());
    assert_eq!(source.retained_bytes(), 0);
    for pid in pids {
        actual::reaped(pid);
    }
    fs::write(
        output.join("report.json"),
        serde_json::to_vec_pretty(
            &json!({"binding":binding,"raw":raw_report,"directions":reports}),
        )
        .unwrap(),
    )
    .unwrap();
}
