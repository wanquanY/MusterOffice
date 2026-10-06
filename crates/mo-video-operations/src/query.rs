use crate::*;
use mo_video_model::{Clip, Track};
impl VideoProject {
    pub fn list(&self, request: PageRequest, check: &dyn Fn() -> bool) -> Result<ProjectPage> {
        cancel(check)?;
        if request.limit == 0 || request.limit > self.limits.max_page_items {
            return Err(Failure::new(FailureCode::LimitExceeded, "page item limit"));
        }
        let query_digest = mo_common::digest("musteroffice.video.list/1", &request.query)
            .map_err(|_| Failure::new(FailureCode::InputInvalid, "invalid list query"))?;
        let start = if let Some(cursor) = request.cursor {
            self.expect(&cursor.document_digest)?;
            if cursor.query_digest != query_digest {
                return Err(Failure::new(
                    FailureCode::InputInvalid,
                    "cursor belongs to a different query",
                ));
            }
            cursor.offset as usize
        } else {
            0
        };
        let d = self.snapshot.document();
        // Physical positions, rather than the count of matching clips, allow
        // range-filtered pagination to resume without rescanning earlier rows.
        let (length, mut rows): (
            usize,
            Box<dyn Iterator<Item = Result<Option<ProjectItem>>> + '_>,
        ) = match request.query {
            ListQuery::Sequences => (
                self.sequences.len(),
                Box::new(self.sequences.iter().skip(start).map(|id| {
                    Ok(Some(ProjectItem::Sequence {
                        id: id.clone(),
                        track_count: d.sequences[id].tracks.len() as u32,
                    }))
                })),
            ),
            ListQuery::Resources => (
                self.resources.len(),
                Box::new(self.resources.iter().skip(start).map(|id| {
                    Ok(Some(ProjectItem::Resource {
                        resource: d.resources[id].clone(),
                    }))
                })),
            ),
            ListQuery::LinkGroups { clip } => (
                self.links.len(),
                Box::new(self.links.iter().skip(start).map(move |id| {
                    let group = &d.link_groups[id];
                    Ok(clip
                        .as_ref()
                        .is_none_or(|id| group.members.contains(id))
                        .then(|| ProjectItem::LinkGroup {
                            group: group.clone(),
                        }))
                })),
            ),
            ListQuery::Tracks { ref sequence } => {
                let s = d
                    .sequences
                    .get(sequence)
                    .ok_or_else(|| Failure::new(FailureCode::NotFound, "sequence not found"))?;
                (
                    s.tracks.len(),
                    Box::new(
                        s.tracks
                            .iter()
                            .skip(start)
                            .map(|id| Ok(Some(track(&d.tracks[id])))),
                    ),
                )
            }
            ListQuery::Clips { ref track, range } => {
                if range.is_some_and(|r| !r.valid()) {
                    return Err(Failure::new(
                        FailureCode::InputInvalid,
                        "invalid list time range",
                    ));
                }
                let t = d
                    .tracks
                    .get(track)
                    .ok_or_else(|| Failure::new(FailureCode::NotFound, "track not found"))?;
                (
                    t.clips.len(),
                    Box::new(t.clips.iter().skip(start).map(move |id| {
                        let clip: &Clip = &d.clips[id];
                        Ok(range
                            .is_none_or(|r| r.intersects(clip.timeline))
                            .then(|| ProjectItem::Clip { clip: clip.clone() }))
                    })),
                )
            }
        };
        if start > length {
            return Err(Failure::new(
                FailureCode::InputInvalid,
                "cursor outside query",
            ));
        }
        let mut items = Vec::new();
        let mut offset = start;
        let mut remaining = self
            .limits
            .query
            .max_output_bytes
            .min(self.limits.max_output_bytes);
        for (visits, row) in (&mut rows).enumerate() {
            cancel(check)?;
            if visits >= self.limits.query.max_visits {
                return Err(Failure::new(
                    FailureCode::LimitExceeded,
                    "list visit budget",
                ));
            }
            offset += 1;
            if let Some(item) = row? {
                // Charge the cumulative page before retaining the next item.
                // The transient item is bounded by validated document size.
                let bytes = check_json_size(&item, remaining, check)?;
                remaining = remaining.saturating_sub(bytes);
                items.push(item);
                if items.len() == request.limit as usize {
                    break;
                }
            }
        }
        let result = ProjectPage {
            document_digest: self.digest().clone(),
            items,
            next: (offset < length).then(|| PageCursor {
                document_digest: self.digest().clone(),
                query_digest,
                offset: offset as u32,
            }),
        };
        check_json_size(
            &result,
            self.limits
                .query
                .max_output_bytes
                .min(self.limits.max_output_bytes),
            check,
        )?;
        Ok(result)
    }
}
fn track(t: &Track) -> ProjectItem {
    ProjectItem::Track {
        id: t.id.clone(),
        name: t.name.clone(),
        track_kind: t.kind,
        locked: t.locked,
        muted: t.muted,
        solo: t.solo,
        clip_count: t.clips.len() as u32,
    }
}
