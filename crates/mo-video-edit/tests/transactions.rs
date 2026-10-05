mod support;
use mo_common::ResourceId;
use mo_video_edit::*;
use mo_video_model::*;
use std::cell::Cell;
use support::*;

#[test]
fn admission_preserves_the_ability_to_apply_the_inverse() {
    let mut doc = document();
    doc.title = "x".repeat(2000);
    let base = snapshot(doc);
    let req = request(
        &base,
        vec![Operation::SetTitle {
            title: "short".into(),
        }],
    );
    assert!(matches!(
        prepare(
            &base,
            &req,
            EditLimits {
                max_request_bytes: 1000,
                ..EditLimits::default()
            },
            &|| false
        ),
        Err(EditError::Budget)
    ));
    assert_eq!(base.document().title.len(), 2000);
}

#[test]
fn solo_change_invalidates_other_tracks_in_the_same_sequence() {
    let base = base();
    let result = apply(
        &base,
        vec![Operation::SetTrack {
            track: track_id("overlay"),
            name: "overlay".into(),
            locked: false,
            muted: false,
            solo: true,
        }],
    )
    .unwrap();
    let main = result
        .invalidations
        .iter()
        .find(|i| i.track == track_id("main-video"))
        .unwrap();
    assert_eq!(main.ranges, vec![range(0, 30)]);
}

