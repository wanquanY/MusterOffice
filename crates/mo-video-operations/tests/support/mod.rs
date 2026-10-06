#![allow(dead_code)]
use mo_common::{DocumentId, RationalTime};
use mo_video_edit::*;
use mo_video_model::*;
use mo_video_operations::{CreateProject, Limits, VideoProject};
pub fn create() -> CreateProject {
    CreateProject {
        document_id: DocumentId::new("authoring").unwrap(),
        title: "示例工程".into(),
        root_sequence: SequenceId::new("main").unwrap(),
        canvas: Canvas {
            width: 1280,
            height: 720,
        },
        frame_rate: FrameRate {
            numerator: 25,
            denominator: 1,
        },
        audio_sample_rate: 48000,
    }
}
pub fn t(n: i64) -> RationalTime {
    RationalTime::new(n, 1).unwrap()
}
pub fn operations() -> Vec<Operation> {
    let mut ops = vec![Operation::AddTrack {
        sequence: SequenceId::new("main").unwrap(),
        track: Track {
            id: TrackId::new("v").unwrap(),
            name: "画面".into(),
            kind: TrackKind::Video,
            locked: false,
            muted: false,
            solo: false,
            clips: vec![],
        },
        before: None,
    }];
    for (name, start, end) in [("a", 0, 2), ("b", 3, 4), ("c", 6, 7)] {
        ops.push(Operation::Insert {
            track: TrackId::new("v").unwrap(),
            clip: Clip {
                id: ClipId::new(name).unwrap(),
                name: name.into(),
                timeline: TimeRange {
                    start: t(start),
                    end: t(end),
                },
                source: ClipSource::Solid {
                    rgba: [255, 0, 0, 255],
                },
                effects: vec![],
                audio: AudioTimePolicy::default(),
                audio_context: None,
            },
            ripple_tracks: vec![],
        });
    }
    ops
}
pub fn project() -> VideoProject {
    let empty = VideoProject::create(create(), Limits::default(), &|| false).unwrap();
    let candidate = empty
        .prepare(
            &Transaction {
                contract: EditContract::V1,
                expected_digest: empty.digest().clone(),
                operations: operations(),
            },
            &|| false,
        )
        .unwrap();
    VideoProject::open(candidate.snapshot, Limits::default(), &|| false).unwrap()
}
