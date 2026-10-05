mod support;
use mo_common::{RationalTime, ResourceId};
use mo_video_edit::*;
use mo_video_model::*;
use std::cell::Cell;
use support::*;

fn cut(start: i64, end: i64, ripple: bool, remainders: &[(&str, &str)]) -> RangeEdit {
    RangeEdit {
        tracks: vec![track_id("main-video")],
        range: range(start, end),
        ripple,
        remainders: remainders
            .iter()
            .map(|(clip, right)| SplitTarget {
                clip: clip_id(clip),
                right: clip_id(right),
            })
            .collect(),
        right_groups: vec![],
        linked_policy: LinkedPolicy::RequireAll,
    }
}
fn with_map(id: &str, a: i64, b: i64, duration: i64) -> RetimeTarget {
    let ClipSource::Media { time_map, .. } = source(a, b, duration) else {
        unreachable!()
    };
    RetimeTarget {
        clip: clip_id(id),
        time_map,
        audio: AudioTimePolicy {
            rate: AudioRatePolicy::Resample,
            reverse: ReverseAudioPolicy::Reverse,
            hold: HoldAudioPolicy::Silence,
        },
    }
}
fn retime(target: RetimeTarget, ripple: bool, effects: EffectTimingPolicy) -> Operation {
    Operation::Retime {
        targets: vec![target],
        effects,
        ripple_tracks: if ripple {
            vec![track_id("main-video")]
        } else {
            vec![]
        },
        linked_policy: LinkedPolicy::RequireAll,
    }
}

#[test]
fn overwrite_keeps_both_source_windows_and_stable_outer_identities() {
    let base = base();
    let result = apply(
        &base,
        vec![Operation::Overwrite {
            track: track_id("main-video"),
            clip: shot("replacement", 3, 7),
            remainders: vec![SplitTarget {
                clip: clip_id("a"),
                right: clip_id("a-right"),
            }],
            right_groups: vec![],
            linked_policy: LinkedPolicy::RequireAll,
        }],
    )
    .unwrap();
    let d = result.candidate.document();
    assert_eq!(d.clips[&clip_id("a")].timeline, range(0, 3));
    assert_eq!(d.clips[&clip_id("a-right")].timeline, range(7, 10));
    assert_eq!(
        map(&d.clips[&clip_id("a")]).anchors.last().unwrap().source,
        time(23)
    );
    assert_eq!(
        map(&d.clips[&clip_id("a-right")]).anchors[0].source,
        time(27)
    );
    assert_eq!(d.clips[&clip_id("b")], base.document().clips[&clip_id("b")]);
    assert_eq!(
        undo(&result.candidate, &result.inverse)
            .unwrap()
            .candidate
            .digest(),
        base.digest()
    );
}

#[test]
fn ripple_delete_handles_partial_edges_full_deletion_and_later_clips_once() {
    let base = base();
    let result = apply(
        &base,
        vec![Operation::DeleteRange {
            edit: cut(8, 22, true, &[]),
        }],
    )
    .unwrap();
    let d = result.candidate.document();
    assert!(!d.clips.contains_key(&clip_id("b")));
    assert_eq!(d.clips[&clip_id("a")].timeline, range(0, 8));
    assert_eq!(d.clips[&clip_id("c")].timeline, range(8, 16));
    assert_eq!(map(&d.clips[&clip_id("c")]).anchors[0].source, time(42));
    assert_eq!(d.clips[&clip_id("x")].timeline, range(0, 10));
    assert_eq!(
        undo(&result.candidate, &result.inverse)
            .unwrap()
            .candidate
            .digest(),
        base.digest()
    );
}

#[test]
fn ripple_delete_splits_straddler_and_does_not_restart_its_fade() {
    let mut doc = document();
    doc.clips.get_mut(&clip_id("a")).unwrap().effects = vec![Effect::Fade {
        fade_in: time(3),
        fade_out: time(3),
        envelope: range(0, 10),
    }];
    let base = snapshot(doc);
    let result = apply(
        &base,
        vec![Operation::DeleteRange {
            edit: cut(3, 7, true, &[("a", "a-right")]),
        }],
    )
    .unwrap();
    let d = result.candidate.document();
    assert_eq!(d.clips[&clip_id("a-right")].timeline, range(3, 6));
    assert_eq!(d.clips[&clip_id("b")].timeline, range(6, 16));
    assert_eq!(
        d.clips[&clip_id("a-right")].effects,
        vec![Effect::Fade {
            fade_in: time(3),
            fade_out: time(3),
            envelope: range(-7, 3)
        }]
    );
    assert_eq!(
        undo(&result.candidate, &result.inverse)
            .unwrap()
            .candidate
            .digest(),
        base.digest()
    );
}

