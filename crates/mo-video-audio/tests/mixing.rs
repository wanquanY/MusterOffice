mod support;
use mo_video_audio::*;
use mo_video_compile::{CompileLimits, CompiledDocument};
use mo_video_model::*;
use std::cell::Cell;
use support::*;
fn compiled(d: VideoDocument) -> CompiledDocument {
    CompiledDocument::new(d, CompileLimits::default(), &|| false).unwrap()
}
#[test]
fn overlapping_tracks_mix_in_f64_without_clipping_or_normalizing() {
    let (mut d, mut host) = document(vec![0.75; 1024], 8000);
    for name in ["speech", "music"] {
        track(&mut d, "main", name);
        clip(
            &mut d,
            name,
            name,
            range(0, 1024),
            media(&host, mapping(0, 1024, 1024)),
        );
    }
    let c = compiled(d);
    let mut mixer = AudioMixer::new(&c, ChannelLayout::stereo(), MixLimits::default()).unwrap();
    let out = mixer.render(&mut host, 0, 1024, &|| false).unwrap();
    assert_eq!(out.pcm.samples(), vec![1.5; 2048]);
    assert_eq!(out.stats.voices, 2);
    assert_eq!(out.stats.source_frames, 2048);
    assert_eq!(out.stats.peak_pcm_bytes, 40960); // f64 mix + final block + mono source + stereo source/voice
}
#[test]
fn fade_phase_trimmed_before_clip_and_reverse_are_partition_independent() {
    let (mut d, mut host) = document(vec![1.; 2048], 8000);
    track(&mut d, "main", "audio");
    clip(
        &mut d,
        "audio",
        "voice",
        range(3, 1027),
        media(&host, mapping(1024, 0, 1024)),
    );
    d.clips.get_mut(&cid("voice")).unwrap().effects = vec![
        Effect::Fade {
            fade_in: t(700),
            fade_out: t(400),
            envelope: range(-200, 1024),
        },
        Effect::Gain {
            milli_decibels: -6000,
        },
    ];
    let c = compiled(d);
    let mut mixer = AudioMixer::new(&c, ChannelLayout::mono(), MixLimits::default()).unwrap();
    let full = mixer.render(&mut host, 0, 1027, &|| false).unwrap();
    for (first, n) in [
        (0, 3),
        (3, 99),
        (102, 200),
        (302, 201),
        (503, 500),
        (1003, 24),
    ] {
        let block = mixer.render(&mut host, first, n, &|| false).unwrap();
        assert_eq!(
            block.pcm.samples(),
            &full.pcm.samples()[first as usize..first as usize + n as usize]
        );
    }
    let gain = libm::pow(10., -0.3);
    for i in 0..1024 {
        let expected = (((i + 200) as f64 / 700.)
            .min((1024 - i) as f64 / 400.)
            .min(1.))
            * gain;
        assert!((f64::from(full.pcm.samples()[i + 3]) - expected).abs() < 4e-8);
    }
}
#[test]
fn nested_reversals_restore_sample_order_and_group_gain_is_applied_after_child_mix() {
    let values: Vec<f32> = (0..1024).map(|n| n as f32 / 1024.).collect();
    let (mut d, mut host) = document(values.clone(), 8000);
    d.sequences.insert(
        sid("child"),
        Sequence {
            id: sid("child"),
            tracks: vec![],
        },
    );
    track(&mut d, "main", "outer");
    track(&mut d, "child", "inner");
    clip(
        &mut d,
        "inner",
        "voice",
        range(0, 1024),
        media(&host, mapping(1024, 0, 1024)),
    );
    clip(
        &mut d,
        "outer",
        "group",
        range(0, 1024),
        ClipSource::Sequence {
            sequence: sid("child"),
            time_map: mapping(1024, 0, 1024),
        },
    );
    d.clips.get_mut(&cid("group")).unwrap().effects = vec![Effect::Gain {
        milli_decibels: -6000,
    }];
    let c = compiled(d);
    let mut mixer = AudioMixer::new(&c, ChannelLayout::mono(), MixLimits::default()).unwrap();
    let out = mixer.render(&mut host, 0, 1024, &|| false).unwrap();
    let gain = libm::pow(10., -0.3);
    for (v, expected) in out.pcm.samples().iter().zip(values) {
        assert_eq!(*v, (f64::from(expected) * gain) as f32);
    }
    assert_eq!(out.stats.voices, 2);
}
#[test]
fn differing_source_rate_is_converted_without_changing_authored_pitch_policy() {
    let (mut d, mut host) = document(
        (0..4096)
            .map(|n| libm::sin(2. * std::f64::consts::PI * 1000. * n as f64 / 16000.) as f32)
            .collect(),
        16000,
    );
    track(&mut d, "main", "audio");
    clip(
        &mut d,
        "audio",
        "voice",
        range(0, 2048),
        media(&host, mapping(0, 2048, 2048)),
    );
    d.clips.get_mut(&cid("voice")).unwrap().audio.rate = AudioRatePolicy::PreservePitch;
    let c = compiled(d);
    let mut mixer = AudioMixer::new(&c, ChannelLayout::mono(), MixLimits::default()).unwrap();
    let out = mixer.render(&mut host, 200, 1000, &|| false).unwrap();
    for (i, v) in out.pcm.samples().iter().enumerate() {
        let expected = libm::sin(2. * std::f64::consts::PI * 1000. * (i + 200) as f64 / 8000.);
        assert!((f64::from(*v) - expected).abs() < 3e-6);
    }
}
#[test]
fn preserve_pitch_stages_are_not_silently_replaced_by_net_resampling() {
    let (mut d, mut host) = document(vec![1.; 2048], 8000);
    track(&mut d, "main", "audio");
    clip(
        &mut d,
        "audio",
        "voice",
        range(0, 1024),
        media(&host, mapping(0, 2048, 1024)),
    );
    d.clips.get_mut(&cid("voice")).unwrap().audio.rate = AudioRatePolicy::PreservePitch;
    let c = compiled(d);
    let mut mixer = AudioMixer::new(&c, ChannelLayout::mono(), MixLimits::default()).unwrap();
    assert!(matches!(
        mixer.render(&mut host, 0, 100, &|| false),
        Err(AudioError::Unsupported(_))
    ));
    assert_eq!(host.reads, 0);
}
#[test]
fn budgets_stale_metadata_and_every_cancellation_checkpoint_preserve_future_renders() {
    let (mut d, mut host) = document(vec![0.25; 2048], 8000);
    track(&mut d, "main", "audio");
    clip(
        &mut d,
        "audio",
        "voice",
        range(0, 1024),
        media(&host, mapping(0, 2048, 1024)),
    );
    let c = compiled(d);
    let mut tiny = AudioMixer::new(
        &c,
        ChannelLayout::mono(),
        MixLimits {
            max_active_pcm_bytes: 16,
            ..MixLimits::default()
        },
    )
    .unwrap();
    assert!(matches!(
        tiny.render(&mut host, 0, 100, &|| false),
        Err(AudioError::Budget)
    ));
    let mut mixer = AudioMixer::new(&c, ChannelLayout::mono(), MixLimits::default()).unwrap();
    let reference = mixer.render(&mut host, 100, 100, &|| false).unwrap();
    let calls = Cell::new(0);
    mixer
        .render(&mut host, 100, 100, &|| {
            calls.set(calls.get() + 1);
            false
        })
        .unwrap();
    for stop in 1..=calls.get() {
        let count = Cell::new(0);
        let result = mixer.render(&mut host, 100, 100, &|| {
            count.set(count.get() + 1);
            count.get() == stop
        });
        assert!(result.is_err());
        assert_eq!(
            mixer
                .render(&mut host, 100, 100, &|| false)
                .unwrap()
                .pcm
                .samples(),
            reference.pcm.samples()
        );
    }
    host.asset.available.end -= 1;
    assert!(matches!(
        mixer.render(&mut host, 100, 100, &|| false),
        Err(AudioError::Invalid(_))
    ));
}

