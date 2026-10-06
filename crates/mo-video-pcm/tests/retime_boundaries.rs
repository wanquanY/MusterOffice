mod retime_host;
use mo_common::RationalTime;
use mo_time::ExactRatio as R;
use mo_video_audio::*;
use mo_video_compile::{CompileLimits, CompiledDocument};
use mo_video_edit::*;
use mo_video_model::*;
use retime_host::{source::*, *};

fn compiled(d: VideoDocument) -> CompiledDocument {
    CompiledDocument::new(d, CompileLimits::default(), &|| false).unwrap()
}
fn edit(d: VideoDocument, operations: Vec<Operation>) -> VideoDocument {
    let base = Snapshot::new(d, ValidationLimits::default(), &|| false).unwrap();
    prepare(
        &base,
        &Transaction {
            contract: EditContract::V1,
            expected_digest: base.digest().clone(),
            operations,
        },
        EditLimits::default(),
        &|| false,
    )
    .unwrap()
    .candidate
    .document()
    .clone()
}
fn trim(delta: i64) -> Operation {
    Operation::Trim {
        clips: vec![cid("a")],
        edge: Edge::Start,
        delta: t(delta),
        ripple_tracks: vec![],
        linked_policy: LinkedPolicy::RequireAll,
    }
}
#[test]
fn extending_trim_restores_hidden_rate_anchors_and_original_sound() {
    let (d, mut host) = stepped();
    let full = render(&mut mixer(&compiled(d.clone())), &mut host, 0, 32000);
    let shortened = edit(d, vec![trim(26000)]);
    let restored = edit(shortened, vec![trim(-24000)]);
    let c = compiled(restored);
    assert_eq!(
        render(&mut mixer(&c), &mut host, 2000, 30000).samples(),
        &full.samples()[2000..]
    );
    assert_eq!(host.jobs, 1);
}
#[test]
fn context_checks_both_sets_of_breakpoints_and_remains_cancellable() {
    let visible = mapping(0, 8000, 8000);
    let mut context = AudioRenderContext {
        time_map: visible.clone(),
        offset: t(0),
    };
    assert!(
        context
            .matches_window(&visible, &R::integer(1), 256, &|| false)
            .unwrap()
    );
    context.time_map.anchors.insert(
        1,
        TimeAnchor {
            local: t(4000),
            source: t(5000),
        },
    );
    // The endpoints agree, but a hidden speed change must not pass validation.
    assert!(
        !context
            .matches_window(&visible, &R::integer(1), 256, &|| false)
            .unwrap()
    );
    assert!(matches!(
        context.matches_window(&visible, &R::integer(1), 256, &|| true),
        Err(ValidationError::Cancelled)
    ));
    let (mut d, _) = stepped();
    d.clips.get_mut(&cid("a")).unwrap().audio_context = Some(context);
    let report = validate(&d, ValidationLimits::default(), &|| false).unwrap();
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.path.ends_with("/audioContext"))
    );
}
#[test]
fn explicit_source_or_rate_edits_clear_context_and_undo_restores_it() {
    let (d, host) = stepped();
    let d = edit(d, vec![trim(26000)]);
    assert!(d.clips[&cid("a")].audio_context.is_some());
    let map = mapping(10000, 22000, 6000);
    for operation in [
        Operation::Slip {
            clips: vec![cid("a")],
            delta: t(-100),
            linked_policy: LinkedPolicy::RequireAll,
        },
        Operation::ReplaceSource {
            clip: cid("a"),
            source: media(&host.raw, map.clone()),
        },
        Operation::Retime {
            targets: vec![RetimeTarget {
                clip: cid("a"),
                time_map: map,
                audio: d.clips[&cid("a")].audio,
            }],
            effects: EffectTimingPolicy::ScaleToDuration,
            ripple_tracks: vec![],
            linked_policy: LinkedPolicy::RequireAll,
        },
    ] {
        let base = Snapshot::new(d.clone(), ValidationLimits::default(), &|| false).unwrap();
        let prepared = prepare(
            &base,
            &Transaction {
                contract: EditContract::V1,
                expected_digest: base.digest().clone(),
                operations: vec![operation],
            },
            EditLimits::default(),
            &|| false,
        )
        .unwrap();
        assert!(
            prepared.candidate.document().clips[&cid("a")]
                .audio_context
                .is_none()
        );
        assert_eq!(
            prepare_inverse(
                &prepared.candidate,
                &prepared.inverse,
                EditLimits::default(),
                &|| false
            )
            .unwrap()
            .candidate
            .digest(),
            base.digest()
        );
    }
}
#[test]
fn reverse_and_hold_form_separate_continuous_stages() {
    let (mut d, raw) = document(
        (0..16000)
            .map(|i| (0.3 * (i as f64 * std::f64::consts::TAU * 440. / 8000.).sin()) as f32)
            .collect(),
        8000,
    );
    track(&mut d, "main", "audio");
    let map = TimeMap {
        anchors: [(0, 16000), (8000, 0), (10000, 0), (26000, 8000)]
            .map(|(l, s)| TimeAnchor {
                local: t(l),
                source: t(s),
            })
            .to_vec(),
    };
    clip(&mut d, "audio", "a", range(0, 26000), media(&raw, map));
    d.clips.get_mut(&cid("a")).unwrap().audio.rate = AudioRatePolicy::PreservePitch;
    let mut host = Host::new(raw);
    let out = render(&mut mixer(&compiled(d.clone())), &mut host, 0, 26000);
    assert!(out.samples()[8000..10000].iter().all(|v| *v == 0.));
    for window in [&out.samples()[2000..6000], &out.samples()[14000..22000]] {
        assert!((pitch(window, 8000) - 440.).abs() < 3.);
    }
    assert_eq!(host.jobs, 2);
    d.clips.get_mut(&cid("a")).unwrap().audio.reverse = ReverseAudioPolicy::Mute;
    let out = render(&mut mixer(&compiled(d)), &mut host, 0, 26000);
    assert!(out.samples()[..10000].iter().all(|v| *v == 0.));
    assert_eq!(host.jobs, 2);
}
#[test]
fn fractional_split_and_integer_move_reuse_context_but_fractional_move_rephases() {
    let (d, mut host) = stepped();
    let full = render(&mut mixer(&compiled(d.clone())), &mut host, 0, 32000);
    let split = edit(
        d,
        vec![Operation::Split {
            targets: vec![SplitTarget {
                clip: cid("a"),
                right: cid("b"),
            }],
            at: RationalTime::new(40001, 16000).unwrap(),
            right_groups: vec![],
            linked_policy: LinkedPolicy::RequireAll,
        }],
    );
    assert_eq!(
        render(&mut mixer(&compiled(split.clone())), &mut host, 0, 32000).samples(),
        full.samples()
    );
    let shifted = edit(
        split.clone(),
        vec![Operation::Move {
            clips: vec![cid("a"), cid("b")],
            delta: t(100),
            target_track: None,
            linked_policy: LinkedPolicy::RequireAll,
        }],
    );
    assert_eq!(
        render(&mut mixer(&compiled(shifted)), &mut host, 100, 32000).samples(),
        full.samples()
    );
    assert_eq!(host.jobs, 1);
    let shifted = edit(
        split,
        vec![Operation::Move {
            clips: vec![cid("a"), cid("b")],
            delta: RationalTime::new(1, 16000).unwrap(),
            target_track: None,
            linked_policy: LinkedPolicy::RequireAll,
        }],
    );
    render(&mut mixer(&compiled(shifted)), &mut host, 1, 32000);
    assert_eq!(host.jobs, 2);
}
#[test]
fn retime_plan_is_admitted_before_pcm_io_and_unit_maps_fit_small_cache() {
    let (mut d, mut host) = stepped();
    let c = compiled(d.clone());
    let limits = MixLimits {
        max_retime_plan_bytes: 1024,
        ..MixLimits::default()
    };
    let mut m = AudioMixer::new(&c, ChannelLayout::mono(), limits).unwrap();
    assert!(matches!(
        m.render(&mut host, 0, 100, &|| false),
        Err(AudioError::Budget)
    ));
    assert_eq!(host.raw.reads, 0);
    assert_eq!(host.jobs, 0);
    if let ClipSource::Media { time_map, .. } = &mut d.clips.get_mut(&cid("a")).unwrap().source {
        time_map.anchors = (0..=1000)
            .map(|i| TimeAnchor {
                local: t(i * 32),
                source: t(i * 32),
            })
            .collect();
    }
    let c = compiled(d);
    let mut m = AudioMixer::new(&c, ChannelLayout::mono(), limits).unwrap();
    let out = m.render(&mut host, 0, 100, &|| false).unwrap();
    assert_eq!(out.stats.retained_retime_plan_bytes, 1024);
    assert_eq!(host.jobs, 0);
}

