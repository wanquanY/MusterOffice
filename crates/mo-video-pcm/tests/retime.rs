mod retime_host;
use mo_common::Digest;
use mo_video_compile::{CompileLimits, CompiledDocument};
use mo_video_edit::*;
use mo_video_model::*;
use retime_host::{source::*, *};

fn compiled(document: VideoDocument) -> CompiledDocument {
    CompiledDocument::new(document, CompileLimits::default(), &|| false).unwrap()
}
#[test]
fn variable_rate_keeps_one_continuous_stage_and_random_reads_match() {
    let (d, mut host) = stepped();
    let d = compiled(d);
    let mut mixer = mixer(&d);
    // A cold query at the end must still materialize from the context origin.
    let end = render(&mut mixer, &mut host, 31000, 1000);
    let full = render(&mut mixer, &mut host, 0, 32000);
    assert_eq!(end.samples(), &full.samples()[31000..]);
    assert_eq!(host.jobs, 1); // 0.5x -> 1x -> 2x, not three independent DSP resets.
    for (at, n, hz) in [
        (4000, 8000, 220.),
        (17000, 5000, 440.),
        (25000, 2000, 880.),
        (29000, 2000, 1760.),
    ] {
        assert!((pitch(&full.samples()[at..at + n], 8000) - hz).abs() < 5.);
    }
    for first in [12345, 0, 31000, 15990, 23995, 456, 7000, 30000] {
        let n = (32000 - first).min(511);
        assert_eq!(
            render(&mut mixer, &mut host, first, n as u32).samples(),
            &full.samples()[first as usize..(first + n) as usize]
        );
    }
    assert_eq!(host.jobs, 1);
    assert!(host.peak_materializer_bytes < 128 << 10);
}
#[test]
fn split_and_repeated_trim_preserve_the_same_audio_and_cache_identity() {
    let (d, mut host) = stepped();
    let original = compiled(d.clone());
    let full = render(&mut mixer(&original), &mut host, 0, 32000);
    let snapshot = Snapshot::new(d, ValidationLimits::default(), &|| false).unwrap();
    let changed = prepare(
        &snapshot,
        &Transaction {
            contract: EditContract::V1,
            expected_digest: snapshot.digest().clone(),
            operations: vec![Operation::Split {
                targets: vec![SplitTarget {
                    clip: cid("a"),
                    right: cid("b"),
                }],
                at: t(20000),
                right_groups: vec![],
                linked_policy: LinkedPolicy::RequireAll,
            }],
        },
        EditLimits::default(),
        &|| false,
    )
    .unwrap();
    let split = compiled(changed.candidate.document().clone());
    assert_eq!(
        render(&mut mixer(&split), &mut host, 0, 32000).samples(),
        full.samples()
    );
    assert_eq!(host.jobs, 1);
    let trimmed = prepare(
        &changed.candidate,
        &Transaction {
            contract: EditContract::V1,
            expected_digest: changed.candidate.digest().clone(),
            operations: vec![Operation::Trim {
                clips: vec![cid("b")],
                edge: Edge::Start,
                delta: t(1000),
                ripple_tracks: vec![],
                linked_policy: LinkedPolicy::RequireAll,
            }],
        },
        EditLimits::default(),
        &|| false,
    )
    .unwrap();
    let trimmed = compiled(trimmed.candidate.document().clone());
    assert_eq!(
        render(&mut mixer(&trimmed), &mut host, 21000, 11000).samples(),
        &full.samples()[21000..]
    );
    assert_eq!(host.jobs, 1);
    assert_eq!(
        prepare_inverse(
            &changed.candidate,
            &changed.inverse,
            EditLimits::default(),
            &|| false
        )
        .unwrap()
        .candidate
        .digest(),
        snapshot.digest()
    );
}
#[test]
fn nested_policies_are_not_collapsed_to_net_speed() {
    let samples = (0..16000)
        .map(|i| (0.3 * (i as f64 * std::f64::consts::TAU * 440. / 8000.).sin()) as f32)
        .collect();
    let (mut d, raw) = document(samples, 8000);
    d.sequences.insert(
        sid("child"),
        Sequence {
            id: sid("child"),
            tracks: vec![],
        },
    );
    track(&mut d, "child", "inside");
    let source = media(&raw, mapping(0, 16000, 32000));
    clip(&mut d, "inside", "inner", range(0, 32000), source);
    d.clips.get_mut(&cid("inner")).unwrap().audio.rate = AudioRatePolicy::PreservePitch;
    track(&mut d, "main", "outside");
    clip(
        &mut d,
        "outside",
        "outer",
        range(0, 16000),
        ClipSource::Sequence {
            sequence: sid("child"),
            time_map: mapping(0, 32000, 16000),
        },
    );
    let mut host = Host::new(raw);
    let c = compiled(d.clone());
    let out = render(&mut mixer(&c), &mut host, 0, 16000);
    assert!((pitch(&out.samples()[4000..12000], 8000) - 880.).abs() < 3.);
    assert_eq!(host.jobs, 1);
    d.clips.get_mut(&cid("outer")).unwrap().audio.rate = AudioRatePolicy::PreservePitch;
    let c = compiled(d);
    let out = render(&mut mixer(&c), &mut host, 0, 16000);
    assert!((pitch(&out.samples()[4000..12000], 8000) - 440.).abs() < 3.);
    assert_eq!(host.jobs, 2);
}
#[test]
fn derived_publication_and_builder_budget_retries_never_repeat_dsp() {
    let (d, mut host) = stepped();
    host.fail_publication = true;
    let c = compiled(d.clone());
    let retry = render(&mut mixer(&c), &mut host, 0, 32000);
    let (_, mut plain) = stepped();
    let clean = render(&mut mixer(&c), &mut plain, 0, 32000);
    assert_eq!(retry.samples(), clean.samples());
    assert_eq!(host.jobs, 1);
}
#[test]
fn unrelated_edits_reuse_audio_but_numerical_profiles_do_not() {
    let (mut d, mut host) = stepped();
    let c = compiled(d.clone());
    let key = c.audio_sequence_digest(&sid("main")).unwrap().clone();
    let full = render(&mut mixer(&c), &mut host, 0, 32000);
    d.title = "new title".into();
    d.canvas.width = 800;
    d.clips.get_mut(&cid("a")).unwrap().name = "new clip title".into();
    let c = compiled(d.clone());
    assert_eq!(c.audio_sequence_digest(&sid("main")).unwrap(), &key);
    assert_eq!(
        render(&mut mixer(&c), &mut host, 0, 32000).samples(),
        full.samples()
    );
    assert_eq!(host.jobs, 1);
    host.profile = Digest::from_sha256([8; 32]);
    render(&mut mixer(&c), &mut host, 0, 32000);
    assert_eq!(host.jobs, 2);
    d.clips
        .get_mut(&cid("a"))
        .unwrap()
        .effects
        .push(Effect::Gain {
            milli_decibels: -6000,
        });
    let changed = compiled(d);
    assert_ne!(changed.audio_sequence_digest(&sid("main")).unwrap(), &key);
    let quiet = render(&mut mixer(&changed), &mut host, 0, 32000);
    assert_eq!(host.jobs, 2); // Own gain is applied after the shared retime stage.
    assert!(
        quiet
            .samples()
            .iter()
            .zip(full.samples())
            .any(|(a, b)| (a - b).abs() > 0.01)
    );
}