#[test]
fn range_identity_contract_rejects_missing_extra_duplicate_and_reused_ids() {
    let base = base();
    for remainder in [
        vec![],
        vec![("a", "right"), ("b", "unused")],
        vec![("a", "right"), ("a", "other")],
        vec![("a", "b")],
    ] {
        assert!(
            apply(
                &base,
                vec![Operation::DeleteRange {
                    edit: cut(3, 7, false, &remainder)
                }]
            )
            .is_err()
        );
    }
    assert!(
        apply(
            &base,
            vec![Operation::Overwrite {
                track: track_id("main-video"),
                clip: shot("same", 3, 7),
                remainders: vec![SplitTarget {
                    clip: clip_id("a"),
                    right: clip_id("same")
                }],
                right_groups: vec![],
                linked_policy: LinkedPolicy::Independent
            }]
        )
        .is_err()
    );
    assert_eq!(base.document().clips.len(), 4);
}

#[test]
fn linked_range_split_requires_both_tracks_and_exact_new_side_groups() {
    let mut doc = document();
    doc.link_groups.insert(
        group_id("sync"),
        LinkGroup {
            id: group_id("sync"),
            members: vec![clip_id("a"), clip_id("x")],
        },
    );
    let base = snapshot(doc);
    assert!(
        apply(
            &base,
            vec![Operation::DeleteRange {
                edit: cut(3, 7, true, &[("a", "a2")])
            }]
        )
        .is_err()
    );
    let mut edit = cut(3, 7, true, &[("a", "a2"), ("x", "x2")]);
    edit.tracks.push(track_id("overlay"));
    assert!(apply(&base, vec![Operation::DeleteRange { edit: edit.clone() }]).is_err());
    edit.right_groups = vec![LinkGroup {
        id: group_id("sync-right"),
        members: vec![clip_id("a2"), clip_id("x2")],
    }];
    let result = apply(&base, vec![Operation::DeleteRange { edit }]).unwrap();
    let d = result.candidate.document();
    assert_eq!(
        d.link_groups[&group_id("sync")].members,
        vec![clip_id("a"), clip_id("x")]
    );
    assert_eq!(d.link_groups.len(), 2);
    assert_eq!(
        d.clips[&clip_id("a2")].timeline,
        d.clips[&clip_id("x2")].timeline
    );
    assert_eq!(
        undo(&result.candidate, &result.inverse)
            .unwrap()
            .candidate
            .digest(),
        base.digest()
    );
}

#[test]
fn asymmetric_linked_cut_reuses_group_for_the_paired_surviving_side() {
    let mut doc = document();
    let x = doc.clips.get_mut(&clip_id("x")).unwrap();
    x.timeline = range(5, 10);
    x.source = source(25, 30, 5);
    doc.link_groups.insert(
        group_id("sync"),
        LinkGroup {
            id: group_id("sync"),
            members: vec![clip_id("a"), clip_id("x")],
        },
    );
    let base = snapshot(doc);
    let mut edit = cut(3, 7, false, &[("a", "a2")]);
    edit.tracks.push(track_id("overlay"));
    let result = apply(&base, vec![Operation::DeleteRange { edit }]).unwrap();
    let d = result.candidate.document();
    assert_eq!(
        d.link_groups[&group_id("sync")].members,
        vec![clip_id("a2"), clip_id("x")]
    );
    assert_eq!(d.clips[&clip_id("a")].timeline, range(0, 3));
    assert_eq!(d.clips[&clip_id("x")].timeline, range(7, 10));
    assert_eq!(
        undo(&result.candidate, &result.inverse)
            .unwrap()
            .candidate
            .digest(),
        base.digest()
    );
}

