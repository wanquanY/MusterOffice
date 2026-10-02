use crate::{OperationEntry, Transaction, apply};
use mo_common::*;
use mo_presentation_model::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Snapshot {
    document: Document,
    revision: Digest,
    semantic_digest: Digest,
}

/// Storage envelope owned by an authorized host. A digest is not an access token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnapshotRecord {
    pub document: Document,
    pub revision: Digest,
    pub semantic_digest: Digest,
}

impl Snapshot {
    /// Verifies content against its stored digest; the host separately verifies revision
    /// provenance, tenant scope, and the history/CAS owner before supplying this record.
    pub fn restore(record: SnapshotRecord, limits: ValidationLimits) -> Result<Self, EditError> {
        ensure_valid(&record.document, limits)?;
        if record.document.semantic_digest()? != record.semantic_digest {
            return Err(EditError::input(
                "stored semantic digest does not match document",
            ));
        }
        Ok(Self {
            document: record.document,
            revision: record.revision,
            semantic_digest: record.semantic_digest,
        })
    }
    pub fn into_record(self) -> SnapshotRecord {
        SnapshotRecord {
            document: self.document,
            revision: self.revision,
            semantic_digest: self.semantic_digest,
        }
    }
    pub fn new(document: Document, limits: ValidationLimits) -> Result<Self, EditError> {
        ensure_valid(&document, limits)?;
        let semantic_digest = document.semantic_digest()?;
        let revision = digest("musteroffice.revision.genesis/1", &semantic_digest)?;
        Ok(Self {
            document,
            revision,
            semantic_digest,
        })
    }
    pub fn document(&self) -> &Document {
        &self.document
    }
    pub fn revision(&self) -> &Digest {
        &self.revision
    }
    pub fn semantic_digest(&self) -> &Digest {
        &self.semantic_digest
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChangeSet {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed_timelines: Vec<SlideId>,
    pub created_objects: Vec<ObjectId>,
    pub updated_objects: Vec<ObjectId>,
    pub deleted_objects: Vec<ObjectId>,
    pub created_slides: Vec<SlideId>,
    pub updated_slides: Vec<SlideId>,
    pub deleted_slides: Vec<SlideId>,
    pub changed_themes: Vec<ThemeId>,
    pub changed_masters: Vec<MasterId>,
    pub changed_layouts: Vec<LayoutId>,
    pub changed_fonts: Vec<FontId>,
    pub changed_resources: Vec<ResourceId>,
    pub slide_order_changed: bool,
    pub metadata_changed: bool,
    /// Surviving/new pages whose static layout dependency closure changed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub invalidated_slides: Vec<SlideId>,
    /// Global page geometry changed. Otherwise use invalidated_slides; metadata
    /// and timeline edits have separate change fields and do not dirty layout.
    pub invalidate_all_layout: bool,
    pub anchor_maps: Vec<AnchorMap>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnchorMap {
    pub paragraph: ParagraphId,
    pub start: u32,
    pub deleted: u32,
    pub inserted: u32,
}

impl AnchorMap {
    pub fn map(&self, anchor: &TextAnchor) -> Result<TextAnchor, EditError> {
        if anchor.paragraph != self.paragraph {
            return Ok(anchor.clone());
        }
        let end = self
            .start
            .checked_add(self.deleted)
            .ok_or_else(|| EditError::input("anchor span overflow"))?;
        let after = self
            .start
            .checked_add(self.inserted)
            .ok_or_else(|| EditError::input("anchor insertion overflow"))?;
        let offset = anchor.scalar_offset;
        let scalar_offset = if offset < self.start {
            offset
        } else if offset > end {
            offset
                .checked_sub(self.deleted)
                .and_then(|v| v.checked_add(self.inserted))
                .ok_or_else(|| EditError::input("mapped anchor overflow"))?
        } else if offset == end && self.deleted > 0 {
            after
        } else {
            match anchor.affinity {
                Affinity::Before => self.start,
                Affinity::After => after,
            }
        };
        Ok(TextAnchor {
            paragraph: anchor.paragraph.clone(),
            scalar_offset,
            affinity: anchor.affinity,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransactionReceipt {
    pub document_id: DocumentId,
    pub request_id: RequestId,
    pub request_digest: Digest,
    pub base_revision: Digest,
    pub revision: Digest,
    pub semantic_digest: Digest,
    pub changes: ChangeSet,
}

#[derive(Debug, Clone)]
pub struct PreparedTransaction {
    pub snapshot: Snapshot,
    pub receipt: TransactionReceipt,
}

#[derive(Debug, Error)]
pub enum EditError {
    #[error("transaction preparation cancelled")]
    Cancelled,
    #[error("INPUT_INVALID: {0}")]
    InputInvalid(String),
    #[error("REVISION_CONFLICT: current revision is {current}")]
    RevisionConflict { current: Digest },
    #[error("REQUEST_ID_REUSED")]
    RequestIdReused,
    #[error("REFERENCE_CONFLICT: {0}")]
    ReferenceConflict(String),
    #[error("operation {operation_id} failed: {source}")]
    Operation {
        operation_id: OperationId,
        source: Box<EditError>,
    },
    #[error("document failed semantic validation")]
    InvalidDocument(ValidationReport),
    #[error(transparent)]
    Canonical(#[from] CanonicalError),
}

impl EditError {
    pub(crate) fn input(message: impl Into<String>) -> Self {
        Self::InputInvalid(message.into())
    }
}

fn ensure_valid(document: &Document, limits: ValidationLimits) -> Result<(), EditError> {
    let report = validate(document, limits);
    if report.is_valid() {
        Ok(())
    } else {
        Err(EditError::InvalidDocument(report))
    }
}

pub fn request_digest(transaction: &Transaction) -> Result<Digest, EditError> {
    Ok(digest("musteroffice.transaction/0.1-draft", transaction)?)
}

/// Call before revision checking with a receipt retrieved from the host's authorized
/// (documentId, requestId) scope. Host must commit this receipt and revision atomically.
pub fn check_replay(
    transaction: &Transaction,
    stored: &TransactionReceipt,
) -> Result<(), EditError> {
    if stored.document_id != transaction.document_id || stored.request_id != transaction.request_id
    {
        return Err(EditError::input(
            "receipt belongs to another idempotency scope",
        ));
    }
    if request_digest(transaction)? != stored.request_digest {
        return Err(EditError::RequestIdReused);
    }
    Ok(())
}

/// Computes an unpublished candidate. Failure never mutates the input snapshot.
/// Success still requires the host's CAS(baseRevision) and durable receipt transaction.
pub fn prepare(
    snapshot: &Snapshot,
    transaction: &Transaction,
    limits: ValidationLimits,
) -> Result<PreparedTransaction, EditError> {
    prepare_cancellable(snapshot, transaction, limits, &|| false)
}

/// Same atomic computation with explicit host cancellation between operations
/// and validation/digest phases. The caller still owns strong worker termination.
pub fn prepare_cancellable(
    snapshot: &Snapshot,
    transaction: &Transaction,
    limits: ValidationLimits,
    check: &dyn Fn() -> bool,
) -> Result<PreparedTransaction, EditError> {
    let cancelled = || {
        if check() {
            Err(EditError::Cancelled)
        } else {
            Ok(())
        }
    };
    cancelled()?;
    if transaction.document_id != snapshot.document.id {
        return Err(EditError::input(
            "document identity differs from transaction scope",
        ));
    }
    if &transaction.base_revision != snapshot.revision() {
        return Err(EditError::RevisionConflict {
            current: snapshot.revision.clone(),
        });
    }
    if transaction.operations.is_empty() || transaction.operations.len() > 10_000 {
        return Err(EditError::input(
            "transaction must contain 1..=10000 operations",
        ));
    }
    let mut ids = BTreeSet::new();
    for entry in &transaction.operations {
        cancelled()?;
        if !ids.insert(&entry.operation_id) {
            return Err(EditError::input("operation identity is duplicated"));
        }
    }
    let request_digest = request_digest(transaction)?;
    cancelled()?;
    let mut document = snapshot.document.clone();
    let mut anchor_maps = Vec::new();
    for OperationEntry {
        operation_id,
        operation,
    } in &transaction.operations
    {
        cancelled()?;
        apply::apply(&mut document, operation, &mut anchor_maps, limits, check).map_err(
            |source| EditError::Operation {
                operation_id: operation_id.clone(),
                source: Box::new(source),
            },
        )?;
    }
    finish_preparation(
        snapshot,
        document,
        transaction.request_id.clone(),
        request_digest,
        anchor_maps,
        limits,
        check,
    )
}

/// All edit paths share validation, revision derivation and invalidation rules.
pub(crate) fn finish_preparation(
    snapshot: &Snapshot,
    document: Document,
    request_id: RequestId,
    request_digest: Digest,
    anchor_maps: Vec<AnchorMap>,
    limits: ValidationLimits,
    check: &dyn Fn() -> bool,
) -> Result<PreparedTransaction, EditError> {
    let cancelled = || {
        if check() {
            Err(EditError::Cancelled)
        } else {
            Ok(())
        }
    };
    cancelled()?;
    ensure_valid(&document, limits)?;
    cancelled()?;
    let semantic_digest = document.semantic_digest()?;
    let revision = digest(
        "musteroffice.revision/1",
        &(&snapshot.revision, &request_digest, &semantic_digest),
    )?;
    let changes = changes(&snapshot.document, &document, anchor_maps, check)?;
    cancelled()?;
    let receipt = TransactionReceipt {
        document_id: document.id.clone(),
        request_id,
        request_digest,
        base_revision: snapshot.revision.clone(),
        revision: revision.clone(),
        semantic_digest: semantic_digest.clone(),
        changes,
    };
    Ok(PreparedTransaction {
        snapshot: Snapshot {
            document,
            revision,
            semantic_digest,
        },
        receipt,
    })
}

fn map_diff<K: Ord + Clone, V: PartialEq>(
    before: &std::collections::BTreeMap<K, V>,
    after: &std::collections::BTreeMap<K, V>,
) -> (Vec<K>, Vec<K>, Vec<K>) {
    let created = after
        .keys()
        .filter(|k| !before.contains_key(*k))
        .cloned()
        .collect();
    let updated = after
        .iter()
        .filter(|(k, v)| before.get(*k).is_some_and(|old| old != *v))
        .map(|(k, _)| k.clone())
        .collect();
    let deleted = before
        .keys()
        .filter(|k| !after.contains_key(*k))
        .cloned()
        .collect();
    (created, updated, deleted)
}

fn changed_keys<K: Ord + Clone, V: PartialEq>(
    before: &std::collections::BTreeMap<K, V>,
    after: &std::collections::BTreeMap<K, V>,
) -> Vec<K> {
    let (a, b, c) = map_diff(before, after);
    a.into_iter()
        .chain(b)
        .chain(c)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn changes(
    before: &Document,
    after: &Document,
    anchor_maps: Vec<AnchorMap>,
    check: &dyn Fn() -> bool,
) -> Result<ChangeSet, EditError> {
    let (created_objects, updated_objects, deleted_objects) =
        map_diff(&before.objects, &after.objects);
    let (created_slides, updated_slides, deleted_slides) = map_diff(&before.slides, &after.slides);
    let mut invalidated_slides = Vec::new();
    for id in &after.slide_order {
        if check() {
            return Err(EditError::Cancelled);
        }
        let next = PageDependencies::new(after, id)
            .ok_or_else(|| EditError::input("page dependency reference"))?;
        let previous = PageDependencies::new(before, id);
        if previous.as_ref() != Some(&next) {
            invalidated_slides.push(id.clone());
        }
    }

    Ok(ChangeSet {
        changed_timelines: changed_keys(&before.timelines, &after.timelines),
        created_objects,
        updated_objects,
        deleted_objects,
        created_slides,
        updated_slides,
        deleted_slides,
        changed_themes: changed_keys(&before.themes, &after.themes),
        changed_masters: changed_keys(&before.masters, &after.masters),
        changed_layouts: changed_keys(&before.layouts, &after.layouts),
        changed_fonts: changed_keys(&before.fonts, &after.fonts),
        changed_resources: changed_keys(&before.resources, &after.resources),
        slide_order_changed: before.slide_order != after.slide_order,
        metadata_changed: before.title != after.title,
        invalidated_slides,
        invalidate_all_layout: before.page_size != after.page_size,
        anchor_maps,
    })
}
