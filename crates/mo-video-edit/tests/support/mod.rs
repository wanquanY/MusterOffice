#![allow(dead_code)]
use mo_common::{ByteLength, Digest, DocumentId, RationalTime, ResourceId};
use mo_video_edit::*;
use mo_video_model::*;
use std::collections::BTreeMap;

pub fn time(v: i64) -> RationalTime {
    RationalTime::new(v, 1).unwrap()
}
pub fn range(a: i64, b: i64) -> TimeRange {
    TimeRange {
        start: time(a),
        end: time(b),
    }
}
pub fn clip_id(v: &str) -> ClipId {
    ClipId::new(v).unwrap()
}
pub fn track_id(v: &str) -> TrackId {
    TrackId::new(v).unwrap()
}
pub fn group_id(v: &str) -> LinkGroupId {
    LinkGroupId::new(v).unwrap()
}
pub fn source(a: i64, b: i64, duration: i64) -> ClipSource {
    ClipSource::Media {
        resource: ResourceId::new("movie").unwrap(),
        stream: StreamId::new("video").unwrap(),
        time_map: TimeMap {
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
        },
    }
}
pub fn shot(id: &str, start: i64, end: i64) -> Clip {
    Clip {
        audio_context: None,
        id: clip_id(id),
        name: id.into(),
        timeline: range(start, end),
        source: source(start + 20, end + 20, end - start),
        effects: vec![],
        audio: AudioTimePolicy::default(),
    }
}
pub fn document() -> VideoDocument {
    let root = SequenceId::new("main").unwrap();
    let mut doc = VideoDocument::empty(
        DocumentId::new("film").unwrap(),
        root.clone(),
        Canvas {
            width: 1920,
            height: 1080,
        },
        FrameRate {
            numerator: 30000,
            denominator: 1001,
        },
    );
    let resource = ResourceId::new("movie").unwrap();
    doc.resources.insert(
        resource.clone(),
        Resource {
            id: resource,
            digest: Digest::from_sha256([7; 32]),
            byte_length: ByteLength::new(1000),
            descriptor: ResourceDescriptor::Media {
                streams: BTreeMap::from([(
                    StreamId::new("video").unwrap(),
                    MediaStream::Video {
                        available: range(-5, 100),
                        dimensions: doc.canvas,
                        frame_rate: None,
                    },
                )]),
            },
        },
    );
    for name in ["main-video", "overlay"] {
        let id = track_id(name);
        doc.sequences
            .get_mut(&root)
            .unwrap()
            .tracks
            .push(id.clone());
        doc.tracks.insert(
            id.clone(),
            Track {
                id,
                name: name.into(),
                kind: TrackKind::Video,
                locked: false,
                muted: false,
                solo: false,
                clips: vec![],
            },
        );
    }
    add(&mut doc, "main-video", shot("a", 0, 10));
    add(&mut doc, "main-video", shot("b", 10, 20));
    add(&mut doc, "main-video", shot("c", 20, 30));
    add(&mut doc, "overlay", shot("x", 0, 10));
    doc
}
pub fn add(doc: &mut VideoDocument, track: &str, clip: Clip) {
    doc.tracks
        .get_mut(&track_id(track))
        .unwrap()
        .clips
        .push(clip.id.clone());
    doc.clips.insert(clip.id.clone(), clip);
}
pub fn snapshot(doc: VideoDocument) -> Snapshot {
    Snapshot::new(doc, ValidationLimits::default(), &|| false).unwrap()
}
pub fn base() -> Snapshot {
    snapshot(document())
}
pub fn request(base: &Snapshot, operations: Vec<Operation>) -> Transaction {
    Transaction {
        contract: EditContract::V1,
        expected_digest: base.digest().clone(),
        operations,
    }
}
pub fn apply(base: &Snapshot, operations: Vec<Operation>) -> Result<PreparedEdit> {
    prepare(
        base,
        &request(base, operations),
        EditLimits::default(),
        &|| false,
    )
}
pub fn move_clip(id: &str, delta: i64) -> Operation {
    Operation::Move {
        clips: vec![clip_id(id)],
        delta: time(delta),
        target_track: None,
        linked_policy: LinkedPolicy::RequireAll,
    }
}
pub fn split(id: &str, right: &str, at: i64) -> Operation {
    Operation::Split {
        targets: vec![SplitTarget {
            clip: clip_id(id),
            right: clip_id(right),
        }],
        at: time(at),
        right_groups: vec![],
        linked_policy: LinkedPolicy::RequireAll,
    }
}
pub fn undo(base: &Snapshot, inverse: &Inverse) -> Result<PreparedEdit> {
    prepare_inverse(base, inverse, EditLimits::default(), &|| false)
}
pub fn map(clip: &Clip) -> &TimeMap {
    match &clip.source {
        ClipSource::Media { time_map, .. } | ClipSource::Sequence { time_map, .. } => time_map,
        _ => panic!("expected time map"),
    }
}