#[test]
fn nested_audio_change_invalidates_parent_and_hidden_context_survives_child_trim() {
    let (mut d, raw) = document(vec![0.2; 16000], 8000);
    d.sequences.insert(
        sid("child"),
        Sequence {
            id: sid("child"),
            tracks: vec![],
        },
    );
    track(&mut d, "child", "inside");
    clip(
        &mut d,
        "inside",
        "inner",
        range(0, 32000),
        media(&raw, mapping(0, 16000, 32000)),
    );
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
    for id in ["inner", "outer"] {
        d.clips.get_mut(&cid(id)).unwrap().audio.rate = AudioRatePolicy::PreservePitch;
    }
    let mut host = Host::new(raw);
    let c = compiled(d.clone());
    render(&mut mixer(&c), &mut host, 0, 16000);
    assert_eq!(host.jobs, 2);
    let request = host
        .requests
        .values()
        .find(|r| {
            matches!(
                r.plan().source(),
                mo_video_compile::AudioSpanSource::Sequence { .. }
            )
        })
        .unwrap()
        .clone();
    d.clips
        .get_mut(&cid("inner"))
        .unwrap()
        .effects
        .push(Effect::Gain {
            milli_decibels: -6000,
        });
    let c = compiled(d.clone());
    let reads = host.raw.reads;
    assert!(matches!(
        mixer(&c).render_retime_input(&mut host, &request, 0, 128, &|| false),
        Err(AudioError::Invalid("stale retime input dependency"))
    ));
    assert_eq!(reads, host.raw.reads);
    render(&mut mixer(&c), &mut host, 0, 16000);
    assert_eq!(host.jobs, 3); // child PCM is reused, parent content changes.
    let d = edit(
        d,
        vec![
            Operation::Trim {
                clips: vec![cid("outer")],
                edge: Edge::End,
                delta: t(-4000),
                ripple_tracks: vec![],
                linked_policy: LinkedPolicy::RequireAll,
            },
            Operation::Trim {
                clips: vec![cid("inner")],
                edge: Edge::End,
                delta: t(-8000),
                ripple_tracks: vec![],
                linked_policy: LinkedPolicy::RequireAll,
            },
        ],
    );
    // The parent visible range ends at the new child end; its full processing
    // context still extends farther. Explicit child-tail silence is valid.
    let out = render(&mut mixer(&compiled(d)), &mut host, 0, 12000);
    assert!(out.samples().iter().all(|v| v.is_finite()));
    assert_eq!(host.jobs, 4);
}

