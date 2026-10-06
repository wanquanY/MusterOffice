use mo_common::{ByteLength, Digest, DocumentId, RationalTime, ResourceId};
use mo_time::ExactRatio;
use mo_video_model::*;
use std::{cell::Cell, collections::BTreeMap};

fn time(t: i64) -> RationalTime {
    RationalTime::new(t, 1).unwrap()
}
fn range(a: i64, b: i64) -> TimeRange {
    TimeRange {
        start: time(a),
        end: time(b),
    }
}
fn seq(s: &str) -> SequenceId {
    SequenceId::new(s).unwrap()
}
fn track(s: &str) -> TrackId {
    TrackId::new(s).unwrap()
}
fn clip(s: &str) -> ClipId {
    ClipId::new(s).unwrap()
}
fn resource(s: &str) -> ResourceId {
    ResourceId::new(s).unwrap()
}
fn map(a: i64, b: i64, duration: i64) -> TimeMap {
    TimeMap {
        anchors: vec![
            TimeAnchor {
                local: time(0),
                source: time(a),
            },
            TimeAnchor {
                local: time(duration),
                source: time(b),
            },
        ],
    }
}
fn document() -> VideoDocument {
    let mut doc = VideoDocument::empty(
        DocumentId::new("film").unwrap(),
        seq("main"),
        Canvas {
            width: 1920,
            height: 1080,
        },
        FrameRate {
            numerator: 30000,
            denominator: 1001,
        },
    );
    let t = track("video");
    let c = clip("shot");
    let r = resource("source");
    let s = StreamId::new("v0").unwrap();
    doc.sequences
        .get_mut(&seq("main"))
        .unwrap()
        .tracks
        .push(t.clone());
    doc.tracks.insert(
        t.clone(),
        Track {
            id: t,
            name: String::new(),
            kind: TrackKind::Video,
            locked: false,
            muted: false,
            solo: false,
            clips: vec![c.clone()],
        },
    );
    doc.resources.insert(
        r.clone(),
        Resource {
            id: r.clone(),
            digest: Digest::from_sha256([1; 32]),
            byte_length: ByteLength::new(1024),
            descriptor: ResourceDescriptor::Media {
                streams: BTreeMap::from([(
                    s.clone(),
                    MediaStream::Video {
                        available: range(-1, 10),
                        dimensions: doc.canvas,
                        frame_rate: None,
                    },
                )]),
            },
        },
    );
    doc.clips.insert(
        c.clone(),
        Clip {
            audio_context: None,
            id: c,
            name: String::new(),
            timeline: range(0, 2),
            source: ClipSource::Media {
                resource: r,
                stream: s,
                time_map: map(-1, 1, 2),
            },
            effects: vec![],
            audio: AudioTimePolicy::default(),
        },
    );
    doc
}
fn report(d: &VideoDocument) -> ValidationReport {
    validate(d, ValidationLimits::default(), &|| false).unwrap()
}
fn has(d: &VideoDocument, c: ValidationCode) -> bool {
    report(d).issues.iter().any(|i| i.code == c)
}
fn add_solid(d: &mut VideoDocument, id: &str, start: i64, end: i64) {
    let c = clip(id);
    d.tracks
        .get_mut(&track("video"))
        .unwrap()
        .clips
        .push(c.clone());
    d.clips.insert(
        c.clone(),
        Clip {
            audio_context: None,
            id: c,
            name: String::new(),
            timeline: range(start, end),
            source: ClipSource::Solid {
                rgba: [0, 0, 0, 255],
            },
            effects: vec![],
            audio: AudioTimePolicy::default(),
        },
    );
}
#[test]
fn mixed_wire_units_negative_source_time_and_vfr_metadata_are_valid() {
    let d = document();
    assert!(report(&d).is_valid());
    let bytes = serde_json::to_string(&d).unwrap();
    let (decoded, r) = decode(&bytes, ValidationLimits::default(), &|| false).unwrap();
    assert!(r.is_valid());
    assert_eq!(
        d.content_digest().unwrap(),
        decoded.content_digest().unwrap()
    );
    assert!(bytes.contains("\"ticks\":\"-1\""));
}
#[test]
fn half_open_ranges_allow_adjacency_but_reject_overlap() {
    let mut d = document();
    add_solid(&mut d, "next", 2, 4);
    assert!(report(&d).is_valid());
    d.clips.get_mut(&clip("next")).unwrap().timeline.start = time(1);
    assert!(has(&d, ValidationCode::Overlap));
    assert!(range(0, 2).contains(time(0)));
    assert!(!range(0, 2).contains(time(2)));
}
#[test]
fn duplicate_ownership_or_orphan_objects_cannot_form_a_valid_project() {
    let mut d = document();
    d.tracks
        .get_mut(&track("video"))
        .unwrap()
        .clips
        .push(clip("shot"));
    assert!(has(&d, ValidationCode::OwnershipConflict));
    d.tracks.get_mut(&track("video")).unwrap().clips.clear();
    assert!(has(&d, ValidationCode::OwnershipConflict));
    d.clips.get_mut(&clip("shot")).unwrap().id = clip("other");
    assert!(has(&d, ValidationCode::IdentityMismatch));
}
#[test]
fn missing_stream_or_incompatible_track_is_not_silently_accepted() {
    let mut d = document();
    d.tracks.get_mut(&track("video")).unwrap().kind = TrackKind::Audio;
    assert!(has(&d, ValidationCode::IncompatibleTrack));
    d.resources.clear();
    assert!(has(&d, ValidationCode::MissingReference));
}
#[test]
fn time_map_rejects_out_of_bounds_hold_and_invalid_local_order() {
    let mut d = document();
    if let ClipSource::Media { time_map, .. } = &mut d.clips.get_mut(&clip("shot")).unwrap().source
    {
        *time_map = map(10, 10, 2);
    }
    assert!(has(&d, ValidationCode::InvalidValue));
    if let ClipSource::Media { time_map, .. } = &mut d.clips.get_mut(&clip("shot")).unwrap().source
    {
        *time_map = map(1, -1, 2);
    }
    assert!(report(&d).is_valid());
    if let ClipSource::Media { time_map, .. } = &mut d.clips.get_mut(&clip("shot")).unwrap().source
    {
        time_map.anchors[1].local = time(0);
    }
    assert!(has(&d, ValidationCode::InvalidValue));
}
#[test]
fn mapping_supports_reverse_hold_and_exclusive_end_exactly() {
    let m = TimeMap {
        anchors: vec![
            TimeAnchor {
                local: time(0),
                source: time(8),
            },
            TimeAnchor {
                local: time(4),
                source: time(2),
            },
            TimeAnchor {
                local: time(6),
                source: time(2),
            },
        ],
    };
    assert_eq!(
        m.map(time(1), 256)
            .unwrap()
            .unwrap()
            .to_time_exact()
            .unwrap(),
        RationalTime::new(13, 2).unwrap()
    );
    assert_eq!(
        m.map(time(5), 256)
            .unwrap()
            .unwrap()
            .compare(&ExactRatio::integer(2)),
        std::cmp::Ordering::Equal
    );
    assert_eq!(m.map(time(6), 256).unwrap(), None);
    assert_eq!(m.map(time(-1), 256).unwrap(), None);
}
#[test]
fn schema_unknown_fields_and_duplicate_json_keys_are_rejected() {
    let d = document();
    let mut v = serde_json::to_value(&d).unwrap();
    v["serverUrl"] = "https://invalid.example".into();
    assert!(decode(&v.to_string(), ValidationLimits::default(), &|| false).is_err());
    let raw = serde_json::to_string(&d).unwrap().replacen(
        "\"title\":",
        "\"title\":\"ambiguous\",\"title\":",
        1,
    );
    assert!(decode(&raw, ValidationLimits::default(), &|| false).is_err());
}
#[test]
fn cancellation_and_budgets_do_not_report_partial_success() {
    let d = document();
    assert!(matches!(
        validate(&d, ValidationLimits::default(), &|| true),
        Err(ValidationError::Cancelled)
    ));
    let limits = ValidationLimits {
        max_nodes: 0,
        ..ValidationLimits::default()
    };
    assert!(matches!(
        validate(&d, limits, &|| false),
        Err(ValidationError::Budget)
    ));
    let calls = Cell::new(0);
    let result = validate(&d, ValidationLimits::default(), &|| {
        calls.set(calls.get() + 1);
        calls.get() > 3
    });
    assert!(matches!(result, Err(ValidationError::Cancelled)));
    let limits = ValidationLimits {
        max_time_anchors: 1,
        ..ValidationLimits::default()
    };
    assert!(matches!(
        validate(&d, limits, &|| false),
        Err(ValidationError::Budget)
    ));
}
#[test]
fn diagnostic_truncation_never_turns_invalid_content_into_success() {
    let mut d = document();
    d.canvas.width = 0;
    d.frame_rate.denominator = 0;
    d.audio_sample_rate = 0;
    let r = validate(
        &d,
        ValidationLimits {
            max_issues: 1,
            ..ValidationLimits::default()
        },
        &|| false,
    )
    .unwrap();
    assert_eq!(r.issues.len(), 1);
    assert!(r.truncated);
    assert!(!r.is_valid());
}
#[test]
fn cycles_are_rejected_without_recursing_untrusted_sequences() {
    let mut d = document();
    d.clips.get_mut(&clip("shot")).unwrap().source = ClipSource::Sequence {
        sequence: seq("main"),
        time_map: map(0, 2, 2),
    };
    assert!(has(&d, ValidationCode::ReferenceCycle));
}
fn nested(d: &mut VideoDocument, name: &str, target: Option<&str>) {
    let s = seq(name);
    let t = track(name);
    let c = clip(name);
    d.sequences.insert(
        s.clone(),
        Sequence {
            id: s,
            tracks: vec![t.clone()],
        },
    );
    d.tracks.insert(
        t.clone(),
        Track {
            id: t,
            name: String::new(),
            kind: TrackKind::Video,
            locked: false,
            muted: false,
            solo: false,
            clips: vec![c.clone()],
        },
    );
    let source = target.map_or(
        ClipSource::Solid {
            rgba: [0, 0, 0, 255],
        },
        |target| ClipSource::Sequence {
            sequence: seq(target),
            time_map: map(0, 2, 2),
        },
    );
    d.clips.insert(
        c.clone(),
        Clip {
            audio_context: None,
            id: c,
            name: String::new(),
            timeline: range(0, 2),
            source,
            effects: vec![],
            audio: AudioTimePolicy::default(),
        },
    );
}
#[test]
fn shared_subgraphs_cannot_hide_excessive_nesting_depth() {
    let mut d = document();
    nested(&mut d, "a", Some("shared"));
    nested(&mut d, "b", Some("chain"));
    nested(&mut d, "chain", Some("shared"));
    nested(&mut d, "shared", None);
    assert!(report(&d).is_valid());
    let r = validate(
        &d,
        ValidationLimits {
            max_depth: 2,
            ..ValidationLimits::default()
        },
        &|| false,
    )
    .unwrap();
    assert!(
        r.issues
            .iter()
            .any(|i| i.code == ValidationCode::LimitExceeded)
    );
}
#[test]
fn linked_media_members_have_one_valid_group() {
    let mut d = document();
    add_solid(&mut d, "other", 2, 4);
    let id = LinkGroupId::new("group").unwrap();
    d.link_groups.insert(
        id.clone(),
        LinkGroup {
            id,
            members: vec![clip("shot"), clip("other")],
        },
    );
    assert!(report(&d).is_valid());
    let id = LinkGroupId::new("duplicate").unwrap();
    d.link_groups.insert(
        id.clone(),
        LinkGroup {
            id,
            members: vec![clip("shot"), clip("missing")],
        },
    );
    assert!(has(&d, ValidationCode::OwnershipConflict));
    assert!(has(&d, ValidationCode::MissingReference));
}
