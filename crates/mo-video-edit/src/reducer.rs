use crate::{timing::*, *};
use mo_common::RationalTime;
use std::collections::{BTreeMap, BTreeSet};
mod range;
mod retime;

pub(crate) struct State<'a> {
    pub document: VideoDocument,
    pub reads: BTreeSet<ObjectKey>,
    pub writes: BTreeSet<ObjectKey>,
    owners: BTreeMap<ClipId, TrackId>,
    track_sequences: BTreeMap<TrackId, SequenceId>,
    limits: EditLimits,
    cancelled: &'a dyn Fn() -> bool,
}
impl<'a> State<'a> {
    pub fn new(
        document: VideoDocument,
        limits: EditLimits,
        cancelled: &'a dyn Fn() -> bool,
    ) -> Self {
        let owners = document
            .tracks
            .values()
            .flat_map(|t| t.clips.iter().map(move |c| (c.clone(), t.id.clone())))
            .collect();
        let track_sequences = document
            .sequences
            .values()
            .flat_map(|s| s.tracks.iter().map(move |t| (t.clone(), s.id.clone())))
            .collect();
        Self {
            document,
            reads: BTreeSet::new(),
            writes: BTreeSet::new(),
            owners,
            track_sequences,
            limits,
            cancelled,
        }
    }
    fn poll(&self) -> Result<()> {
        if (self.cancelled)() {
            Err(EditError::Cancelled)
        } else {
            Ok(())
        }
    }
    pub fn check_budget(&self) -> Result<()> {
        self.poll()?;
        let d = &self.document;
        let count = d
            .clips
            .len()
            .saturating_add(d.tracks.len())
            .saturating_add(d.resources.len())
            .saturating_add(d.sequences.len())
            .saturating_add(d.link_groups.len());
        if self.writes.len() > self.limits.max_changed_objects
            || count > self.limits.validation.max_nodes
        {
            return Err(EditError::Budget);
        }
        Ok(())
    }
    fn bits(&self) -> u64 {
        self.limits.validation.exact_bits
    }
    fn write(&mut self, key: ObjectKey) {
        self.reads.insert(key.clone());
        self.writes.insert(key);
    }
    fn clip(&mut self, id: &ClipId) -> Result<Clip> {
        self.poll()?;
        let key = ObjectKey::Clip(id.clone());
        self.reads.insert(key.clone());
        let clip = self
            .document
            .clips
            .get(id)
            .cloned()
            .ok_or(EditError::Missing(key))?;
        match &clip.source {
            ClipSource::Media { resource, .. } => {
                self.reads.insert(ObjectKey::Resource(resource.clone()));
            }
            ClipSource::Sequence { sequence, .. } => {
                self.reads.insert(ObjectKey::Sequence(sequence.clone()));
            }
            ClipSource::Text { font, .. } => {
                self.reads.insert(ObjectKey::Resource(font.clone()));
            }
            ClipSource::Solid { .. } => {}
        }
        Ok(clip)
    }
    fn track(&mut self, id: &TrackId, unlocked: bool) -> Result<&Track> {
        self.poll()?;
        let key = ObjectKey::Track(id.clone());
        self.reads.insert(key.clone());
        let track = self
            .document
            .tracks
            .get(id)
            .ok_or(EditError::Missing(key))?;
        if unlocked && track.locked {
            return Err(EditError::Locked(id.clone()));
        }
        Ok(track)
    }
    fn owner(&mut self, id: &ClipId) -> Result<TrackId> {
        let owner = self
            .owners
            .get(id)
            .cloned()
            .ok_or_else(|| EditError::Missing(ObjectKey::Clip(id.clone())))?;
        self.track(&owner, true)?;
        Ok(owner)
    }
    fn put_clip(&mut self, clip: Clip) {
        self.write(ObjectKey::Clip(clip.id.clone()));
        self.document.clips.insert(clip.id.clone(), clip);
    }
    fn put_track(&mut self, track: Track) {
        self.write(ObjectKey::Track(track.id.clone()));
        self.document.tracks.insert(track.id.clone(), track);
    }
    fn selection(&mut self, clips: &[ClipId], policy: LinkedPolicy) -> Result<BTreeSet<ClipId>> {
        let selection: BTreeSet<_> = clips.iter().cloned().collect();
        if selection.is_empty() || selection.len() != clips.len() {
            return Err(EditError::InvalidOperation(
                "empty or repeated clip selection",
            ));
        }
        for id in clips {
            self.clip(id)?;
            self.owner(id)?;
        }
        for group in self.document.link_groups.values() {
            if group.members.iter().any(|c| selection.contains(c)) {
                self.reads.insert(ObjectKey::LinkGroup(group.id.clone()));
                if policy == LinkedPolicy::RequireAll
                    && group.members.iter().any(|c| !selection.contains(c))
                {
                    return Err(EditError::LinkedSelection(group.id.clone()));
                }
            }
        }
        Ok(selection)
    }
    fn fresh_clip(&mut self, id: &ClipId) -> Result<()> {
        let key = ObjectKey::Clip(id.clone());
        self.reads.insert(key.clone());
        if self.document.clips.contains_key(id) {
            return Err(EditError::Duplicate(key));
        }
        Ok(())
    }
    fn sequence_for(&mut self, track: &TrackId) -> Result<SequenceId> {
        let sequence = self
            .track_sequences
            .get(track)
            .cloned()
            .ok_or(EditError::InvalidOperation("unowned track"))?;
        self.reads.insert(ObjectKey::Sequence(sequence.clone()));
        Ok(sequence)
    }
    fn selection_sequence(&mut self, ids: &BTreeSet<ClipId>) -> Result<SequenceId> {
        let tracks: BTreeSet<_> = ids.iter().map(|id| self.owners[id].clone()).collect();
        let sequences = tracks
            .iter()
            .map(|t| self.sequence_for(t))
            .collect::<Result<BTreeSet<_>>>()?;
        if sequences.len() != 1 {
            return Err(EditError::InvalidOperation(
                "ripple targets must share one sequence",
            ));
        }
        Ok(sequences.into_iter().next().expect("one sequence"))
    }
    fn ripple(
        &mut self,
        tracks: &[TrackId],
        at: RationalTime,
        delta: RationalTime,
        exclude: &BTreeSet<ClipId>,
        sequence: &SequenceId,
    ) -> Result<()> {
        let unique: BTreeSet<_> = tracks.iter().collect();
        if unique.len() != tracks.len() {
            return Err(EditError::InvalidOperation("duplicate ripple track"));
        }
        let mut shifted_ids = vec![];
        for id in tracks {
            if &self.sequence_for(id)? != sequence {
                return Err(EditError::InvalidOperation(
                    "ripple cannot cross sequence time coordinates",
                ));
            }
            for clip_id in self.track(id, true)?.clips.clone() {
                if exclude.contains(&clip_id) {
                    continue;
                }
                let clip = self.clip(&clip_id)?;
                if clip.timeline.start.compare_time(at).is_ge() {
                    shifted_ids.push(clip_id);
                } else if clip.timeline.end.compare_time(at).is_gt() {
                    return Err(EditError::InvalidOperation(
                        "ripple crosses an unsplit clip",
                    ));
                }
            }
        }
        if shifted_ids.is_empty() {
            return Ok(());
        }
        self.selection(&shifted_ids, LinkedPolicy::RequireAll)?;
        for id in shifted_ids {
            let clip = self.clip(&id)?;
            self.put_clip(shifted(&clip, delta, self.bits())?);
        }
        Ok(())
    }

