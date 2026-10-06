//! Video authoring computation shared by native hosts and standalone WASM.
//! No durable commits, product jobs, asset I/O, model providers or PPT dependency.
mod contract;
mod failure;
mod query;
pub use contract::*;
pub use failure::*;
use mo_common::{Digest, Ticks, check_json_size};
use mo_video_compile::{CompileLimits, CompiledDocument, QueryLimits, VideoFramePlan};
use mo_video_edit::{EditLimits, Snapshot};
pub use mo_video_edit::{Inverse, Transaction};
use mo_video_model::VideoDocument;
use serde::{Serialize, de::DeserializeOwned};
pub const PROFILE: &str = "musteroffice.video.operations/1-draft";
pub type Result<T> = std::result::Result<T, Failure>;
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub edit: EditLimits,
    pub query: QueryLimits,
    pub max_snapshot_bytes: usize,
    pub max_output_bytes: usize,
    pub max_page_items: u16,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            edit: EditLimits::default(),
            query: QueryLimits::default(),
            max_snapshot_bytes: 34 << 20,
            max_output_bytes: 64 << 20,
            max_page_items: 256,
        }
    }
}
/// Immutable content handle; a new candidate never changes this snapshot.
/// Compiled indexes are retained for repeated frame and summary queries.
pub struct VideoProject {
    snapshot: Snapshot,
    compiled: CompiledDocument,
    limits: Limits,
    sequences: Vec<mo_video_model::SequenceId>,
    resources: Vec<mo_common::ResourceId>,
    links: Vec<mo_video_model::LinkGroupId>,
}
impl VideoProject {
    pub fn create(input: CreateProject, limits: Limits, check: &dyn Fn() -> bool) -> Result<Self> {
        let mut document = VideoDocument::empty(
            input.document_id,
            input.root_sequence,
            input.canvas,
            input.frame_rate,
        );
        document.title = input.title;
        document.audio_sample_rate = input.audio_sample_rate;
        Self::from_document(document, limits, check)
    }
    pub fn open(record: VideoSnapshot, limits: Limits, check: &dyn Fn() -> bool) -> Result<Self> {
        check_json_size(&record, limits.max_snapshot_bytes, check)?;
        let project = Self::from_document(record.document, limits, check)?;
        if project.digest() != &record.document_digest {
            return Err(Failure::new(
                FailureCode::SnapshotCorrupt,
                "snapshot content digest mismatch",
            ));
        }
        Ok(project)
    }
    fn from_document(
        document: VideoDocument,
        limits: Limits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self> {
        if limits.max_page_items == 0
            || limits.max_page_items > 1024
            || limits.max_output_bytes == 0
            || limits.max_snapshot_bytes == 0
        {
            return Err(Failure::new(
                FailureCode::InputInvalid,
                "invalid computation limits",
            ));
        }
        let snapshot = Snapshot::new(document, limits.edit.validation, check)?;
        check_json_size(&record(&snapshot), limits.max_snapshot_bytes, check)?;
        let compiled = CompiledDocument::new(
            snapshot.document().clone(),
            CompileLimits {
                validation: limits.edit.validation,
            },
            check,
        )?;
        let sequences = snapshot.document().sequences.keys().cloned().collect();
        let resources = snapshot.document().resources.keys().cloned().collect();
        let links = snapshot.document().link_groups.keys().cloned().collect();
        Ok(Self {
            links,
            snapshot,
            compiled,
            limits,
            sequences,
            resources,
        })
    }
    pub fn digest(&self) -> &Digest {
        self.snapshot.digest()
    }
    pub fn compiled(&self) -> &CompiledDocument {
        &self.compiled
    }
    pub fn snapshot(&self) -> VideoSnapshot {
        record(&self.snapshot)
    }
    fn expect(&self, digest: &Digest) -> Result<()> {
        if digest == self.digest() {
            Ok(())
        } else {
            Err(Failure::new(
                FailureCode::StaleSnapshot,
                "expected a different document digest",
            ))
        }
    }
    pub fn summary(&self, check: &dyn Fn() -> bool) -> Result<ProjectSummary> {
        cancel(check)?;
        let d = self.snapshot.document();
        let result = ProjectSummary {
            document_id: d.id.clone(),
            document_digest: self.digest().clone(),
            title: d.title.clone(),
            canvas: d.canvas,
            frame_rate: d.frame_rate,
            audio_sample_rate: d.audio_sample_rate,
            root_sequence: d.root_sequence.clone(),
            duration: self.compiled.duration().clone(),
            frame_count: Ticks::new(self.compiled.frame_count()?),
            audio_sample_count: Ticks::new(self.compiled.audio_sample_count()?),
            sequence_count: d.sequences.len() as u32,
            track_count: d.tracks.len() as u32,
            clip_count: d.clips.len() as u32,
            resource_count: d.resources.len() as u32,
            link_group_count: d.link_groups.len() as u32,
        };
        check_json_size(&result, self.limits.max_output_bytes, check)?;
        Ok(result)
    }
    pub fn prepare(
        &self,
        input: &Transaction,
        check: &dyn Fn() -> bool,
    ) -> Result<MutationCandidate> {
        self.candidate(
            mo_video_edit::prepare(&self.snapshot, input, self.limits.edit, check)?,
            check,
        )
    }
    pub fn prepare_inverse(
        &self,
        input: &InverseRequest,
        check: &dyn Fn() -> bool,
    ) -> Result<MutationCandidate> {
        self.expect(&input.expected_digest)?;
        self.candidate(
            mo_video_edit::prepare_inverse(
                &self.snapshot,
                &input.inverse,
                self.limits.edit,
                check,
            )?,
            check,
        )
    }
    fn candidate(
        &self,
        prepared: mo_video_edit::PreparedEdit,
        check: &dyn Fn() -> bool,
    ) -> Result<MutationCandidate> {
        let value = MutationCandidate {
            base_digest: prepared.base_digest,
            request_digest: prepared.request_digest,
            snapshot: record(&prepared.candidate),
            read_set: prepared.read_set,
            write_set: prepared.write_set,
            inverse: prepared.inverse,
            invalidations: prepared.invalidations,
        };
        check_json_size(&value.snapshot, self.limits.max_snapshot_bytes, check)?;
        check_json_size(&value, self.limits.max_output_bytes, check)?;
        Ok(value)
    }
    pub fn frame(&self, input: FrameRequest, check: &dyn Fn() -> bool) -> Result<VideoFramePlan> {
        self.expect(&input.expected_digest)?;
        let query = QueryLimits {
            max_output_bytes: self
                .limits
                .query
                .max_output_bytes
                .min(self.limits.max_output_bytes),
            ..self.limits.query
        };
        Ok(self.compiled.frame(input.frame.get(), query, check)?.value)
    }
    /// All declared resource identities, not merely the currently visible frames.
    /// Product resolvers must separately prove access to the corresponding bytes.
    pub fn resources(&self, check: &dyn Fn() -> bool) -> Result<ResourceClosure> {
        let mut resources = Vec::new();
        for r in self.snapshot.document().resources.values() {
            cancel(check)?;
            resources.push(ResourceRequirement {
                resource_id: r.id.clone(),
                digest: r.digest.clone(),
                byte_length: r.byte_length,
            });
        }
        let result = ResourceClosure {
            document_digest: self.digest().clone(),
            resources,
        };
        check_json_size(&result, self.limits.max_output_bytes, check)?;
        Ok(result)
    }
}
fn record(snapshot: &Snapshot) -> VideoSnapshot {
    VideoSnapshot {
        format: SnapshotFormat::V1,
        document_digest: snapshot.digest().clone(),
        document: snapshot.document().clone(),
    }
}
pub(crate) fn cancel(check: &dyn Fn() -> bool) -> Result<()> {
    if check() {
        Err(Failure::new(
            FailureCode::Cancelled,
            "computation cancelled",
        ))
    } else {
        Ok(())
    }
}
pub fn decode<T: DeserializeOwned>(
    input: &str,
    max_bytes: usize,
    check: &dyn Fn() -> bool,
) -> Result<T> {
    cancel(check)?;
    if input.len() > max_bytes {
        return Err(Failure::new(
            FailureCode::LimitExceeded,
            "JSON input exceeds configured budget",
        ));
    }
    let value = mo_common::from_json_str_with_path(input).map_err(|_| {
        Failure::new(
            FailureCode::InputInvalid,
            "invalid or unsupported video JSON",
        )
    })?;
    cancel(check)?;
    Ok(value)
}
pub fn encode<T: Serialize>(
    value: &T,
    max_bytes: usize,
    check: &dyn Fn() -> bool,
) -> Result<String> {
    let bytes = check_json_size(value, max_bytes, check)?;
    let mut output = Vec::with_capacity(bytes);
    serde_json::to_writer(&mut output, value).map_err(|_| {
        Failure::new(
            FailureCode::InternalInvariant,
            "output serialization failed",
        )
    })?;
    cancel(check)?;
    String::from_utf8(output)
        .map_err(|_| Failure::new(FailureCode::InternalInvariant, "output encoding failed"))
}