#[test]
fn independent_overwrite_retains_authorized_independent_audio_or_other_tracks() {
    let mut doc = document();
    doc.link_groups.insert(
        group_id("sync"),
        LinkGroup {
            id: group_id("sync"),
            members: vec![clip_id("a"), clip_id("x")],
        },
    );
    let base = snapshot(doc);
    let result = apply(
        &base,
        vec![Operation::Overwrite {
            track: track_id("main-video"),
            clip: shot("replacement", 0, 10),
            remainders: vec![],
            right_groups: vec![],
            linked_policy: LinkedPolicy::Independent,
        }],
    )
    .unwrap();
    assert_eq!(
        result.candidate.document().clips[&clip_id("x")],
        base.document().clips[&clip_id("x")]
    );
    assert!(result.candidate.document().link_groups.is_empty());
    let later = apply(&result.candidate, vec![move_clip("x", 1)]).unwrap();
    assert!(matches!(
        undo(&later.candidate, &result.inverse),
        Err(EditError::Conflict(_))
    ));
}

#[test]
fn cuts_use_half_open_boundaries_and_preserve_gaps_without_ripple() {
    let base = base();
    let result = apply(
        &base,
        vec![Operation::DeleteRange {
            edit: cut(10, 20, false, &[]),
        }],
    )
    .unwrap();
    let d = result.candidate.document();
    assert_eq!(d.clips[&clip_id("a")], base.document().clips[&clip_id("a")]);
    assert_eq!(d.clips[&clip_id("c")], base.document().clips[&clip_id("c")]);
    assert!(!d.clips.contains_key(&clip_id("b")));
    let all = apply(
        &base,
        vec![Operation::DeleteRange {
            edit: cut(0, 30, true, &[]),
        }],
    )
    .unwrap();
    assert!(
        all.candidate.document().tracks[&track_id("main-video")]
            .clips
            .is_empty()
    );
    assert!(
        apply(
            &base,
            vec![Operation::DeleteRange {
                edit: cut(-1, 1, false, &[])
            }]
        )
        .is_err()
    );
    assert!(
        apply(
            &base,
            vec![Operation::DeleteRange {
                edit: cut(10, 10, false, &[])
            }]
        )
        .is_err()
    );
}

#[test]
fn retime_preserves_explicit_audio_policy_and_scales_effect_time_exactly() {
    let mut doc = document();
    doc.clips.get_mut(&clip_id("a")).unwrap().effects = vec![Effect::Fade {
        fade_in: time(4),
        fade_out: time(2),
        envelope: range(0, 10),
    }];
    let base = snapshot(doc);
    let target = with_map("a", 20, 30, 5);
    let result = apply(
        &base,
        vec![retime(
            target.clone(),
            true,
            EffectTimingPolicy::ScaleToDuration,
        )],
    )
    .unwrap();
    let d = result.candidate.document();
    assert_eq!(d.clips[&clip_id("a")].timeline, range(0, 5));
    assert_eq!(d.clips[&clip_id("a")].audio, target.audio);
    assert_eq!(
        d.clips[&clip_id("a")].effects,
        vec![Effect::Fade {
            fade_in: time(2),
            fade_out: time(1),
            envelope: range(0, 5)
        }]
    );
    assert_eq!(d.clips[&clip_id("b")].timeline, range(5, 15));
    assert_eq!(
        undo(&result.candidate, &result.inverse)
            .unwrap()
            .candidate
            .digest(),
        base.digest()
    );
}

#[test]
fn retime_reverse_hold_and_effect_preservation_have_one_saved_authority() {
    let mut doc = document();
    doc.clips.get_mut(&clip_id("x")).unwrap().effects = vec![Effect::Fade {
        fade_in: time(1),
        fade_out: time(2),
        envelope: range(0, 10),
    }];
    let base = snapshot(doc);
    let mut target = with_map("x", 40, 20, 5);
    target.time_map.anchors.insert(
        1,
        TimeAnchor {
            local: time(1),
            source: time(40),
        },
    );
    let result = apply(
        &base,
        vec![retime(target, false, EffectTimingPolicy::PreserveClipTime)],
    )
    .unwrap();
    let clip = &result.candidate.document().clips[&clip_id("x")];
    assert_eq!(clip.timeline, range(0, 5));
    assert_eq!(map(clip).anchors[0].source, map(clip).anchors[1].source);
    assert_eq!(clip.effects, base.document().clips[&clip_id("x")].effects);
    assert_eq!(
        undo(&result.candidate, &result.inverse)
            .unwrap()
            .candidate
            .digest(),
        base.digest()
    );
}

