use crate::*;
use mo_common::{Digest, digest};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct Fingerprints<'a> {
    doc: &'a VideoDocument,
    links: BTreeMap<&'a ClipId, &'a LinkGroup>,
}
impl<'a> Fingerprints<'a> {
    pub fn new(doc: &'a VideoDocument) -> Self {
        let links = doc
            .link_groups
            .values()
            .flat_map(|g| g.members.iter().map(move |id| (id, g)))
            .collect();
        Self { doc, links }
    }
    pub fn get(&self, key: &ObjectKey) -> Result<Digest> {
        let doc = self.doc;
        Ok(match key {
            ObjectKey::Header => digest(
                "musteroffice.video.header/1",
                &(
                    &doc.format,
                    &doc.id,
                    &doc.title,
                    doc.canvas,
                    doc.frame_rate,
                    doc.audio_sample_rate,
                    &doc.root_sequence,
                ),
            ),
            ObjectKey::Sequence(id) => {
                digest("musteroffice.video.sequence/1", &doc.sequences.get(id))
            }
            ObjectKey::Track(id) => digest("musteroffice.video.track/1", &doc.tracks.get(id)),
            // Link creation after an edit is a dependency too, even though it did not change the
            // clip record. Include reverse references so undo cannot silently break a new sync group.
            ObjectKey::Clip(id) => digest(
                "musteroffice.video.clip/1",
                &(doc.clips.get(id), self.links.get(id)),
            ),
            ObjectKey::Resource(id) => {
                digest("musteroffice.video.resource/1", &doc.resources.get(id))
            }
            ObjectKey::LinkGroup(id) => {
                digest("musteroffice.video.link-group/1", &doc.link_groups.get(id))
            }
        }?)
    }
}

fn original(doc: &VideoDocument, key: &ObjectKey) -> RestoreObject {
    match key {
        ObjectKey::Header => RestoreObject::Title {
            title: doc.title.clone(),
        },
        ObjectKey::Sequence(id) => RestoreObject::Sequence {
            id: id.clone(),
            value: doc.sequences.get(id).cloned(),
        },
        ObjectKey::Track(id) => RestoreObject::Track {
            id: id.clone(),
            value: doc.tracks.get(id).cloned(),
        },
        ObjectKey::Clip(id) => RestoreObject::Clip {
            id: id.clone(),
            value: doc.clips.get(id).cloned(),
        },
        ObjectKey::Resource(id) => RestoreObject::Resource {
            id: id.clone(),
            value: doc.resources.get(id).cloned(),
        },
        ObjectKey::LinkGroup(id) => RestoreObject::LinkGroup {
            id: id.clone(),
            value: doc.link_groups.get(id).cloned(),
        },
    }
}

pub(crate) fn restore(doc: &mut VideoDocument, value: &RestoreObject) {
    fn apply<K: Ord + Clone, V: Clone>(map: &mut ObjectMap<K, V>, id: &K, value: &Option<V>) {
        if let Some(value) = value {
            map.insert(id.clone(), value.clone());
        } else {
            map.remove(id);
        }
    }
    match value {
        RestoreObject::Title { title } => doc.title = title.clone(),
        RestoreObject::Sequence { id, value } => apply(&mut doc.sequences, id, value),
        RestoreObject::Track { id, value } => apply(&mut doc.tracks, id, value),
        RestoreObject::Clip { id, value } => apply(&mut doc.clips, id, value),
        RestoreObject::Resource { id, value } => apply(&mut doc.resources, id, value),
        RestoreObject::LinkGroup { id, value } => apply(&mut doc.link_groups, id, value),
    }
}

