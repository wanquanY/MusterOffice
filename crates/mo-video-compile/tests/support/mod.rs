#![allow(dead_code)]
use mo_common::{ByteLength, Digest, DocumentId, RationalTime, ResourceId};
use mo_video_model::*;
use std::collections::BTreeMap;

pub fn t(n: i64) -> RationalTime {
    RationalTime::new(n, 1).unwrap()
}
pub fn range(a: i64, b: i64) -> TimeRange {
    TimeRange {
        start: t(a),
        end: t(b),
    }
}
pub fn sid(s: &str) -> SequenceId {
    SequenceId::new(s).unwrap()
}
pub fn tid(s: &str) -> TrackId {
    TrackId::new(s).unwrap()
}
pub fn cid(s: &str) -> ClipId {
    ClipId::new(s).unwrap()
}
pub fn map(a: i64, b: i64, d: i64) -> TimeMap {
    TimeMap {
        anchors: vec![
            TimeAnchor {
                local: t(0),
                source: t(a),
            },
            TimeAnchor {
                local: t(d),
                source: t(b),
            },
        ],
    }
}
pub fn doc() -> VideoDocument {
    VideoDocument::empty(
        DocumentId::new("film").unwrap(),
        sid("main"),
        Canvas {
            width: 1920,
            height: 1080,
        },
        FrameRate {
            numerator: 30_000,
            denominator: 1001,
        },
    )
}
pub fn sequence(d: &mut VideoDocument, id: &str) {
    d.sequences.insert(
        sid(id),
        Sequence {
            id: sid(id),
            tracks: vec![],
        },
    );
}
pub fn track(d: &mut VideoDocument, sequence: &str, id: &str, kind: TrackKind) {
    d.sequences
        .get_mut(&sid(sequence))
        .unwrap()
        .tracks
        .push(tid(id));
    d.tracks.insert(
        tid(id),
        Track {
            id: tid(id),
            name: id.into(),
            kind,
            locked: false,
            muted: false,
            solo: false,
            clips: vec![],
        },
    );
}
pub fn clip(d: &mut VideoDocument, track: &str, id: &str, timeline: TimeRange, source: ClipSource) {
    d.tracks.get_mut(&tid(track)).unwrap().clips.push(cid(id));
    d.clips.insert(
        cid(id),
        Clip {
            audio_context: None,
            id: cid(id),
            name: id.into(),
            timeline,
            source,
            effects: vec![],
            audio: AudioTimePolicy::default(),
        },
    );
}
pub fn solid() -> ClipSource {
    ClipSource::Solid {
        rgba: [200, 100, 50, 255],
    }
}
pub fn nested(sequence: &str, mapping: TimeMap) -> ClipSource {
    ClipSource::Sequence {
        sequence: sid(sequence),
        time_map: mapping,
    }
}
pub fn media(d: &mut VideoDocument, audio: bool, mapping: TimeMap) -> ClipSource {
    let id = ResourceId::new(if audio { "audio" } else { "video" }).unwrap();
    let stream = StreamId::new("stream").unwrap();
    let descriptor = if audio {
        MediaStream::Audio {
            available: range(-10, 100),
            sample_rate: 44_100,
            channels: 2,
        }
    } else {
        MediaStream::Video {
            available: range(-10, 100),
            dimensions: d.canvas,
            frame_rate: None,
        }
    };
    d.resources.insert(
        id.clone(),
        Resource {
            id: id.clone(),
            digest: Digest::from_sha256([u8::from(audio); 32]),
            byte_length: ByteLength::new(100),
            descriptor: ResourceDescriptor::Media {
                streams: BTreeMap::from([(stream.clone(), descriptor)]),
            },
        },
    );
    ClipSource::Media {
        resource: id,
        stream,
        time_map: mapping,
    }
}
