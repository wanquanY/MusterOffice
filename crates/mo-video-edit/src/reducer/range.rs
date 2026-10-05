use super::*;

#[derive(Default)]
struct Fragments {
    before: Option<ClipId>,
    after: Option<ClipId>,
}
impl Fragments {
    fn split(&self) -> bool {
        self.before.is_some() && self.after.is_some()
    }
}

impl State<'_> {
    pub(super) fn delete_range(&mut self, edit: &RangeEdit) -> Result<()> {
        if !edit.range.valid() || edit.range.start.ticks.get() < 0 {
            return Err(EditError::InvalidOperation(
                "cut range must be positive and on the timeline",
            ));
        }
        let scope: BTreeSet<_> = edit.tracks.iter().cloned().collect();
        if scope.is_empty() || scope.len() != edit.tracks.len() {
            return Err(EditError::InvalidOperation(
                "empty or duplicate range track scope",
            ));
        }
        let mut sequences = BTreeSet::new();
        let mut tracks = vec![];
        let mut touched = BTreeSet::new();
        let mut needed = BTreeSet::new();
        for id in &edit.tracks {
            let track = self.track(id, true)?.clone();
            sequences.insert(self.sequence_for(id)?);
            for id in &track.clips {
                let clip = self.clip(id)?;
                if clip.timeline.intersects(edit.range)
                    || (edit.ripple && clip.timeline.start.compare_time(edit.range.end).is_ge())
                {
                    touched.insert(id.clone());
                }
                if clip.timeline.start.compare_time(edit.range.start).is_lt()
                    && clip.timeline.end.compare_time(edit.range.end).is_gt()
                {
                    needed.insert(id.clone());
                }
            }
            tracks.push(track);
        }
        if sequences.len() != 1 {
            return Err(EditError::InvalidOperation(
                "range tracks must share one sequence time coordinate",
            ));
        }
        self.reads
            .extend(sequences.into_iter().map(ObjectKey::Sequence));
        let mapping: BTreeMap<_, _> = edit
            .remainders
            .iter()
            .map(|r| (r.clip.clone(), r.right.clone()))
            .collect();
        let new_ids: BTreeSet<_> = mapping.values().cloned().collect();
        if mapping.len() != edit.remainders.len()
            || new_ids.len() != mapping.len()
            || mapping.keys().cloned().collect::<BTreeSet<_>>() != needed
        {
            return Err(EditError::InvalidOperation(
                "provide exactly one distinct right identity for every straddling clip",
            ));
        }
        for id in &new_ids {
            self.fresh_clip(id)?;
        }
        let groups: Vec<_> = self
            .document
            .link_groups
            .values()
            .filter(|g| g.members.iter().any(|m| touched.contains(m)))
            .cloned()
            .collect();
        for group in &groups {
            self.reads.insert(ObjectKey::LinkGroup(group.id.clone()));
            self.reads
                .extend(group.members.iter().cloned().map(ObjectKey::Clip));
            if edit.linked_policy == LinkedPolicy::RequireAll
                && group.members.iter().any(|m| {
                    self.owners
                        .get(m)
                        .is_none_or(|owner| !scope.contains(owner))
                })
            {
                return Err(EditError::LinkedSelection(group.id.clone()));
            }
        }
        self.check_right_groups(edit)?;
        let shift = sub(edit.range.start, edit.range.end, self.bits())?;
        let mut fragments = BTreeMap::new();
        for mut track in tracks {
            let old_ids = std::mem::take(&mut track.clips);
            let mut changed_members = false;
            for id in &old_ids {
                let clip = self.clip(id)?;
                let mut pieces = Fragments::default();
                if clip.timeline.end.compare_time(edit.range.start).is_le() {
                    pieces.before = Some(id.clone());
                    track.clips.push(id.clone());
                } else if clip.timeline.start.compare_time(edit.range.end).is_ge() {
                    pieces.after = Some(id.clone());
                    track.clips.push(id.clone());
                    if edit.ripple {
                        self.put_clip(shifted(&clip, shift, self.bits())?);
                    }
                } else {
                    let left = clip.timeline.start.compare_time(edit.range.start).is_lt();
                    let right = clip.timeline.end.compare_time(edit.range.end).is_gt();
                    if left {
                        let value =
                            window(&clip, clip.timeline.start, edit.range.start, self.bits())?;
                        self.put_clip(value);
                        pieces.before = Some(id.clone());
                        track.clips.push(id.clone());
                    }
                    if right {
                        let right_id = if left {
                            mapping[id].clone()
                        } else {
                            id.clone()
                        };
                        let mut value =
                            window(&clip, edit.range.end, clip.timeline.end, self.bits())?;
                        value.id = right_id.clone();
                        if edit.ripple {
                            value = shifted(&value, shift, self.bits())?;
                        }
                        self.put_clip(value);
                        self.owners.insert(right_id.clone(), track.id.clone());
                        track.clips.push(right_id.clone());
                        pieces.after = Some(right_id);
                    }
                    if !left && !right {
                        self.write(ObjectKey::Clip(id.clone()));
                        self.document.clips.remove(id);
                        self.owners.remove(id);
                    }
                    changed_members |= left == right; // both creates a new ID; neither removes one
                }
                fragments.insert(id.clone(), pieces);
            }
            if changed_members {
                self.put_track(track);
            }
        }
        self.relink_range(edit, groups, &fragments)?;
        self.check_budget()
    }

    fn check_right_groups(&mut self, edit: &RangeEdit) -> Result<()> {
        if edit.linked_policy == LinkedPolicy::Independent && !edit.right_groups.is_empty() {
            return Err(EditError::InvalidOperation(
                "independent range edit cannot imply right groups",
            ));
        }
        let mut ids = BTreeSet::new();
        let mut sets = BTreeSet::new();
        for group in &edit.right_groups {
            let key = ObjectKey::LinkGroup(group.id.clone());
            self.reads.insert(key.clone());
            if self.document.link_groups.contains_key(&group.id) {
                return Err(EditError::Duplicate(key));
            }
            let members: BTreeSet<_> = group.members.iter().cloned().collect();
            if members.len() < 2
                || members.len() != group.members.len()
                || !ids.insert(group.id.clone())
                || !sets.insert(members)
            {
                return Err(EditError::InvalidOperation(
                    "invalid or duplicate right link group",
                ));
            }
        }
        Ok(())
    }

    fn relink_range(
        &mut self,
        edit: &RangeEdit,
        groups: Vec<LinkGroup>,
        fragments: &BTreeMap<ClipId, Fragments>,
    ) -> Result<()> {
        let available: BTreeMap<BTreeSet<ClipId>, _> = edit
            .right_groups
            .iter()
            .map(|g| (g.members.iter().cloned().collect(), g))
            .collect();
        let mut used = BTreeSet::new();
        for mut group in groups {
            self.poll()?;
            self.write(ObjectKey::LinkGroup(group.id.clone()));
            if edit.linked_policy == LinkedPolicy::Independent
                || !group
                    .members
                    .iter()
                    .any(|id| fragments.get(id).is_some_and(Fragments::split))
            {
                group
                    .members
                    .retain(|id| self.document.clips.contains_key(id));
                self.put_group_or_remove(group);
                continue;
            }
            let left: Vec<_> = group
                .members
                .iter()
                .filter_map(|id| fragments[id].before.clone())
                .collect();
            let right: Vec<_> = group
                .members
                .iter()
                .filter_map(|id| fragments[id].after.clone())
                .collect();
            if left.len() >= 2 {
                if right.len() >= 2 {
                    let key: BTreeSet<_> = right.iter().cloned().collect();
                    let new_group = available.get(&key).ok_or(EditError::InvalidOperation(
                        "missing exact right fragment link group",
                    ))?;
                    used.insert(new_group.id.clone());
                    self.write(ObjectKey::LinkGroup(new_group.id.clone()));
                    self.document
                        .link_groups
                        .insert(new_group.id.clone(), (*new_group).clone());
                }
                group.members = left;
            } else {
                group.members = right;
            }
            self.put_group_or_remove(group);
        }
        if used.len() != edit.right_groups.len() {
            return Err(EditError::InvalidOperation(
                "unused right fragment link group",
            ));
        }
        Ok(())
    }
    fn put_group_or_remove(&mut self, group: LinkGroup) {
        if group.members.len() >= 2 {
            self.document.link_groups.insert(group.id.clone(), group);
        } else {
            self.document.link_groups.remove(&group.id);
        }
    }
}