pub(crate) fn finish(
    base: &Snapshot,
    document: VideoDocument,
    mut reads: BTreeSet<ObjectKey>,
    writes: BTreeSet<ObjectKey>,
    request_digest: Digest,
    limits: EditLimits,
    cancelled: &dyn Fn() -> bool,
) -> Result<PreparedEdit> {
    if writes.len() > limits.max_changed_objects {
        return Err(EditError::Budget);
    }
    let candidate = Snapshot::new(document, limits.validation, cancelled)?;
    reads.extend(writes.iter().cloned());
    let versions = |doc: &VideoDocument| -> Result<Vec<ObjectVersion>> {
        let fingerprints = Fingerprints::new(doc);
        reads
            .iter()
            .map(|key| {
                if cancelled() {
                    return Err(EditError::Cancelled);
                }
                Ok(ObjectVersion {
                    object: key.clone(),
                    digest: fingerprints.get(key)?,
                })
            })
            .collect()
    };
    let read_set = versions(base.document())?;
    let inverse = Inverse {
        conditions: versions(candidate.document())?,
        restore: writes
            .iter()
            .map(|k| original(base.document(), k))
            .collect(),
    };
    // A candidate must remain undoable within this host's admission profile.
    admit(&inverse, limits, cancelled)?;
    let invalidations = invalidations(base.document(), candidate.document(), &writes, cancelled)?;
    Ok(PreparedEdit {
        base_digest: base.digest.clone(),
        request_digest,
        candidate,
        read_set,
        write_set: writes.into_iter().collect(),
        inverse,
        invalidations,
    })
}

fn invalidations(
    before: &VideoDocument,
    after: &VideoDocument,
    writes: &BTreeSet<ObjectKey>,
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<Invalidation>> {
    let mut ranges: BTreeMap<TrackId, Vec<TimeRange>> = BTreeMap::new();
    for doc in [before, after] {
        let track_sequences: BTreeMap<_, _> = doc
            .sequences
            .values()
            .flat_map(|s| s.tracks.iter().map(move |t| (t, &s.id)))
            .collect();
        let mut dirty_sequences = BTreeSet::new();
        let layout_dirty: BTreeSet<_> = doc
            .sequences
            .values()
            .filter(|s| {
                writes.contains(&ObjectKey::Sequence(s.id.clone()))
                    || s.tracks
                        .iter()
                        .any(|t| writes.contains(&ObjectKey::Track(t.clone())))
            })
            .map(|s| &s.id)
            .collect();
        let mut parents: BTreeMap<&SequenceId, Vec<(&TrackId, &Clip)>> = BTreeMap::new();
        for track in doc.tracks.values() {
            if cancelled() {
                return Err(EditError::Cancelled);
            }
            for id in &track.clips {
                let clip = &doc.clips[id];
                let resource_dirty = match &clip.source {
                    ClipSource::Media { resource, .. } => {
                        writes.contains(&ObjectKey::Resource(resource.clone()))
                    }
                    ClipSource::Text { font, .. } => {
                        writes.contains(&ObjectKey::Resource(font.clone()))
                    }
                    _ => false,
                };
                if let ClipSource::Sequence { sequence, .. } = &clip.source {
                    parents.entry(sequence).or_default().push((&track.id, clip));
                }
                let seq = track_sequences[&track.id];
                if writes.contains(&ObjectKey::Clip(id.clone()))
                    || writes.contains(&ObjectKey::Track(track.id.clone()))
                    || layout_dirty.contains(seq)
                    || resource_dirty
                {
                    ranges
                        .entry(track.id.clone())
                        .or_default()
                        .push(clip.timeline);
                    dirty_sequences.insert(seq);
                }
            }
        }
        let mut pending: Vec<_> = dirty_sequences.iter().copied().collect();
        while let Some(sequence) = pending.pop() {
            if cancelled() {
                return Err(EditError::Cancelled);
            }
            if let Some(dependents) = parents.get(sequence) {
                for (track, clip) in dependents {
                    ranges
                        .entry((*track).clone())
                        .or_default()
                        .push(clip.timeline);
                    let parent = track_sequences[track];
                    if dirty_sequences.insert(parent) {
                        pending.push(parent);
                    }
                }
            }
        }
    }
    Ok(ranges
        .into_iter()
        .map(|(track, mut ranges)| {
            ranges.sort_by(|a, b| {
                a.start
                    .compare_time(b.start)
                    .then(a.end.compare_time(b.end))
            });
            let mut merged: Vec<TimeRange> = vec![];
            for range in ranges {
                if let Some(last) = merged.last_mut()
                    && range.start.compare_time(last.end).is_le()
                {
                    if range.end.compare_time(last.end).is_gt() {
                        last.end = range.end;
                    }
                } else {
                    merged.push(range);
                }
            }
            Invalidation {
                track,
                ranges: merged,
            }
        })
        .collect())
}
