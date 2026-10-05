//! Pure, atomic video editing. Candidates never mutate their base and are not durable receipts.
mod contract;
mod history;
mod reducer;
mod timing;
pub use contract::*;
use mo_common::{CanonicalError, Digest, check_json_size};
use mo_time::TimeError;
use mo_video_model::*;
use std::collections::BTreeSet;

pub type Result<T> = std::result::Result<T, EditError>;

#[derive(Debug, thiserror::Error)]
pub enum EditError {
    #[error("invalid video document: {0:?}")]
    InvalidDocument(ValidationReport),
    #[error("edit conflicts with current objects: {0:?}")]
    Conflict(Vec<ObjectKey>),
    #[error("expected a different document digest")]
    StaleSnapshot,
    #[error("missing object: {0:?}")]
    Missing(ObjectKey),
    #[error("identity already exists: {0:?}")]
    Duplicate(ObjectKey),
    #[error("track is locked: {0}")]
    Locked(TrackId),
    #[error("linked selection is incomplete: {0}")]
    LinkedSelection(LinkGroupId),
    #[error("invalid edit operation: {0}")]
    InvalidOperation(&'static str),
    #[error("edit exceeds admission budget")]
    Budget,
    #[error("edit cancelled")]
    Cancelled,
    #[error("operation {index}: {source}")]
    Operation {
        index: usize,
        source: Box<EditError>,
    },
    #[error(transparent)]
    Time(#[from] TimeError),
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error(transparent)]
    Canonical(#[from] CanonicalError),
}

#[derive(Debug, Clone, Copy)]
pub struct EditLimits {
    pub validation: ValidationLimits,
    pub max_operations: usize,
    pub max_request_bytes: usize,
    pub max_changed_objects: usize,
}
impl Default for EditLimits {
    fn default() -> Self {
        Self {
            validation: ValidationLimits::default(),
            max_operations: 4096,
            max_request_bytes: 8 * 1024 * 1024,
            max_changed_objects: 100_000,
        }
    }
}

/// A validated content snapshot. Storage revisions, owner epochs and idempotency keys are host data.
#[derive(Debug, Clone)]
pub struct Snapshot {
    document: VideoDocument,
    digest: Digest,
}
impl Snapshot {
    pub fn new(
        document: VideoDocument,
        limits: ValidationLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Self> {
        let report = validate(&document, limits, cancelled)?;
        if !report.is_valid() {
            return Err(EditError::InvalidDocument(report));
        }
        let digest = document.content_digest()?;
        if cancelled() {
            return Err(EditError::Cancelled);
        }
        Ok(Self { document, digest })
    }
    pub fn document(&self) -> &VideoDocument {
        &self.document
    }
    pub fn digest(&self) -> &Digest {
        &self.digest
    }
    pub fn into_document(self) -> VideoDocument {
        self.document
    }
}

#[derive(Debug, Clone)]
pub struct PreparedEdit {
    pub base_digest: Digest,
    pub request_digest: Digest,
    pub candidate: Snapshot,
    pub read_set: Vec<ObjectVersion>,
    pub write_set: Vec<ObjectKey>,
    pub inverse: Inverse,
    pub invalidations: Vec<Invalidation>,
}

fn admit<T: serde::Serialize>(
    value: &T,
    limits: EditLimits,
    cancelled: &dyn Fn() -> bool,
) -> Result<()> {
    if limits.max_operations == 0 || limits.max_changed_objects == 0 {
        return Err(EditError::Budget);
    }
    check_json_size(value, limits.max_request_bytes, cancelled)
        .map(|_| ())
        .map_err(|e| match e {
            mo_common::JsonBudgetError::Cancelled => EditError::Cancelled,
            _ => EditError::Budget,
        })
}

pub fn prepare(
    base: &Snapshot,
    request: &Transaction,
    limits: EditLimits,
    cancelled: &dyn Fn() -> bool,
) -> Result<PreparedEdit> {
    admit(request, limits, cancelled)?;
    if &request.expected_digest != base.digest() {
        return Err(EditError::StaleSnapshot);
    }
    if request.operations.is_empty() || request.operations.len() > limits.max_operations {
        return Err(EditError::Budget);
    }
    let mut state = reducer::State::new(base.document.clone(), limits, cancelled);
    for (index, operation) in request.operations.iter().enumerate() {
        state
            .apply(operation)
            .map_err(|source| EditError::Operation {
                index,
                source: Box::new(source),
            })?;
        state.check_budget()?;
    }
    history::finish(
        base,
        state.document,
        state.reads,
        state.writes,
        mo_common::digest("musteroffice.video.edit/1", request)?,
        limits,
        cancelled,
    )
}

pub fn prepare_inverse(
    base: &Snapshot,
    inverse: &Inverse,
    limits: EditLimits,
    cancelled: &dyn Fn() -> bool,
) -> Result<PreparedEdit> {
    admit(inverse, limits, cancelled)?;
    if inverse.restore.len() > limits.max_changed_objects
        || inverse.conditions.len() > limits.validation.max_nodes.saturating_add(1)
    {
        return Err(EditError::Budget);
    }
    let mut reads = BTreeSet::new();
    let mut conflicts = vec![];
    let fingerprints = history::Fingerprints::new(base.document());
    for condition in &inverse.conditions {
        if cancelled() {
            return Err(EditError::Cancelled);
        }
        if !reads.insert(condition.object.clone()) {
            return Err(EditError::InvalidOperation("duplicate inverse condition"));
        }
        if fingerprints.get(&condition.object)? != condition.digest {
            conflicts.push(condition.object.clone());
        }
    }
    if !conflicts.is_empty() {
        return Err(EditError::Conflict(conflicts));
    }
    let mut doc = base.document.clone();
    let mut writes = BTreeSet::new();
    for restore in &inverse.restore {
        if cancelled() {
            return Err(EditError::Cancelled);
        }
        let key = restore.key();
        if !reads.contains(&key) || !writes.insert(key) {
            return Err(EditError::InvalidOperation(
                "unconditional or repeated inverse write",
            ));
        }
        history::restore(&mut doc, restore);
    }
    history::finish(
        base,
        doc,
        reads,
        writes,
        mo_common::digest("musteroffice.video.inverse/1", inverse)?,
        limits,
        cancelled,
    )
}
