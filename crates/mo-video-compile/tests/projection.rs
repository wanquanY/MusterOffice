mod support;
use mo_common::RationalTime;
use mo_time::ExactRatio as R;
use mo_video_compile::*;
use mo_video_model::*;
use std::cell::Cell;
use support::*;

fn compile(d: VideoDocument) -> CompiledDocument {
    CompiledDocument::new(d, CompileLimits::default(), &|| false).unwrap()
}
fn visual(c: &CompiledDocument, n: i64) -> Vec<VisualNode> {
    c.visual_at(&TimeSample::after(t(n)), QueryLimits::default(), &|| false)
        .unwrap()
        .value
}
fn audio(c: &CompiledDocument, n: i64) -> Vec<AudioNode> {
    c.audio_at(&TimeSample::after(t(n)), QueryLimits::default(), &|| false)
        .unwrap()
        .value
}
fn sample(n: i64, side: SampleSide) -> TimeSample {
    TimeSample {
        time: R::integer(n),
        side,
    }
}

#[test]
fn reverse_at_nested_end_selects_last_child_without_quantization_or_epsilon() {
    let mut d = doc();
    sequence(&mut d, "child");
    track(&mut d, "main", "outer", TrackKind::Video);
    track(&mut d, "child", "inner", TrackKind::Video);
    clip(&mut d, "inner", "first", range(0, 2), solid());
    clip(&mut d, "inner", "last", range(2, 4), solid());
    clip(
        &mut d,
        "outer",
        "group",
        range(0, 4),
        nested("child", map(4, 0, 4)),
    );
    let c = compile(d);
    let nodes = visual(&c, 0);
    let VisualSource::Group {
        layers, mapping, ..
    } = &nodes[0].source
    else {
        panic!()
    };
    assert_eq!(mapping.sample, sample(4, SampleSide::Before));
    assert_eq!(layers[0].clip, cid("last"));
    assert_eq!(layers[0].local, sample(2, SampleSide::Before));
    let at_cut = visual(&c, 2);
    let VisualSource::Group { layers, .. } = &at_cut[0].source else {
        panic!()
    };
    assert_eq!(layers[0].clip, cid("first"));
    assert!(visual(&c, 4).is_empty());
}

#[test]
fn group_opacity_and_transform_stay_above_child_compositing() {
    let mut d = doc();
    sequence(&mut d, "child");
    track(&mut d, "main", "outer", TrackKind::Video);
    track(&mut d, "child", "back", TrackKind::Video);
    track(&mut d, "child", "front", TrackKind::Graphics);
    clip(&mut d, "back", "background", range(0, 4), solid());
    clip(&mut d, "front", "foreground", range(0, 4), solid());
    clip(
        &mut d,
        "outer",
        "group",
        range(0, 4),
        nested("child", map(0, 4, 4)),
    );
    d.clips
        .get_mut(&cid("group"))
        .unwrap()
        .effects
        .push(Effect::Transform {
            x_milli_pixels: 500,
            y_milli_pixels: -100,
            scale_x_ppm: 1_000_000,
            scale_y_ppm: 1_000_000,
            rotation_milli_degrees: 90_000,
            opacity_ppm: 500_000,
        });
    let nodes = visual(&compile(d), 1);
    assert!(matches!(
        nodes[0].effects[0],
        VisualEffect::Transform {
            opacity_ppm: 500_000,
            ..
        }
    ));
    let VisualSource::Group { layers, .. } = &nodes[0].source else {
        panic!()
    };
    assert_eq!(
        layers.iter().map(|n| n.clip.as_str()).collect::<Vec<_>>(),
        ["background", "foreground"]
    );
    assert!(layers.iter().all(|n| n.effects.is_empty()));
}

#[test]
fn shifted_fade_phase_survives_trim_and_zero_ramps_have_one_sided_edges() {
    let mut d = doc();
    track(&mut d, "main", "video", TrackKind::Video);
    clip(&mut d, "video", "shot", range(0, 4), solid());
    d.clips
        .get_mut(&cid("shot"))
        .unwrap()
        .effects
        .push(Effect::Fade {
            fade_in: t(4),
            fade_out: t(0),
            envelope: range(-2, 2),
        });
    let c = compile(d.clone());
    assert_eq!(
        visual(&c, 0)[0].effects,
        vec![VisualEffect::Opacity {
            factor: R::new(1.into(), 2.into(), 256).unwrap()
        }]
    );
    assert_eq!(
        visual(&c, 2)[0].effects,
        vec![VisualEffect::Opacity {
            factor: R::integer(0)
        }]
    );
    let left = c
        .visual_at(
            &sample(2, SampleSide::Before),
            QueryLimits::default(),
            &|| false,
        )
        .unwrap()
        .value;
    assert_eq!(
        left[0].effects,
        vec![VisualEffect::Opacity {
            factor: R::integer(1)
        }]
    );
    d.clips.get_mut(&cid("shot")).unwrap().effects = vec![Effect::Fade {
        fade_in: t(0),
        fade_out: t(0),
        envelope: range(1, 2),
    }];
    let c = compile(d);
    assert_eq!(
        visual(&c, 0)[0].effects,
        vec![VisualEffect::Opacity {
            factor: R::integer(0)
        }]
    );
    assert_eq!(
        visual(&c, 1)[0].effects,
        vec![VisualEffect::Opacity {
            factor: R::integer(1)
        }]
    );
}