#[test]
fn malformed_retime_out_of_bounds_and_overlap_fail_atomically() {
    let base = base();
    for target in [
        with_map("a", 20, 30, 11),
        with_map("a", 20, 130, 5),
        with_map("a", 20, 30, 0),
    ] {
        assert!(
            apply(
                &base,
                vec![retime(target, false, EffectTimingPolicy::PreserveClipTime)]
            )
            .is_err()
        );
    }
    let mut target = with_map("x", 20, 30, 5);
    target.time_map.anchors[0].local = time(1);
    assert!(
        apply(
            &base,
            vec![retime(target, false, EffectTimingPolicy::PreserveClipTime)]
        )
        .is_err()
    );
    let mut target = with_map("x", 20, 30, 5);
    target.time_map.anchors[0].source = time(100);
    target.time_map.anchors[1].source = time(100);
    assert!(
        apply(
            &base,
            vec![retime(target, false, EffectTimingPolicy::PreserveClipTime)]
        )
        .is_err()
    );
    assert_eq!(base.document().clips[&clip_id("a")].timeline, range(0, 10));
}

#[test]
fn joint_retime_rejects_inconsistent_ripple_duration() {
    let base = base();
    let operation = Operation::Retime {
        targets: vec![with_map("a", 20, 30, 5), with_map("x", 20, 30, 7)],
        effects: EffectTimingPolicy::PreserveClipTime,
        ripple_tracks: vec![track_id("main-video"), track_id("overlay")],
        linked_policy: LinkedPolicy::RequireAll,
    };
    assert!(apply(&base, vec![operation]).is_err());
}

fn second_sequence() -> VideoDocument {
    let mut doc = document();
    let sequence = SequenceId::new("other").unwrap();
    doc.sequences
        .get_mut(&doc.root_sequence.clone())
        .unwrap()
        .tracks
        .retain(|id| id != &track_id("overlay"));
    doc.sequences.insert(
        sequence.clone(),
        Sequence {
            id: sequence,
            tracks: vec![track_id("overlay")],
        },
    );
    doc
}

#[test]
fn range_and_ripple_operations_cannot_mix_nested_sequence_time_coordinates() {
    let base = snapshot(second_sequence());
    let mut edit = cut(3, 7, true, &[("a", "a2"), ("x", "x2")]);
    edit.tracks.push(track_id("overlay"));
    assert!(apply(&base, vec![Operation::DeleteRange { edit }]).is_err());
    assert!(
        apply(
            &base,
            vec![Operation::Trim {
                clips: vec![clip_id("a")],
                edge: Edge::End,
                delta: time(-1),
                ripple_tracks: vec![track_id("overlay")],
                linked_policy: LinkedPolicy::RequireAll
            }]
        )
        .is_err()
    );
    assert!(
        apply(
            &base,
            vec![Operation::Link {
                group: LinkGroup {
                    id: group_id("cross"),
                    members: vec![clip_id("a"), clip_id("x")]
                }
            }]
        )
        .is_err()
    );
}

#[test]
fn exact_fractional_cut_preserves_source_time() {
    let base = base();
    let mut edit = cut(3, 7, true, &[("a", "a2")]);
    edit.range = TimeRange {
        start: RationalTime::new(1001, 30000).unwrap(),
        end: RationalTime::new(2002, 30000).unwrap(),
    };
    let result = apply(&base, vec![Operation::DeleteRange { edit }]).unwrap();
    let d = result.candidate.document();
    assert_eq!(
        d.clips[&clip_id("a2")].timeline.start,
        RationalTime::new(1001, 30000).unwrap()
    );
    assert_eq!(
        map(&d.clips[&clip_id("a2")]).anchors[0].source,
        RationalTime::new(301001, 15000).unwrap()
    );
    assert_eq!(
        undo(&result.candidate, &result.inverse)
            .unwrap()
            .candidate
            .digest(),
        base.digest()
    );
}

#[test]
fn effect_scaling_never_silently_rounds_unrepresentable_time() {
    let mut doc = document();
    doc.clips.get_mut(&clip_id("x")).unwrap().effects = vec![Effect::Fade {
        fade_in: RationalTime::new(1, 999_999_937).unwrap(),
        fade_out: time(0),
        envelope: range(0, 10),
    }];
    let base = snapshot(doc);
    let mut target = with_map("x", 20, 30, 1);
    target.time_map.anchors[1].local = RationalTime::new(1, 999_999_929).unwrap();
    let error = apply(
        &base,
        vec![retime(target, false, EffectTimingPolicy::ScaleToDuration)],
    )
    .unwrap_err();
    assert!(error.to_string().contains("cannot be represented"));
    assert_eq!(base.document().clips[&clip_id("x")].timeline, range(0, 10));
}

