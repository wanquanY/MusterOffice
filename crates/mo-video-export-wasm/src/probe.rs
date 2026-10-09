//! Fixed resource-free input for an actual host capability measurement. This is
//! a computation fixture, never a user project or proof that a codec ran.
use crate::{Result, compile_error, invalid, json};
use mo_common::{DocumentId, RationalTime};
use mo_video_compile::{CompileLimits, CompiledDocument};
use mo_video_model::{
    AudioTimePolicy, Canvas, Clip, ClipId, ClipSource, FrameRate, SequenceId, TimeRange, Track,
    TrackId, TrackKind, VideoDocument,
};

pub(super) fn fixture() -> Result<String> {
    let root = SequenceId::new("main").map_err(|_| invalid("probe sequence"))?;
    let track_id = TrackId::new("visual").map_err(|_| invalid("probe track"))?;
    let mut document = VideoDocument::empty(
        DocumentId::new("export-capability-probe").map_err(|_| invalid("probe document"))?,
        root.clone(),
        Canvas {
            width: 128,
            height: 96,
        },
        FrameRate {
            numerator: 30,
            denominator: 1,
        },
    );
    document
        .sequences
        .get_mut(&root)
        .ok_or_else(|| invalid("probe root"))?
        .tracks
        .push(track_id.clone());
    let mut track = Track {
        id: track_id.clone(),
        name: String::new(),
        kind: TrackKind::Graphics,
        locked: false,
        muted: false,
        solo: false,
        clips: Vec::new(),
    };
    for (name, start, rgba) in [("red", 0, [255, 0, 0, 255]), ("blue", 1, [0, 0, 255, 255])] {
        let id = ClipId::new(name).map_err(|_| invalid("probe clip"))?;
        track.clips.push(id.clone());
        document.clips.insert(
            id.clone(),
            Clip {
                id,
                name: String::new(),
                timeline: TimeRange {
                    start: RationalTime::new(start, 10).map_err(|_| invalid("probe time"))?,
                    end: RationalTime::new(start + 1, 10).map_err(|_| invalid("probe time"))?,
                },
                source: ClipSource::Solid { rgba },
                effects: Vec::new(),
                audio: AudioTimePolicy::default(),
                audio_context: None,
            },
        );
    }
    document.tracks.insert(track_id, track);
    let compiled = CompiledDocument::new(document.clone(), CompileLimits::default(), &|| false)
        .map_err(compile_error)?;
    json(&serde_json::json!({
        "format":"musteroffice.video.export-probe-fixture/1", "document": document,
        "documentDigest":compiled.digest(),
        "options": {"videoBitRate":8000000,"gopFrames":30,
            "audio":{"kind":"stereo","bitRate":192000},"audioBlockFrames":4096,
            "backgroundSrgb":[0,0,0],"sourcePeakNits":null}
    }))
}