#[test]
fn absolute_video_and_audio_clock_counts_include_only_starts_before_end() {
    let mut d = doc();
    track(&mut d, "main", "video", TrackKind::Video);
    clip(
        &mut d,
        "video",
        "one",
        TimeRange {
            start: t(0),
            end: RationalTime::new(1001, 30_000).unwrap(),
        },
        solid(),
    );
    let c = compile(d);
    assert_eq!(c.frame_count().unwrap(), 1);
    assert_eq!(c.audio_sample_count().unwrap(), 1602);
    assert_eq!(
        c.frame(0, QueryLimits::default(), &|| false)
            .unwrap()
            .value
            .sample
            .time,
        R::integer(0)
    );
    assert!(matches!(
        c.frame(1, QueryLimits::default(), &|| false),
        Err(CompileError::Outside)
    ));
    assert!(matches!(
        c.frame(-1, QueryLimits::default(), &|| false),
        Err(CompileError::Outside)
    ));
    let last = c
        .audio_sample(1601, QueryLimits::default(), &|| false)
        .unwrap()
        .value;
    assert_eq!(
        last.sample.time,
        R::new(1601.into(), 48000.into(), 256).unwrap()
    );
    assert!(matches!(
        c.audio_sample(1602, QueryLimits::default(), &|| false),
        Err(CompileError::Outside)
    ));
    assert_eq!(compile(doc()).frame_count().unwrap(), 0);
}