#[test]
fn fractional_rate_anchor_hold_and_reverse_cuts_keep_one_global_audio_phase() {
    let (mut d, mut host) = document(
        (0..1024)
            .map(|n| ((n * 1637) % 64000 - 32000) as f32 / 32768.)
            .collect(),
        8000,
    );
    track(&mut d, "main", "audio");
    let half = |n| mo_common::RationalTime::new(n, 16000).unwrap();
    let map = TimeMap {
        anchors: vec![
            TimeAnchor {
                local: t(0),
                source: t(256),
            },
            TimeAnchor {
                local: half(201),
                source: t(457),
            },
            TimeAnchor {
                local: half(301),
                source: t(457),
            },
            TimeAnchor {
                local: t(256),
                source: half(703),
            },
        ],
    };
    clip(&mut d, "audio", "voice", range(0, 256), media(&host, map));
    let c = compiled(d);
    let mut mixer = AudioMixer::new(&c, ChannelLayout::mono(), MixLimits::default()).unwrap();
    let full = mixer.render(&mut host, 0, 256, &|| false).unwrap();
    assert_eq!(&full.pcm.samples()[101..151], vec![0.; 50]);
    for (first, frames) in [
        (0, 100),
        (100, 1),
        (101, 49),
        (150, 2),
        (152, 37),
        (189, 67),
    ] {
        let block = mixer.render(&mut host, first, frames, &|| false).unwrap();
        assert_eq!(
            block.pcm.samples(),
            &full.pcm.samples()[first as usize..first as usize + frames as usize]
        );
    }
}

#[test]
fn owned_revision_keeps_cache_and_matches_borrowed_then_releases_document() {
    use std::sync::Arc;
    let (mut d, mut host) = document((0..8192).map(|n| (n as f32 * 0.1).sin()).collect(), 16000);
    track(&mut d, "main", "audio");
    clip(
        &mut d,
        "audio",
        "voice",
        range(0, 2048),
        media(&host, mapping(100, 3100, 2048)),
    );
    let c = Arc::new(compiled(d));
    let weak = Arc::downgrade(&c);
    let mut borrowed = AudioMixer::new(&c, ChannelLayout::stereo(), MixLimits::default()).unwrap();
    let mut owned =
        AudioMixer::from_shared(c.clone(), ChannelLayout::stereo(), MixLimits::default()).unwrap();
    assert_eq!(Arc::strong_count(&c), 2);
    for (first, frames) in [(0, 512), (1200, 217), (4, 91), (0, 512)] {
        let a = borrowed
            .render(&mut host, first, frames, &|| false)
            .unwrap();
        let b = owned.render(&mut host, first, frames, &|| false).unwrap();
        assert_eq!(a.pcm.samples(), b.pcm.samples());
        assert_eq!(a.stats.retained_kernel_bytes, b.stats.retained_kernel_bytes);
        assert!(b.stats.retained_kernel_bytes > 0);
        owned.clear();
    }
    drop(borrowed);
    drop(c);
    assert!(weak.upgrade().is_some());
    owned.render(&mut host, 99, 50, &|| false).unwrap();
    drop(owned);
    assert!(weak.upgrade().is_none());
}