#[test]
fn frame_boundary_split_keeps_fractional_time_without_rounding() {
    let base = base();
    let at = mo_common::RationalTime::new(1001, 30000).unwrap();
    let result = apply(
        &base,
        vec![Operation::Split {
            targets: vec![SplitTarget {
                clip: clip_id("a"),
                right: clip_id("a2"),
            }],
            at,
            right_groups: vec![],
            linked_policy: LinkedPolicy::RequireAll,
        }],
    )
    .unwrap();
    let d = result.candidate.document();
    assert_eq!(d.clips[&clip_id("a")].timeline.end, at);
    assert_eq!(
        map(&d.clips[&clip_id("a2")]).anchors[0].source,
        mo_common::RationalTime::new(601001, 30000).unwrap()
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
fn failure_after_successful_first_operation_never_changes_base() {
    let base = base();
    let before = serde_json::to_string(base.document()).unwrap();
    let result = apply(
        &base,
        vec![
            Operation::SetTitle {
                title: "new".into(),
            },
            split("a", "a2", 10),
        ],
    );
    assert!(matches!(result, Err(EditError::Operation { index: 1, .. })));
    assert_eq!(before, serde_json::to_string(base.document()).unwrap());
    assert!(base.document().title.is_empty());
}

#[test]
fn overlap_failure_and_stale_snapshot_are_explicit() {
    let base = base();
    assert!(matches!(
        apply(&base, vec![move_clip("a", 1)]),
        Err(EditError::InvalidDocument(_))
    ));
    let edited = apply(
        &base,
        vec![Operation::SetTitle {
            title: "current".into(),
        }],
    )
    .unwrap();
    assert!(matches!(
        prepare(
            &edited.candidate,
            &request(&base, vec![split("a", "right", 5)]),
            EditLimits::default(),
            &|| false
        ),
        Err(EditError::StaleSnapshot)
    ));
}

#[test]
fn undo_and_redo_restore_exact_content_without_rewinding_an_unrelated_edit() {
    let base = base();
    let first = apply(&base, vec![split("a", "a2", 5)]).unwrap();
    let later = apply(
        &first.candidate,
        vec![
            move_clip("x", 2),
            Operation::SetTitle {
                title: "keep me".into(),
            },
        ],
    )
    .unwrap();
    let undone = undo(&later.candidate, &first.inverse).unwrap();
    assert!(
        !undone
            .candidate
            .document()
            .clips
            .contains_key(&clip_id("a2"))
    );
    assert_eq!(
        undone.candidate.document().clips[&clip_id("a")],
        base.document().clips[&clip_id("a")]
    );
    assert_eq!(
        undone.candidate.document().clips[&clip_id("x")].timeline,
        range(2, 12)
    );
    assert_eq!(undone.candidate.document().title, "keep me");
    let redone = undo(&undone.candidate, &undone.inverse).unwrap();
    assert_eq!(redone.candidate.digest(), later.candidate.digest());
}

#[test]
fn undo_conflicts_with_later_clip_edits_or_new_link_dependencies() {
    let base = base();
    let first = apply(&base, vec![move_clip("x", 2)]).unwrap();
    let changed = apply(&first.candidate, vec![move_clip("x", 1)]).unwrap();
    assert!(matches!(
        undo(&changed.candidate, &first.inverse),
        Err(EditError::Conflict(_))
    ));
    let linked = apply(
        &first.candidate,
        vec![Operation::Link {
            group: LinkGroup {
                id: group_id("sync"),
                members: vec![clip_id("a"), clip_id("x")],
            },
        }],
    )
    .unwrap();
    assert!(matches!(
        undo(&linked.candidate, &first.inverse),
        Err(EditError::Conflict(_))
    ));
}

#[test]
fn locked_tracks_and_resource_dependencies_cannot_be_bypassed() {
    let base = base();
    let locked = apply(
        &base,
        vec![Operation::SetTrack {
            track: track_id("main-video"),
            name: "Main".into(),
            locked: true,
            muted: false,
            solo: false,
        }],
    )
    .unwrap();
    assert!(matches!(
        apply(&locked.candidate, vec![split("a", "a2", 5)]),
        Err(EditError::Operation { .. })
    ));
    assert!(matches!(
        apply(
            &base,
            vec![Operation::RemoveResource {
                resource: ResourceId::new("movie").unwrap()
            }]
        ),
        Err(EditError::InvalidDocument(_))
    ));
}

#[test]
fn cancellation_at_every_observed_checkpoint_keeps_base_immutable() {
    let base = base();
    let req = request(&base, vec![split("a", "right", 5), move_clip("x", 2)]);
    let count = Cell::new(0);
    let prepared = prepare(&base, &req, EditLimits::default(), &|| {
        count.set(count.get() + 1);
        false
    })
    .unwrap();
    assert_ne!(prepared.candidate.digest(), base.digest());
    for stop in 1..=count.get() {
        let polls = Cell::new(0);
        let result = prepare(&base, &req, EditLimits::default(), &|| {
            polls.set(polls.get() + 1);
            polls.get() >= stop
        });
        assert!(result.is_err(), "checkpoint {stop}");
        assert!(!base.document().clips.contains_key(&clip_id("right")));
    }
}

#[test]
fn budgets_and_inverse_preconditions_fail_closed() {
    let base = base();
    let req = request(&base, vec![split("a", "right", 5)]);
    assert!(matches!(
        prepare(
            &base,
            &req,
            EditLimits {
                max_request_bytes: 10,
                ..EditLimits::default()
            },
            &|| false
        ),
        Err(EditError::Budget)
    ));
    assert!(matches!(
        prepare(
            &base,
            &req,
            EditLimits {
                max_changed_objects: 1,
                ..EditLimits::default()
            },
            &|| false
        ),
        Err(EditError::Budget)
    ));
    let mut inverse = apply(&base, vec![split("a", "right", 5)]).unwrap().inverse;
    inverse.conditions.clear();
    assert!(matches!(
        undo(&base, &inverse),
        Err(EditError::InvalidOperation(_))
    ));
}

#[test]
fn split_preserves_piecewise_time_map_reverse_hold_and_fade_phase() {
    let mut doc = document();
    let clip = doc.clips.get_mut(&clip_id("a")).unwrap();
    clip.source = ClipSource::Media {
        resource: ResourceId::new("movie").unwrap(),
        stream: StreamId::new("video").unwrap(),
        time_map: TimeMap {
            anchors: vec![
                TimeAnchor {
                    local: time(0),
                    source: time(40),
                },
                TimeAnchor {
                    local: time(2),
                    source: time(30),
                },
                TimeAnchor {
                    local: time(4),
                    source: time(30),
                },
                TimeAnchor {
                    local: time(10),
                    source: time(20),
                },
            ],
        },
    };
    clip.effects = vec![Effect::Fade {
        fade_in: time(4),
        fade_out: time(3),
        envelope: range(0, 10),
    }];
    let base = snapshot(doc);
    let result = apply(&base, vec![split("a", "a2", 3)]).unwrap();
    let left = &result.candidate.document().clips[&clip_id("a")];
    let right = &result.candidate.document().clips[&clip_id("a2")];
    assert_eq!(map(left).anchors.last().unwrap().source, time(30));
    assert_eq!(map(right).anchors[0].source, time(30));
    assert_eq!(map(right).anchors[1].local, time(1));
    assert_eq!(
        right.effects,
        vec![Effect::Fade {
            fade_in: time(4),
            fade_out: time(3),
            envelope: range(-3, 7)
        }]
    );
    let restored = undo(&result.candidate, &result.inverse).unwrap();
    assert_eq!(restored.candidate.digest(), base.digest());
}

#[test]
fn linked_split_requires_explicit_right_group_and_preserves_both_sides() {
    let mut doc = document();
    doc.link_groups.insert(
        group_id("sync"),
        LinkGroup {
            id: group_id("sync"),
            members: vec![clip_id("a"), clip_id("x")],
        },
    );
    let base = snapshot(doc);
    assert!(apply(&base, vec![split("a", "a2", 5)]).is_err());
    let result = apply(
        &base,
        vec![Operation::Split {
            targets: vec![
                SplitTarget {
                    clip: clip_id("a"),
                    right: clip_id("a2"),
                },
                SplitTarget {
                    clip: clip_id("x"),
                    right: clip_id("x2"),
                },
            ],
            at: time(5),
            linked_policy: LinkedPolicy::RequireAll,
            right_groups: vec![LinkGroup {
                id: group_id("right-sync"),
                members: vec![clip_id("a2"), clip_id("x2")],
            }],
        }],
    )
    .unwrap();
    assert_eq!(result.candidate.document().link_groups.len(), 2);
    assert_eq!(
        undo(&result.candidate, &result.inverse)
            .unwrap()
            .candidate
            .digest(),
        base.digest()
    );
}

#[test]
fn trim_ripple_moves_successors_once_and_checks_source_handles() {
    let base = base();
    let result = apply(
        &base,
        vec![Operation::Trim {
            clips: vec![clip_id("a")],
            edge: Edge::Start,
            delta: time(2),
            ripple_tracks: vec![track_id("main-video")],
            linked_policy: LinkedPolicy::RequireAll,
        }],
    )
    .unwrap();
    let d = result.candidate.document();
    assert_eq!(d.clips[&clip_id("a")].timeline, range(0, 8));
    assert_eq!(map(&d.clips[&clip_id("a")]).anchors[0].source, time(22));
    assert_eq!(d.clips[&clip_id("b")].timeline, range(8, 18));
    assert_eq!(d.clips[&clip_id("c")].timeline, range(18, 28));
    assert_eq!(d.clips[&clip_id("x")].timeline, range(0, 10));
    assert!(
        apply(
            &base,
            vec![Operation::Slip {
                clips: vec![clip_id("a")],
                delta: time(100),
                linked_policy: LinkedPolicy::RequireAll
            }]
        )
        .is_err()
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
fn roll_and_slide_preserve_total_length_and_source_windows() {
    let base = base();
    let roll = apply(
        &base,
        vec![Operation::Roll {
            left: clip_id("a"),
            right: clip_id("b"),
            delta: time(2),
            linked_policy: LinkedPolicy::RequireAll,
        }],
    )
    .unwrap();
    assert_eq!(
        roll.candidate.document().clips[&clip_id("a")].timeline,
        range(0, 12)
    );
    assert_eq!(
        roll.candidate.document().clips[&clip_id("b")].timeline,
        range(12, 20)
    );
    let slide = apply(
        &base,
        vec![Operation::Slide {
            left: clip_id("a"),
            clip: clip_id("b"),
            right: clip_id("c"),
            delta: time(2),
            linked_policy: LinkedPolicy::RequireAll,
        }],
    )
    .unwrap();
    let d = slide.candidate.document();
    assert_eq!(d.clips[&clip_id("a")].timeline, range(0, 12));
    assert_eq!(d.clips[&clip_id("b")].timeline, range(12, 22));
    assert_eq!(d.clips[&clip_id("c")].timeline, range(22, 30));
    assert_eq!(
        map(&d.clips[&clip_id("b")]),
        map(&base.document().clips[&clip_id("b")])
    );
    assert_eq!(
        undo(&slide.candidate, &slide.inverse)
            .unwrap()
            .candidate
            .digest(),
        base.digest()
    );
}

#[test]
fn ripple_refuses_straddlers_locked_tracks_and_partial_link_sets() {
    let base = base();
    let insertion = Operation::Insert {
        track: track_id("main-video"),
        clip: shot("new", 5, 6),
        ripple_tracks: vec![track_id("main-video")],
    };
    assert!(apply(&base, vec![insertion]).is_err());
    let mut doc = document();
    doc.link_groups.insert(
        group_id("later"),
        LinkGroup {
            id: group_id("later"),
            members: vec![clip_id("b"), clip_id("x")],
        },
    );
    let result = apply(
        &snapshot(doc),
        vec![Operation::Trim {
            clips: vec![clip_id("a")],
            edge: Edge::End,
            delta: time(-1),
            ripple_tracks: vec![track_id("main-video")],
            linked_policy: LinkedPolicy::RequireAll,
        }],
    );
    assert!(result.is_err());
}

#[test]
fn delete_prunes_link_members_and_can_be_undone() {
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
        vec![Operation::Delete {
            clips: vec![clip_id("a")],
            linked_policy: LinkedPolicy::Independent,
        }],
    )
    .unwrap();
    assert!(result.candidate.document().link_groups.is_empty());
    assert_eq!(result.candidate.document().clips.len(), 3);
    assert_eq!(
        undo(&result.candidate, &result.inverse)
            .unwrap()
            .candidate
            .digest(),
        base.digest()
    );
}

#[test]
fn restoring_a_dissolved_link_conflicts_if_a_surviving_member_changed() {
    let mut doc = document();
    doc.link_groups.insert(
        group_id("sync"),
        LinkGroup {
            id: group_id("sync"),
            members: vec![clip_id("a"), clip_id("x")],
        },
    );
    let base = snapshot(doc);
    for operation in [
        Operation::Delete {
            clips: vec![clip_id("a")],
            linked_policy: LinkedPolicy::Independent,
        },
        Operation::Unlink {
            group: group_id("sync"),
        },
    ] {
        let first = apply(&base, vec![operation]).unwrap();
        let later = apply(&first.candidate, vec![move_clip("x", 1)]).unwrap();
        assert!(matches!(
            undo(&later.candidate, &first.inverse),
            Err(EditError::Conflict(_))
        ));
    }
}

#[test]
fn operation_schema_is_strict_and_roundtrips_without_float_time() {
    let base = base();
    let req = request(&base, vec![split("a", "a2", 5)]);
    let mut json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["operations"][0]["at"]["ticks"], "5");
    assert_eq!(
        serde_json::from_value::<Transaction>(json.clone()).unwrap(),
        req
    );
    json["operations"][0]["unknown"] = true.into();
    assert!(serde_json::from_value::<Transaction>(json).is_err());
}