#[test]
fn track_order_clip_order_gaps_and_muted_duration_are_explicit() {
    let mut d = doc();
    track(&mut d, "main", "graphics", TrackKind::Graphics);
    track(&mut d, "main", "front", TrackKind::Graphics);
    clip(&mut d, "graphics", "a", range(2, 5), solid());
    clip(&mut d, "graphics", "b", range(1, 4), solid());
    clip(&mut d, "front", "c", range(0, 6), solid());
    d.tracks.get_mut(&tid("front")).unwrap().muted = true;
    let c = compile(d);
    assert_eq!(
        c.duration().compare(&R::integer(6)),
        std::cmp::Ordering::Equal
    );
    assert!(visual(&c, 0).is_empty());
    assert!(visual(&c, 5).is_empty());
    assert_eq!(
        visual(&c, 2)
            .iter()
            .map(|n| n.clip.as_str())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
}

#[test]
fn solo_is_independent_for_visual_and_audio_and_muted_solo_does_not_mute_others() {
    let mut d = doc();
    track(&mut d, "main", "visual", TrackKind::Video);
    track(&mut d, "main", "music", TrackKind::Audio);
    track(&mut d, "main", "speech", TrackKind::Audio);
    clip(&mut d, "visual", "shot", range(0, 4), solid());
    let source = media(&mut d, true, map(0, 4, 4));
    clip(&mut d, "music", "music", range(0, 4), source.clone());
    clip(&mut d, "speech", "speech", range(0, 4), source);
    d.tracks.get_mut(&tid("speech")).unwrap().solo = true;
    let c = compile(d.clone());
    assert_eq!(visual(&c, 1).len(), 1);
    assert_eq!(audio(&c, 1)[0].clip, cid("speech"));
    assert_eq!(audio(&c, 1).len(), 1);
    d.tracks.get_mut(&tid("speech")).unwrap().muted = true;
    assert_eq!(audio(&compile(d), 1)[0].clip, cid("music"));
}

#[test]
fn nested_audio_retains_each_pitch_policy_and_reverse_mute_is_not_cancelled_by_a_child() {
    let mut d = doc();
    sequence(&mut d, "child");
    track(&mut d, "main", "outer", TrackKind::Audio);
    track(&mut d, "child", "inner", TrackKind::Audio);
    let source = media(&mut d, true, map(4, 0, 4));
    clip(&mut d, "inner", "voice", range(0, 4), source);
    d.clips.get_mut(&cid("voice")).unwrap().audio = AudioTimePolicy {
        rate: AudioRatePolicy::Resample,
        reverse: ReverseAudioPolicy::Reverse,
        ..Default::default()
    };
    clip(
        &mut d,
        "outer",
        "group",
        range(0, 4),
        nested("child", map(4, 0, 4)),
    );
    assert!(audio(&compile(d.clone()), 0).is_empty()); // parent policy mutes even though net derivative is positive
    d.clips.get_mut(&cid("group")).unwrap().audio.reverse = ReverseAudioPolicy::Reverse;
    d.clips
        .get_mut(&cid("group"))
        .unwrap()
        .effects
        .push(Effect::Gain {
            milli_decibels: -6000,
        });
    let nodes = audio(&compile(d), 0);
    assert_eq!(nodes[0].policy.rate, AudioRatePolicy::PreservePitch);
    assert_eq!(nodes[0].mapping.sample, sample(4, SampleSide::Before));
    assert_eq!(
        nodes[0].effects,
        vec![AudioEffect::Gain {
            milli_decibels: -6000
        }]
    );
    let AudioSource::Group { voices, .. } = &nodes[0].source else {
        panic!()
    };
    assert_eq!(voices[0].policy.rate, AudioRatePolicy::Resample);
    assert_eq!(voices[0].mapping.sample, sample(0, SampleSide::After));
    assert!(voices[0].effects.is_empty());
}

#[test]
fn nested_bus_projection_does_not_duplicate_audio_and_hold_is_silent() {
    let mut d = doc();
    sequence(&mut d, "child");
    track(&mut d, "main", "outer", TrackKind::Video);
    track(&mut d, "main", "sound", TrackKind::Audio);
    track(&mut d, "child", "inner", TrackKind::Video);
    track(&mut d, "child", "audio", TrackKind::Audio);
    clip(&mut d, "inner", "shot", range(0, 4), solid());
    let source = media(&mut d, true, map(0, 4, 4));
    clip(&mut d, "audio", "voice", range(0, 4), source);
    clip(
        &mut d,
        "outer",
        "group",
        range(0, 4),
        nested("child", map(0, 4, 4)),
    );
    assert!(audio(&compile(d.clone()), 0).is_empty());
    clip(
        &mut d,
        "sound",
        "audio-group",
        range(0, 4),
        nested("child", map(2, 2, 4)),
    );
    assert!(audio(&compile(d.clone()), 0).is_empty());
    d.clips.get_mut(&cid("audio-group")).unwrap().source = nested("child", map(0, 4, 4));
    let c = compile(d);
    assert_eq!(audio(&c, 0).len(), 1);
    assert_eq!(visual(&c, 0).len(), 1);
}

#[test]
fn invalid_document_and_admission_never_produce_a_compiled_snapshot() {
    let mut d = doc();
    track(&mut d, "main", "video", TrackKind::Video);
    clip(
        &mut d,
        "video",
        "one",
        range(0, 4),
        nested("missing", map(0, 4, 4)),
    );
    assert!(matches!(
        CompiledDocument::new(d, CompileLimits::default(), &|| false),
        Err(CompileError::Invalid(_))
    ));
    let limits = CompileLimits {
        validation: ValidationLimits {
            max_nodes: 0,
            ..Default::default()
        },
    };
    assert!(matches!(
        CompiledDocument::new(doc(), limits, &|| false),
        Err(CompileError::Validation(ValidationError::Budget))
    ));
}

#[test]
fn budgets_and_cancellation_at_every_checkpoint_do_not_mutate_the_compiled_document() {
    let mut d = doc();
    sequence(&mut d, "child");
    track(&mut d, "main", "outer", TrackKind::Video);
    track(&mut d, "child", "inner", TrackKind::Video);
    clip(&mut d, "inner", "shot", range(0, 4), solid());
    clip(
        &mut d,
        "outer",
        "group",
        range(0, 4),
        nested("child", map(0, 4, 4)),
    );
    let count = Cell::new(0);
    CompiledDocument::new(d.clone(), CompileLimits::default(), &|| {
        count.set(count.get() + 1);
        false
    })
    .unwrap();
    for stop in 1..=count.get() {
        let at = Cell::new(0);
        assert!(
            CompiledDocument::new(d.clone(), CompileLimits::default(), &|| {
                at.set(at.get() + 1);
                at.get() == stop
            })
            .is_err()
        );
    }
    let c = compile(d);
    let digest = c.digest().clone();
    for limits in [
        QueryLimits {
            max_nodes: 1,
            ..Default::default()
        },
        QueryLimits {
            max_depth: 1,
            ..Default::default()
        },
        QueryLimits {
            max_visits: 1,
            ..Default::default()
        },
        QueryLimits {
            max_output_bytes: 100,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            c.frame(0, limits, &|| false),
            Err(CompileError::Budget)
        ));
    }
    count.set(0);
    c.frame(0, QueryLimits::default(), &|| {
        count.set(count.get() + 1);
        false
    })
    .unwrap();
    for stop in 1..=count.get() {
        let at = Cell::new(0);
        assert!(matches!(
            c.frame(0, QueryLimits::default(), &|| {
                at.set(at.get() + 1);
                at.get() == stop
            }),
            Err(CompileError::Cancelled)
        ));
    }
    assert_eq!(c.digest(), &digest);
    assert_eq!(c.document().content_digest().unwrap(), digest);
}