#[test]
fn range_cancel_at_each_checkpoint_and_locked_scope_do_not_publish_partial_state() {
    let base = base();
    let req = request(
        &base,
        vec![Operation::DeleteRange {
            edit: cut(3, 7, true, &[("a", "a2")]),
        }],
    );
    let count = Cell::new(0);
    prepare(&base, &req, EditLimits::default(), &|| {
        count.set(count.get() + 1);
        false
    })
    .unwrap();
    for stop in 1..=count.get() {
        let polls = Cell::new(0);
        assert!(
            prepare(&base, &req, EditLimits::default(), &|| {
                polls.set(polls.get() + 1);
                polls.get() >= stop
            })
            .is_err()
        );
    }
    let mut doc = document();
    doc.tracks.get_mut(&track_id("main-video")).unwrap().locked = true;
    assert!(apply(&snapshot(doc), req.operations).is_err());
    assert!(!base.document().clips.contains_key(&clip_id("a2")));
}

#[test]
fn audio_retime_contract_rejects_unknown_behavior() {
    let base = base();
    let req = request(
        &base,
        vec![retime(
            with_map("x", 20, 30, 5),
            false,
            EffectTimingPolicy::PreserveClipTime,
        )],
    );
    let mut wire = serde_json::to_value(req).unwrap();
    wire["operations"][0]["targets"][0]["audio"]["hold"] = "repeatSample".into();
    assert!(serde_json::from_value::<Transaction>(wire).is_err());
    assert!(
        base.document()
            .resources
            .contains_key(&ResourceId::new("movie").unwrap())
    );
}

#[test]
fn actual_audio_track_keeps_gain_policy_and_sync_through_cut_and_retime() {
    let mut doc = document();
    doc.tracks.get_mut(&track_id("overlay")).unwrap().kind = TrackKind::Audio;
    let ResourceDescriptor::Media { streams } = &mut doc
        .resources
        .get_mut(&ResourceId::new("movie").unwrap())
        .unwrap()
        .descriptor
    else {
        unreachable!()
    };
    streams.insert(
        StreamId::new("audio").unwrap(),
        MediaStream::Audio {
            available: range(-1, 100),
            sample_rate: 48000,
            channels: 2,
        },
    );
    let x = doc.clips.get_mut(&clip_id("x")).unwrap();
    let ClipSource::Media { stream, .. } = &mut x.source else {
        unreachable!()
    };
    *stream = StreamId::new("audio").unwrap();
    x.effects = vec![Effect::Gain {
        milli_decibels: -3000,
    }];
    doc.link_groups.insert(
        group_id("sync"),
        LinkGroup {
            id: group_id("sync"),
            members: vec![clip_id("a"), clip_id("x")],
        },
    );
    let base = snapshot(doc);
    let mut edit = cut(3, 7, true, &[("a", "a2"), ("x", "x2")]);
    edit.tracks.push(track_id("overlay"));
    edit.right_groups = vec![LinkGroup {
        id: group_id("sync-right"),
        members: vec![clip_id("a2"), clip_id("x2")],
    }];
    let cut = apply(&base, vec![Operation::DeleteRange { edit }]).unwrap();
    let d = cut.candidate.document();
    assert_eq!(
        d.clips[&clip_id("a2")].timeline,
        d.clips[&clip_id("x2")].timeline
    );
    assert_eq!(
        d.clips[&clip_id("x2")].effects,
        vec![Effect::Gain {
            milli_decibels: -3000
        }]
    );
    let mut audio = with_map("x", 20, 30, 5);
    audio.audio.rate = AudioRatePolicy::PreservePitch;
    let changed = apply(
        &base,
        vec![Operation::Retime {
            targets: vec![with_map("a", 20, 30, 5), audio],
            effects: EffectTimingPolicy::ScaleToDuration,
            ripple_tracks: vec![track_id("main-video"), track_id("overlay")],
            linked_policy: LinkedPolicy::RequireAll,
        }],
    )
    .unwrap();
    let d = changed.candidate.document();
    assert_eq!(
        d.clips[&clip_id("a")].timeline,
        d.clips[&clip_id("x")].timeline
    );
    assert_eq!(
        d.clips[&clip_id("x")].audio.rate,
        AudioRatePolicy::PreservePitch
    );
    assert_eq!(
        undo(&changed.candidate, &changed.inverse)
            .unwrap()
            .candidate
            .digest(),
        base.digest()
    );
}