#[test]
fn host_requests_do_not_pin_the_bounded_plan_cache() {
    let (mut d, raw) = document(vec![0.2; 16000], 8000);
    track(&mut d, "main", "audio");
    clip(
        &mut d,
        "audio",
        "a",
        range(0, 8000),
        media(&raw, mapping(0, 4000, 8000)),
    );
    clip(
        &mut d,
        "audio",
        "b",
        range(8000, 16000),
        media(&raw, mapping(4000, 16000, 8000)),
    );
    for id in ["a", "b"] {
        d.clips.get_mut(&cid(id)).unwrap().audio.rate = AudioRatePolicy::PreservePitch;
    }
    let c = compiled(d);
    let mut host = Host::new(raw);
    // One two-anchor plan fits, two do not. Host retains both requests/tiles.
    let mut m = AudioMixer::new(
        &c,
        ChannelLayout::mono(),
        MixLimits {
            max_retime_plan_bytes: 6000,
            ..MixLimits::default()
        },
    )
    .unwrap();
    let a = render(&mut m, &mut host, 0, 1000);
    let b = render(&mut m, &mut host, 8000, 1000);
    assert_eq!(render(&mut m, &mut host, 0, 1000).samples(), a.samples());
    assert_eq!(render(&mut m, &mut host, 8000, 1000).samples(), b.samples());
    assert_eq!(host.jobs, 2);
    assert_eq!(host.requests.len(), 2);
    assert!(
        m.render(&mut host, 0, 1000, &|| false)
            .unwrap()
            .stats
            .retained_retime_plan_bytes
            <= 6000
    );
}
