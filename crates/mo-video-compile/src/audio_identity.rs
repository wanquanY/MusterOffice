//! Audio-only dependency identity. Names, UI locks and visual-only edits do not
//! invalidate PCM; bus order, mute/solo, timing, effects and child audio do.
use crate::{CompileError, CompiledDocument, Result, project::SequenceIndex};
use mo_common::{Digest, digest};
use mo_video_model::*;
use serde::Serialize;
use std::collections::BTreeMap;

pub(crate) fn identities(
    document: &VideoDocument,
    indexes: &BTreeMap<SequenceId, SequenceIndex>,
    check: &dyn Fn() -> bool,
) -> Result<BTreeMap<SequenceId, Digest>> {
    let mut cache = BTreeMap::new();
    for id in document.sequences.keys() {
        sequence(document, indexes, id, &mut cache, check)?;
    }
    Ok(cache)
}
fn sequence(
    document: &VideoDocument,
    indexes: &BTreeMap<SequenceId, SequenceIndex>,
    id: &SequenceId,
    cache: &mut BTreeMap<SequenceId, Digest>,
    check: &dyn Fn() -> bool,
) -> Result<Digest> {
    if check() {
        return Err(CompileError::Cancelled);
    }
    if let Some(value) = cache.get(id) {
        return Ok(value.clone());
    }
    // The document already passed cycle/depth/size validation. Each sequence
    // body is computed once; shared child references only clone its digest.
    let mut tracks = Vec::new();
    for track_id in &indexes[id].audio_tracks {
        let mut clips = Vec::new();
        for clip_id in &document.tracks[track_id].clips {
            if check() {
                return Err(CompileError::Cancelled);
            }
            let clip = &document.clips[clip_id];
            let (source, time_map) = match &clip.source {
                ClipSource::Media {
                    resource,
                    stream,
                    time_map,
                } => {
                    let resource = &document.resources[resource];
                    let ResourceDescriptor::Media { streams } = &resource.descriptor else {
                        return Err(CompileError::Invariant("audio resource descriptor"));
                    };
                    (
                        digest(
                            "musteroffice.audio.media-input/1",
                            &(&resource.digest, stream, &streams[stream]),
                        )?,
                        time_map,
                    )
                }
                ClipSource::Sequence {
                    sequence: child,
                    time_map,
                } => (sequence(document, indexes, child, cache, check)?, time_map),
                _ => return Err(CompileError::Invariant("audio source")),
            };
            #[derive(Serialize)]
            struct ClipKey<'a> {
                timeline: TimeRange,
                source: Digest,
                time_map: &'a TimeMap,
                context: &'a Option<AudioRenderContext>,
                policy: AudioTimePolicy,
                effects: &'a [Effect],
            }
            clips.push(digest(
                "musteroffice.audio.clip-input/1",
                &ClipKey {
                    timeline: clip.timeline,
                    source,
                    time_map,
                    context: &clip.audio_context,
                    policy: clip.audio,
                    effects: &clip.effects,
                },
            )?);
        }
        tracks.push(clips);
    }
    let result = digest(
        "musteroffice.audio.sequence-input/1",
        &(document.audio_sample_rate, tracks),
    )?;
    cache.insert(id.clone(), result.clone());
    Ok(result)
}
impl CompiledDocument {
    pub fn audio_sequence_digest(&self, sequence: &SequenceId) -> Result<&Digest> {
        self.audio_digests
            .get(sequence)
            .ok_or(CompileError::Invariant("audio sequence identity"))
    }
}
