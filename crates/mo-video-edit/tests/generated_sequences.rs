mod support;
use mo_video_edit::*;
use mo_video_model::{AudioTimePolicy, ClipSource, validate};
use support::*;

/// Long-form acceptance corpus; run explicitly in release mode. Fixed seed and failing index
/// reproduce the entire prefix, while individual semantics have small boundary regression tests.
#[test]
#[ignore = "100,000 transaction corpus; run with --release --ignored --nocapture"]
fn one_hundred_thousand_seeded_transactions_preserve_atomicity_and_inverse() {
    let mut seed = 0x57494e444f575f31_u64;
    let mut current = base();
    let mut accepted = 0;
    let mut refused = 0;
    let mut inverses = 0;
    for index in 0..100_000 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        if index % 257 == 0 {
            current = base();
        }
        let delta = (seed % 7) as i64 - 3;
        let id = ["a", "b", "c", "x"][(seed as usize >> 8) % 4];
        let start = ((seed >> 24) % 25) as i64;
        let end = start + 1 + ((seed >> 32) % 4) as i64;
        let remainders = || {
            current.document().tracks[&track_id("main-video")]
                .clips
                .iter()
                .filter_map(|id| {
                    let c = &current.document().clips[id];
                    (c.timeline.start.compare_time(time(start)).is_lt()
                        && c.timeline.end.compare_time(time(end)).is_gt())
                    .then(|| SplitTarget {
                        clip: id.clone(),
                        right: clip_id(&format!("fragment-{index}-{id}")),
                    })
                })
                .collect::<Vec<_>>()
        };
        let operation = match (seed >> 16) % 10 {
            0 => move_clip(id, delta),
            1 => Operation::Slip {
                clips: vec![clip_id(id)],
                delta: time(delta),
                linked_policy: LinkedPolicy::RequireAll,
            },
            2 => Operation::Trim {
                clips: vec![clip_id(id)],
                edge: if seed & 1 == 0 {
                    Edge::Start
                } else {
                    Edge::End
                },
                delta: time(delta),
                ripple_tracks: vec![],
                linked_policy: LinkedPolicy::RequireAll,
            },
            3 => Operation::Roll {
                left: clip_id("a"),
                right: clip_id("b"),
                delta: time(delta),
                linked_policy: LinkedPolicy::RequireAll,
            },
            4 => Operation::Slide {
                left: clip_id("a"),
                clip: clip_id("b"),
                right: clip_id("c"),
                delta: time(delta),
                linked_policy: LinkedPolicy::RequireAll,
            },
            5 => Operation::Trim {
                clips: vec![clip_id("a")],
                edge: Edge::End,
                delta: time(delta),
                ripple_tracks: vec![track_id("main-video")],
                linked_policy: LinkedPolicy::RequireAll,
            },
            6 => split(id, &format!("right-{index}"), 5),
            7 => Operation::DeleteRange {
                edit: RangeEdit {
                    tracks: vec![track_id("main-video")],
                    range: range(start, end),
                    ripple: seed & 1 == 0,
                    remainders: remainders(),
                    right_groups: vec![],
                    linked_policy: LinkedPolicy::RequireAll,
                },
            },
            8 => Operation::Overwrite {
                track: track_id("main-video"),
                clip: shot(&format!("overwrite-{index}"), start, end),
                remainders: remainders(),
                right_groups: vec![],
                linked_policy: LinkedPolicy::RequireAll,
            },
            _ => {
                let ClipSource::Media { time_map, .. } = source(20, 30, 5 + delta) else {
                    unreachable!()
                };
                Operation::Retime {
                    targets: vec![RetimeTarget {
                        clip: clip_id(id),
                        time_map,
                        audio: AudioTimePolicy::default(),
                    }],
                    effects: EffectTimingPolicy::PreserveClipTime,
                    ripple_tracks: vec![],
                    linked_policy: LinkedPolicy::RequireAll,
                }
            }
        };
        let digest = current.digest().clone();
        match apply(&current, vec![operation]) {
            Ok(prepared) => {
                accepted += 1;
                assert!(
                    validate(prepared.candidate.document(), Default::default(), &|| false)
                        .unwrap()
                        .is_valid(),
                    "at {index}"
                );
                let inverse = undo(&prepared.candidate, &prepared.inverse)
                    .unwrap_or_else(|e| panic!("at {index}: {e}"));
                assert_eq!(inverse.candidate.digest(), &digest, "inverse at {index}");
                inverses += 1;
                if seed & 2 == 0 {
                    current = prepared.candidate;
                }
            }
            Err(_) => {
                refused += 1;
                assert_eq!(
                    current.document().content_digest().unwrap(),
                    digest,
                    "atomicity at {index}"
                );
            }
        }
    }
    assert!(accepted > 10_000 && refused > 10_000);
    eprintln!(
        "seed=0x57494e444f575f31 attempted=100000 accepted={accepted} refused={refused} inverse_roundtrips={inverses}"
    );
}
