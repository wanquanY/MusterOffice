use crate::*;
use mo_common::{JsonDecodeError, RationalTime, check_json_size, from_json_str_with_path};
use mo_time::ExactRatio;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy)]
pub struct ValidationLimits {
    pub max_nodes: usize,
    pub max_json_bytes: usize,
    pub max_depth: usize,
    pub max_issues: usize,
    pub max_streams: usize,
    pub max_time_anchors: usize,
    pub exact_bits: u64,
}
impl Default for ValidationLimits {
    fn default() -> Self {
        Self {
            max_nodes: 100_000,
            max_json_bytes: 32 * 1024 * 1024,
            max_depth: 64,
            max_issues: 128,
            max_streams: 4096,
            max_time_anchors: 100_000,
            exact_bits: 256,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ValidationCode {
    InvalidValue,
    MissingReference,
    DuplicateIdentity,
    OwnershipConflict,
    ReferenceCycle,
    LimitExceeded,
    IdentityMismatch,
    IncompatibleTrack,
    Overlap,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValidationIssue {
    pub code: ValidationCode,
    pub path: String,
    pub message: String,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValidationReport {
    pub issues: Vec<ValidationIssue>,
    pub truncated: bool,
}
impl ValidationReport {
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty() && !self.truncated
    }
}
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("invalid validation limits")]
    Limits,
    #[error("video validation cancelled")]
    Cancelled,
    #[error("video document exceeds admission budget")]
    Budget,
    #[error(transparent)]
    Json(#[from] JsonDecodeError),
}
/// Rejects duplicate JSON keys before map deserialization; no arbitrary extension values.
pub fn decode(
    input: &str,
    limits: ValidationLimits,
    check: &dyn Fn() -> bool,
) -> Result<(VideoDocument, ValidationReport), ValidationError> {
    if input.len() > limits.max_json_bytes {
        return Err(ValidationError::Budget);
    }
    if check() {
        return Err(ValidationError::Cancelled);
    }
    let doc = from_json_str_with_path(input)?;
    let report = validate(&doc, limits, check)?;
    Ok((doc, report))
}
pub fn validate(
    doc: &VideoDocument,
    limits: ValidationLimits,
    check: &dyn Fn() -> bool,
) -> Result<ValidationReport, ValidationError> {
    if limits.max_depth == 0
        || limits.max_depth > 128
        || limits.max_issues == 0
        || limits.max_issues > 1024
        || !(128..=4096).contains(&limits.exact_bits)
    {
        return Err(ValidationError::Limits);
    }
    check_json_size(doc, limits.max_json_bytes, check).map_err(|e| match e {
        mo_common::JsonBudgetError::Cancelled => ValidationError::Cancelled,
        _ => ValidationError::Budget,
    })?;
    let count = [
        doc.sequences.len(),
        doc.tracks.len(),
        doc.clips.len(),
        doc.resources.len(),
        doc.link_groups.len(),
    ]
    .into_iter()
    .try_fold(0usize, usize::checked_add)
    .ok_or(ValidationError::Budget)?;
    if count > limits.max_nodes {
        return Err(ValidationError::Budget);
    }
    Validator {
        doc,
        limits,
        check,
        report: ValidationReport::default(),
        track_owners: BTreeMap::new(),
        clip_owners: BTreeMap::new(),
        anchors: 0,
        streams: 0,
        sequence_ends: BTreeMap::new(),
    }
    .run()
}
struct Validator<'a> {
    doc: &'a VideoDocument,
    limits: ValidationLimits,
    check: &'a dyn Fn() -> bool,
    report: ValidationReport,
    track_owners: BTreeMap<TrackId, SequenceId>,
    clip_owners: BTreeMap<ClipId, TrackId>,
    anchors: usize,
    streams: usize,
    sequence_ends: BTreeMap<SequenceId, RationalTime>,
}
impl Validator<'_> {
    fn poll(&self) -> Result<(), ValidationError> {
        if (self.check)() {
            Err(ValidationError::Cancelled)
        } else {
            Ok(())
        }
    }
    fn issue(&mut self, code: ValidationCode, path: impl Into<String>, message: impl Into<String>) {
        if self.report.issues.len() >= self.limits.max_issues {
            self.report.truncated = true;
            return;
        }
        self.report.issues.push(ValidationIssue {
            code,
            path: path.into(),
            message: message.into(),
        });
    }
    fn require(&mut self, valid: bool, path: &str, message: &str) {
        if !valid {
            self.issue(ValidationCode::InvalidValue, path, message);
        }
    }
    fn reference(&mut self, exists: bool, path: &str) {
        if !exists {
            self.issue(
                ValidationCode::MissingReference,
                path,
                "referenced object is missing",
            );
        }
    }
    fn canvas(&mut self, c: Canvas, path: &str) {
        self.require(
            c.width > 0 && c.height > 0 && c.width <= 32768 && c.height <= 32768,
            path,
            "dimensions must be in 1..=32768",
        );
    }
    fn run(mut self) -> Result<ValidationReport, ValidationError> {
        let d = self.doc;
        self.canvas(d.canvas, "/canvas");
        self.require(
            d.frame_rate.valid(),
            "/frameRate",
            "invalid frame-rate ratio",
        );
        self.require(
            (8000..=384000).contains(&d.audio_sample_rate),
            "/audioSampleRate",
            "sample rate outside supported document bounds",
        );
        self.reference(d.sequences.contains_key(&d.root_sequence), "/rootSequence");
        for (id, seq) in &d.sequences {
            self.poll()?;
            let path = format!("/sequences/{id}");
            if id != &seq.id {
                self.issue(
                    ValidationCode::IdentityMismatch,
                    &path,
                    "sequence map key differs from id",
                );
            }
            for track in &seq.tracks {
                self.poll()?;
                self.reference(d.tracks.contains_key(track), &path);
                if self
                    .track_owners
                    .insert(track.clone(), id.clone())
                    .is_some()
                {
                    self.issue(
                        ValidationCode::OwnershipConflict,
                        &path,
                        "track listed more than once",
                    );
                }
            }
        }
        for (id, track) in &d.tracks {
            self.poll()?;
            let path = format!("/tracks/{id}");
            if id != &track.id {
                self.issue(
                    ValidationCode::IdentityMismatch,
                    &path,
                    "track map key differs from id",
                );
            }
            if !self.track_owners.contains_key(id) {
                self.issue(
                    ValidationCode::OwnershipConflict,
                    &path,
                    "track has no sequence owner",
                );
            }
            let mut ranges = Vec::new();
            for clip in &track.clips {
                self.poll()?;
                self.reference(d.clips.contains_key(clip), &path);
                if self.clip_owners.insert(clip.clone(), id.clone()).is_some() {
                    self.issue(
                        ValidationCode::OwnershipConflict,
                        &path,
                        "clip listed more than once",
                    );
                }
                if let Some(clip) = d.clips.get(clip) {
                    ranges.push(clip);
                    if let Some(sequence) = self.track_owners.get(id) {
                        let end = self
                            .sequence_ends
                            .entry(sequence.clone())
                            .or_insert(clip.timeline.end);
                        if end.compare_time(clip.timeline.end).is_lt() {
                            *end = clip.timeline.end;
                        }
                    }
                }
            }
            if track.kind == TrackKind::Video {
                ranges.sort_by(|a, b| {
                    a.timeline
                        .start
                        .compare_time(b.timeline.start)
                        .then_with(|| a.id.cmp(&b.id))
                });
                for pair in ranges.windows(2) {
                    if pair[0].timeline.intersects(pair[1].timeline) {
                        self.issue(
                            ValidationCode::Overlap,
                            &path,
                            "base video clips overlap without an explicit transition",
                        );
                    }
                }
            }
        }
        for (id, resource) in &d.resources {
            self.poll()?;
            let path = format!("/resources/{id}");
            if id != &resource.id {
                self.issue(
                    ValidationCode::IdentityMismatch,
                    &path,
                    "resource map key differs from id",
                );
            }
            self.require(
                resource.byte_length.get() > 0,
                &path,
                "resource must describe actual bytes",
            );
            if let ResourceDescriptor::Media { streams } = &resource.descriptor {
                self.streams = self
                    .streams
                    .checked_add(streams.len())
                    .ok_or(ValidationError::Budget)?;
                if self.streams > self.limits.max_streams {
                    return Err(ValidationError::Budget);
                }
                self.require(!streams.is_empty(), &path, "media must contain a stream");
                for (sid, stream) in streams {
                    let path = format!("{path}/descriptor/streams/{sid}");
                    if let Some(range) = stream.available() {
                        self.require(range.valid(), &path, "invalid source availability range");
                    }
                    match stream {
                        MediaStream::Video {
                            dimensions,
                            frame_rate,
                            ..
                        } => {
                            self.canvas(*dimensions, &path);
                            self.require(
                                frame_rate.is_none_or(FrameRate::valid),
                                &path,
                                "invalid nominal frame rate",
                            );
                        }
                        MediaStream::Audio {
                            sample_rate,
                            channels,
                            ..
                        } => self.require(
                            (8000..=384000).contains(sample_rate) && (1..=64).contains(channels),
                            &path,
                            "invalid audio stream",
                        ),
                        MediaStream::Image { dimensions } => self.canvas(*dimensions, &path),
                    }
                }
            }
        }
        for (id, clip) in &d.clips {
            self.poll()?;
            let path = format!("/clips/{id}");
            if id != &clip.id {
                self.issue(
                    ValidationCode::IdentityMismatch,
                    &path,
                    "clip map key differs from id",
                );
            }
            self.require(
                clip.timeline.valid() && clip.timeline.start.ticks.get() >= 0,
                &path,
                "invalid nonnegative timeline placement",
            );
            let owner = self
                .clip_owners
                .get(id)
                .and_then(|t| d.tracks.get(t))
                .map(|t| t.kind);
            if owner.is_none() {
                self.issue(
                    ValidationCode::OwnershipConflict,
                    &path,
                    "clip has no track owner",
                );
            }
            match &clip.source {
                ClipSource::Media {
                    resource,
                    stream,
                    time_map,
                } => {
                    self.reference(d.resources.contains_key(resource), &path);
                    let media = d.resources.get(resource).and_then(|r| match &r.descriptor {
                        ResourceDescriptor::Media { streams } => streams.get(stream),
                        _ => None,
                    });
                    self.reference(media.is_some(), &path);
                    if let (Some(media), Some(kind)) = (media, owner)
                        && !media.compatible(kind)
                    {
                        self.issue(
                            ValidationCode::IncompatibleTrack,
                            &path,
                            "stream does not match track kind",
                        );
                    }
                    self.time_map(
                        time_map,
                        clip.timeline,
                        media.and_then(MediaStream::available),
                        &path,
                    )?;
                    if matches!(media, Some(MediaStream::Image { .. })) {
                        self.require(
                            time_map.anchors.iter().all(|a| a.source.ticks.get() == 0),
                            &path,
                            "still images use fixed source time zero",
                        );
                    }
                }
                ClipSource::Sequence { sequence, time_map } => {
                    self.reference(d.sequences.contains_key(sequence), &path);
                    let end = self.sequence_end(sequence);
                    let available = end.map(|end| TimeRange { start: zero(), end });
                    self.require(
                        available.is_some_and(TimeRange::valid),
                        &path,
                        "nested sequence has no playable duration",
                    );
                    self.time_map(time_map, clip.timeline, available, &path)?;
                    self.require(
                        matches!(owner, Some(TrackKind::Video | TrackKind::Audio)),
                        &path,
                        "nested composition requires a video or audio track",
                    );
                }
                ClipSource::Solid { .. } => self.require(
                    matches!(owner, Some(TrackKind::Video | TrackKind::Graphics)),
                    &path,
                    "solid requires a visual track",
                ),
                ClipSource::Text {
                    text,
                    font,
                    font_size_milli_pixels,
                    ..
                } => {
                    self.require(
                        matches!(owner, Some(TrackKind::Captions | TrackKind::Graphics)),
                        &path,
                        "text requires a caption or graphics track",
                    );
                    self.require(
                        !text.is_empty() && (1..=1_000_000).contains(font_size_milli_pixels),
                        &path,
                        "invalid text or font size",
                    );
                    self.reference(
                        d.resources.get(font).is_some_and(|r| {
                            matches!(r.descriptor, ResourceDescriptor::Font { .. })
                        }),
                        &path,
                    );
                }
            }
            self.effects(clip, owner, &path);
        }
        let mut linked = BTreeSet::new();
        for (id, group) in &d.link_groups {
            self.poll()?;
            let path = format!("/linkGroups/{id}");
            if id != &group.id {
                self.issue(
                    ValidationCode::IdentityMismatch,
                    &path,
                    "link-group key differs from id",
                );
            }
            self.require(
                group.members.len() >= 2,
                &path,
                "link group requires at least two clips",
            );
            let mut sequence = None;
            for clip in &group.members {
                self.poll()?;
                self.reference(d.clips.contains_key(clip), &path);
                if let Some(owner) = self
                    .clip_owners
                    .get(clip)
                    .and_then(|t| self.track_owners.get(t))
                    .cloned()
                {
                    self.require(
                        sequence.as_ref().is_none_or(|s| s == &owner),
                        &path,
                        "linked clips must share a sequence time coordinate",
                    );
                    sequence = Some(owner);
                }
                if !linked.insert(clip) {
                    self.issue(
                        ValidationCode::OwnershipConflict,
                        &path,
                        "clip belongs to multiple or repeated link members",
                    );
                }
            }
        }
        self.sequences()?;
        self.poll()?;
        Ok(self.report)
    }
    fn sequence_end(&self, id: &SequenceId) -> Option<RationalTime> {
        self.sequence_ends.get(id).copied()
    }
    fn time_map(
        &mut self,
        map: &TimeMap,
        timeline: TimeRange,
        available: Option<TimeRange>,
        path: &str,
    ) -> Result<(), ValidationError> {
        self.anchors = self
            .anchors
            .checked_add(map.anchors.len())
            .ok_or(ValidationError::Budget)?;
        if self.anchors > self.limits.max_time_anchors {
            return Err(ValidationError::Budget);
        }
        self.require(
            map.anchors.len() >= 2,
            path,
            "time map requires at least two anchors",
        );
        self.require(
            map.anchors
                .first()
                .is_some_and(|a| a.local.ticks.get() == 0),
            path,
            "time map must start at local zero",
        );
        let duration = timeline.duration(self.limits.exact_bits);
        self.require(
            map.anchors.last().is_some_and(|a| {
                duration
                    .as_ref()
                    .is_ok_and(|d| d.compare(&ExactRatio::from_time(a.local)).is_eq())
            }),
            path,
            "time map must cover the exact clip duration",
        );
        for a in &map.anchors {
            self.poll()?;
            if let Some(range) = available {
                self.require(
                    range.contains_boundary(a.source),
                    path,
                    "time-map source anchor outside availability",
                );
            }
        }
        for pair in map.anchors.windows(2) {
            self.require(
                pair[0].local.compare_time(pair[1].local).is_lt(),
                path,
                "local anchors must increase strictly",
            );
            if pair[0].source.compare_time(pair[1].source).is_eq()
                && let Some(range) = available
            {
                self.require(
                    range.contains(pair[0].source),
                    path,
                    "hold cannot sample the exclusive source end",
                );
            }
        }
        Ok(())
    }
    fn effects(&mut self, clip: &Clip, owner: Option<TrackKind>, path: &str) {
        for effect in &clip.effects {
            match effect {
                Effect::Transform {
                    scale_x_ppm,
                    scale_y_ppm,
                    opacity_ppm,
                    ..
                } => self.require(
                    owner != Some(TrackKind::Audio)
                        && *scale_x_ppm != 0
                        && *scale_y_ppm != 0
                        && *opacity_ppm <= 1_000_000,
                    path,
                    "invalid visual transform",
                ),
                Effect::Gain { milli_decibels } => self.require(
                    owner == Some(TrackKind::Audio) && (-120_000..=24_000).contains(milli_decibels),
                    path,
                    "invalid gain or non-audio track",
                ),
                Effect::Fade {
                    fade_in,
                    fade_out,
                    envelope,
                } => {
                    let duration = envelope.duration(self.limits.exact_bits);
                    let sum = ExactRatio::from_time(*fade_in)
                        .add(&ExactRatio::from_time(*fade_out), self.limits.exact_bits);
                    self.require(
                        envelope.valid()
                            && fade_in.ticks.get() >= 0
                            && fade_out.ticks.get() >= 0
                            && sum.as_ref().is_ok_and(|s| {
                                duration.as_ref().is_ok_and(|d| s.compare(d).is_le())
                            }),
                        path,
                        "fades exceed the envelope duration",
                    );
                }
            }
        }
    }
    fn sequences(&mut self) -> Result<(), ValidationError> {
        // Post-order DAG traversal: memoized heights also check depth through shared subgraphs.
        let mut graph: BTreeMap<SequenceId, Vec<SequenceId>> = self
            .doc
            .sequences
            .keys()
            .map(|id| (id.clone(), vec![]))
            .collect();
        for (id, clip) in &self.doc.clips {
            if let ClipSource::Sequence { sequence, .. } = &clip.source
                && let Some(owner) = self
                    .clip_owners
                    .get(id)
                    .and_then(|t| self.track_owners.get(t))
                && self.doc.sequences.contains_key(sequence)
            {
                graph
                    .get_mut(owner)
                    .expect("known owner")
                    .push(sequence.clone());
            }
        }
        let mut state = BTreeMap::new();
        let mut heights: BTreeMap<SequenceId, usize> = BTreeMap::new();
        for root in graph.keys() {
            if state.get(root) == Some(&2) {
                continue;
            }
            let mut stack = vec![(root.clone(), 0usize)];
            state.insert(root.clone(), 1);
            while let Some((id, index)) = stack.last_mut() {
                self.poll()?;
                let children = &graph[id];
                if *index < children.len() {
                    let child = children[*index].clone();
                    *index += 1;
                    match state.get(&child) {
                        Some(1) => {
                            self.issue(
                                ValidationCode::ReferenceCycle,
                                "/sequences",
                                "nested sequence cycle",
                            );
                            return Ok(());
                        }
                        Some(2) => {}
                        _ => {
                            state.insert(child.clone(), 1);
                            stack.push((child, 0));
                            if stack.len() > self.limits.max_depth {
                                self.issue(
                                    ValidationCode::LimitExceeded,
                                    "/sequences",
                                    "nested sequence depth limit",
                                );
                                return Ok(());
                            }
                        }
                    }
                } else {
                    let height = 1 + children.iter().map(|c| heights[c]).max().unwrap_or(0);
                    if height > self.limits.max_depth {
                        self.issue(
                            ValidationCode::LimitExceeded,
                            "/sequences",
                            "nested sequence depth limit",
                        );
                        return Ok(());
                    }
                    heights.insert(id.clone(), height);
                    state.insert(id.clone(), 2);
                    stack.pop();
                }
            }
        }
        Ok(())
    }
}
fn zero() -> RationalTime {
    RationalTime::new(0, 1).expect("zero")
}