    pub fn apply(&mut self, op: &Operation) -> Result<()> {
        self.poll()?;
        match op {
            Operation::SetTitle { title } => {
                self.write(ObjectKey::Header);
                self.document.title = title.clone();
            }
            Operation::AddSequence { sequence } => {
                let key = ObjectKey::Sequence(sequence.id.clone());
                if self.document.sequences.contains_key(&sequence.id) {
                    return Err(EditError::Duplicate(key));
                }
                if !sequence.tracks.is_empty() {
                    return Err(EditError::InvalidOperation("new sequence must be empty"));
                }
                self.write(key);
                self.document
                    .sequences
                    .insert(sequence.id.clone(), sequence.clone());
            }
            Operation::AddTrack {
                sequence,
                track,
                before,
            } => {
                let key = ObjectKey::Track(track.id.clone());
                if self.document.tracks.contains_key(&track.id) {
                    return Err(EditError::Duplicate(key));
                }
                if !track.clips.is_empty() {
                    return Err(EditError::InvalidOperation("new track must be empty"));
                }
                let skey = ObjectKey::Sequence(sequence.clone());
                self.reads.insert(skey.clone());
                let mut seq = self
                    .document
                    .sequences
                    .get(sequence)
                    .cloned()
                    .ok_or_else(|| EditError::Missing(skey.clone()))?;
                let index = match before {
                    Some(id) => seq
                        .tracks
                        .iter()
                        .position(|t| t == id)
                        .ok_or(EditError::InvalidOperation("before track not in sequence"))?,
                    None => seq.tracks.len(),
                };
                seq.tracks.insert(index, track.id.clone());
                self.write(skey);
                self.document.sequences.insert(sequence.clone(), seq);
                self.track_sequences
                    .insert(track.id.clone(), sequence.clone());
                self.put_track(track.clone());
            }
            Operation::RemoveTrack { track } => {
                if !self.track(track, true)?.clips.is_empty() {
                    return Err(EditError::InvalidOperation(
                        "track must be empty before removal",
                    ));
                }
                let owner = self
                    .document
                    .sequences
                    .values()
                    .find(|s| s.tracks.contains(track))
                    .cloned()
                    .ok_or(EditError::InvalidOperation("track has no sequence"))?;
                let mut seq = owner;
                seq.tracks.retain(|t| t != track);
                self.write(ObjectKey::Sequence(seq.id.clone()));
                self.document.sequences.insert(seq.id.clone(), seq);
                self.write(ObjectKey::Track(track.clone()));
                self.document.tracks.remove(track);
                self.track_sequences.remove(track);
            }
            Operation::SetTrack {
                track,
                name,
                locked,
                muted,
                solo,
            } => {
                let mut value = self.track(track, false)?.clone();
                value.name = name.clone();
                value.locked = *locked;
                value.muted = *muted;
                value.solo = *solo;
                self.put_track(value);
            }
            Operation::AddResource { resource } => {
                let key = ObjectKey::Resource(resource.id.clone());
                if self.document.resources.contains_key(&resource.id) {
                    return Err(EditError::Duplicate(key));
                }
                self.write(key);
                self.document
                    .resources
                    .insert(resource.id.clone(), resource.clone());
            }
            Operation::RemoveResource { resource } => {
                let key = ObjectKey::Resource(resource.clone());
                if !self.document.resources.contains_key(resource) {
                    return Err(EditError::Missing(key));
                }
                self.write(key);
                self.document.resources.remove(resource);
            }
            Operation::Insert {
                track,
                clip,
                ripple_tracks,
            } => {
                self.fresh_clip(&clip.id)?;
                self.track(track, true)?;
                let sequence = self.sequence_for(track)?;
                self.ripple(
                    ripple_tracks,
                    clip.timeline.start,
                    clip.timeline.duration(self.bits())?.to_time_exact()?,
                    &BTreeSet::new(),
                    &sequence,
                )?;
                let mut owner = self.track(track, true)?.clone();
                owner.clips.push(clip.id.clone());
                self.put_track(owner);
                self.owners.insert(clip.id.clone(), track.clone());
                self.put_clip(clip.clone());
            }
            Operation::Overwrite {
                track,
                clip,
                remainders,
                right_groups,
                linked_policy,
            } => {
                self.fresh_clip(&clip.id)?;
                self.delete_range(&RangeEdit {
                    tracks: vec![track.clone()],
                    range: clip.timeline,
                    ripple: false,
                    remainders: remainders.clone(),
                    right_groups: right_groups.clone(),
                    linked_policy: *linked_policy,
                })?;
                // A right remainder must not consume the replacement's identity.
                self.fresh_clip(&clip.id)?;
                let mut owner = self.track(track, true)?.clone();
                owner.clips.push(clip.id.clone());
                self.put_track(owner);
                self.owners.insert(clip.id.clone(), track.clone());
                self.put_clip(clip.clone());
            }
            Operation::DeleteRange { edit } => self.delete_range(edit)?,
            Operation::Retime {
                targets,
                effects,
                ripple_tracks,
                linked_policy,
            } => self.retime(targets, *effects, ripple_tracks, *linked_policy)?,
            Operation::Delete {
                clips,
                linked_policy,
            } => {
                let selection = self.selection(clips, *linked_policy)?;
                let owners: BTreeSet<_> = clips.iter().map(|id| self.owners[id].clone()).collect();
                for owner in owners {
                    let mut track = self.track(&owner, true)?.clone();
                    track.clips.retain(|c| !selection.contains(c));
                    self.put_track(track);
                }
                for id in clips {
                    self.poll()?;
                    self.write(ObjectKey::Clip(id.clone()));
                    self.document.clips.remove(id);
                    self.owners.remove(id);
                }
                let groups: Vec<_> = self
                    .document
                    .link_groups
                    .values()
                    .filter(|g| g.members.iter().any(|m| selection.contains(m)))
                    .cloned()
                    .collect();
                for mut group in groups {
                    // Undoing deletion may restore a dissolved group; a later edit of any
                    // surviving member must conflict instead of restoring an obsolete link.
                    self.reads
                        .extend(group.members.iter().cloned().map(ObjectKey::Clip));
                    self.write(ObjectKey::LinkGroup(group.id.clone()));
                    group.members.retain(|m| !selection.contains(m));
                    if group.members.len() < 2 {
                        self.document.link_groups.remove(&group.id);
                    } else {
                        self.document.link_groups.insert(group.id.clone(), group);
                    }
                }
            }
            Operation::Move {
                clips,
                delta,
                target_track,
                linked_policy,
            } => {
                let selection = self.selection(clips, *linked_policy)?;
                if let Some(target) = target_track {
                    self.track(target, true)?;
                    let owners: BTreeSet<_> =
                        clips.iter().map(|id| self.owners[id].clone()).collect();
                    for owner in owners {
                        if &owner == target {
                            continue;
                        }
                        let mut track = self.track(&owner, true)?.clone();
                        track.clips.retain(|c| !selection.contains(c));
                        self.put_track(track);
                    }
                    let mut track = self.track(target, true)?.clone();
                    for id in clips {
                        if &self.owners[id] != target {
                            track.clips.push(id.clone());
                            self.owners.insert(id.clone(), target.clone());
                        }
                    }
                    self.put_track(track);
                }
                for id in clips {
                    let clip = self.clip(id)?;
                    self.put_clip(shifted(&clip, *delta, self.bits())?);
                }
            }

            Operation::Split {
                targets,
                at,
                right_groups,
                linked_policy,
            } => self.split(targets, *at, right_groups, *linked_policy)?,
            Operation::Trim {
                clips,
                edge,
                delta,
                ripple_tracks,
                linked_policy,
            } => self.trim(clips, *edge, *delta, ripple_tracks, *linked_policy)?,
            Operation::Slip {
                clips,
                delta,
                linked_policy,
            } => {
                self.selection(clips, *linked_policy)?;
                for id in clips {
                    let mut clip = self.clip(id)?;
                    let (ClipSource::Media { time_map, .. }
                    | ClipSource::Sequence { time_map, .. }) = &mut clip.source
                    else {
                        return Err(EditError::InvalidOperation("source has no slip time map"));
                    };
                    for anchor in &mut time_map.anchors {
                        self.poll()?;
                        anchor.source = add(anchor.source, *delta, self.bits())?;
                    }
                    self.put_clip(clip);
                }
            }
            Operation::Roll {
                left,
                right,
                delta,
                linked_policy,
            } => {
                self.selection(&[left.clone(), right.clone()], *linked_policy)?;
                let (a, b) = self.adjacent(left, right)?;
                let cut = add(a.timeline.end, *delta, self.bits())?;
                self.put_clip(window(&a, a.timeline.start, cut, self.bits())?);
                self.put_clip(window(&b, cut, b.timeline.end, self.bits())?);
            }
            Operation::Slide {
                left,
                clip,
                right,
                delta,
                linked_policy,
            } => {
                self.selection(&[left.clone(), clip.clone(), right.clone()], *linked_policy)?;
                let (a, b) = self.adjacent(left, clip)?;
                let (_, c) = self.adjacent(clip, right)?;
                self.put_clip(window(
                    &a,
                    a.timeline.start,
                    add(a.timeline.end, *delta, self.bits())?,
                    self.bits(),
                )?);
                self.put_clip(shifted(&b, *delta, self.bits())?);
                self.put_clip(window(
                    &c,
                    add(c.timeline.start, *delta, self.bits())?,
                    c.timeline.end,
                    self.bits(),
                )?);
            }
            Operation::SetEffects { clip, effects } => {
                self.owner(clip)?;
                let mut value = self.clip(clip)?;
                value.effects = effects.clone();
                self.put_clip(value);
            }
            Operation::ReplaceSource { clip, source } => {
                self.owner(clip)?;
                let mut value = self.clip(clip)?;
                value.source = source.clone();
                self.put_clip(value);
            }
            Operation::Link { group } => self.link(group)?,
            Operation::Unlink { group } => {
                let key = ObjectKey::LinkGroup(group.clone());
                let members = self
                    .document
                    .link_groups
                    .get(group)
                    .ok_or_else(|| EditError::Missing(key.clone()))?
                    .members
                    .clone();
                for id in members {
                    self.clip(&id)?;
                    self.owner(&id)?;
                }
                self.write(key);
                self.document.link_groups.remove(group);
            }
        }
        Ok(())
    }
    fn adjacent(&mut self, left: &ClipId, right: &ClipId) -> Result<(Clip, Clip)> {
        if self.owner(left)? != self.owner(right)? {
            return Err(EditError::InvalidOperation("clips must share a track"));
        }
        let a = self.clip(left)?;
        let b = self.clip(right)?;
        if a.timeline.end.compare_time(b.timeline.start).is_ne() {
            return Err(EditError::InvalidOperation("clips must be adjacent"));
        }
        Ok((a, b))
    }
    fn link(&mut self, group: &LinkGroup) -> Result<()> {
        let key = ObjectKey::LinkGroup(group.id.clone());
        if self.document.link_groups.contains_key(&group.id) {
            return Err(EditError::Duplicate(key));
        }
        self.selection(&group.members, LinkedPolicy::Independent)?;
        self.write(key);
        self.document
            .link_groups
            .insert(group.id.clone(), group.clone());
        Ok(())
    }
    fn split(
        &mut self,
        targets: &[SplitTarget],
        at: RationalTime,
        right_groups: &[LinkGroup],
        policy: LinkedPolicy,
    ) -> Result<()> {
        let selection = self.selection(
            &targets.iter().map(|t| t.clip.clone()).collect::<Vec<_>>(),
            policy,
        )?;
        let mapping: BTreeMap<_, _> = targets
            .iter()
            .map(|t| (t.clip.clone(), t.right.clone()))
            .collect();
        if policy == LinkedPolicy::RequireAll {
            let expected: BTreeSet<BTreeSet<ClipId>> = self
                .document
                .link_groups
                .values()
                .filter(|g| g.members.iter().any(|m| selection.contains(m)))
                .map(|g| g.members.iter().map(|m| mapping[m].clone()).collect())
                .collect();
            let supplied: BTreeSet<BTreeSet<ClipId>> = right_groups
                .iter()
                .map(|g| g.members.iter().cloned().collect())
                .collect();
            if expected != supplied {
                return Err(EditError::InvalidOperation(
                    "split must preserve right-side link groups with explicit identities",
                ));
            }
        } else if !right_groups.is_empty() {
            return Err(EditError::InvalidOperation(
                "independent split cannot imply new groups",
            ));
        }
        for target in targets {
            self.fresh_clip(&target.right)?;
            let clip = self.clip(&target.clip)?;
            if at.compare_time(clip.timeline.start).is_le()
                || at.compare_time(clip.timeline.end).is_ge()
            {
                return Err(EditError::InvalidOperation(
                    "split point must be inside clip",
                ));
            }
            let left = window(&clip, clip.timeline.start, at, self.bits())?;
            let mut right = window(&clip, at, clip.timeline.end, self.bits())?;
            right.id = target.right.clone();
            let owner = self.owner(&target.clip)?;
            self.owners.insert(target.right.clone(), owner);
            self.put_clip(left);
            self.put_clip(right);
        }
        let owners: BTreeSet<_> = targets
            .iter()
            .map(|t| self.owners[&t.clip].clone())
            .collect();
        for owner in owners {
            let mut track = self.track(&owner, true)?.clone();
            track.clips = track
                .clips
                .into_iter()
                .flat_map(|id| {
                    let right = mapping.get(&id).cloned();
                    std::iter::once(id).chain(right)
                })
                .collect();
            self.put_track(track);
        }
        for group in right_groups {
            self.link(group)?;
        }
        Ok(())
    }
    fn trim(
        &mut self,
        clips: &[ClipId],
        edge: Edge,
        delta: RationalTime,
        ripple_tracks: &[TrackId],
        policy: LinkedPolicy,
    ) -> Result<()> {
        let selection = self.selection(clips, policy)?;
        let mut boundary = None;
        for id in clips {
            let clip = self.clip(id)?;
            if !ripple_tracks.is_empty() {
                if boundary.is_some_and(|t: RationalTime| t.compare_time(clip.timeline.end).is_ne())
                {
                    return Err(EditError::InvalidOperation(
                        "ripple group must share its end boundary",
                    ));
                }
                boundary = Some(clip.timeline.end);
            }
            let mut trimmed = match edge {
                Edge::Start => window(
                    &clip,
                    add(clip.timeline.start, delta, self.bits())?,
                    clip.timeline.end,
                    self.bits(),
                )?,
                Edge::End => window(
                    &clip,
                    clip.timeline.start,
                    add(clip.timeline.end, delta, self.bits())?,
                    self.bits(),
                )?,
            };
            if edge == Edge::Start && !ripple_tracks.is_empty() {
                trimmed = shifted(&trimmed, sub(zero(), delta, self.bits())?, self.bits())?;
            }
            self.put_clip(trimmed);
        }
        if let Some(at) = boundary {
            let shift = if edge == Edge::Start {
                sub(zero(), delta, self.bits())?
            } else {
                delta
            };
            let sequence = self.selection_sequence(&selection)?;
            self.ripple(ripple_tracks, at, shift, &selection, &sequence)?;
        }
        Ok(())
    }
}
