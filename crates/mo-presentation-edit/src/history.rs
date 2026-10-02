//! Guarded semantic history, shared by native and WASM hosts.
//!
//! The host resolves the original transaction and base checkpoint from its
//! authorized durable history. Neither a snapshot digest nor this pure
//! calculation authorizes undoing someone else's transaction. History retains
//! the original checkpoint's resources until the host releases the history.
use crate::{
    AnchorMap, EditError, PreparedTransaction, Snapshot, SnapshotRecord, Transaction,
    TransactionReceipt, prepare_cancellable, transaction::finish_preparation,
};
use mo_common::*;
use mo_presentation_model::ValidationLimits;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum HistoryDirection {
    Undo,
    Redo,
}

/// A new command against the current revision, referring to an original edit.
/// Product APIs should accept a durable history reference and resolve the
/// original material on the server, rather than trusting client checkpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HistoryTransaction {
    pub document_id: DocumentId,
    pub request_id: RequestId,
    pub base_revision: Digest,
    pub direction: HistoryDirection,
    pub original_snapshot: SnapshotRecord,
    pub original_transaction: Transaction,
}

pub fn history_request_digest(request: &HistoryTransaction) -> Result<Digest, EditError> {
    Ok(digest("musteroffice.history-transaction/1", request)?)
}

pub fn check_history_replay(
    request: &HistoryTransaction,
    stored: &TransactionReceipt,
) -> Result<(), EditError> {
    if stored.document_id != request.document_id || stored.request_id != request.request_id {
        return Err(EditError::input("receipt belongs to another history scope"));
    }
    if history_request_digest(request)? != stored.request_digest {
        return Err(EditError::RequestIdReused);
    }
    Ok(())
}

/// Recomputes the original edit and reverses only its changed declarations.
/// Each changed declaration must still match the expected value. Independent
/// edits survive; overlapping edits cause a conflict, never last-writer-wins.
/// The candidate always has a new revision and requires the host's normal CAS.
pub fn prepare_history(
    current: &Snapshot,
    request: &HistoryTransaction,
    limits: ValidationLimits,
    check: &dyn Fn() -> bool,
) -> Result<PreparedTransaction, EditError> {
    cancelled(check)?;
    if current.document().id != request.document_id
        || request.original_snapshot.document.id != request.document_id
    {
        return Err(EditError::input("history document identity differs"));
    }
    if current.revision() != &request.base_revision {
        return Err(EditError::RevisionConflict {
            current: current.revision().clone(),
        });
    }
    let original = Snapshot::restore(request.original_snapshot.clone(), limits)?;
    let applied = prepare_cancellable(&original, &request.original_transaction, limits, check)?;
    let (expected, replacement, anchors) = match request.direction {
        HistoryDirection::Undo => (
            applied.snapshot.document(),
            original.document(),
            applied
                .receipt
                .changes
                .anchor_maps
                .iter()
                .rev()
                .map(|map| AnchorMap {
                    paragraph: map.paragraph.clone(),
                    start: map.start,
                    deleted: map.inserted,
                    inserted: map.deleted,
                })
                .collect(),
        ),
        HistoryDirection::Redo => (
            original.document(),
            applied.snapshot.document(),
            applied.receipt.changes.anchor_maps.clone(),
        ),
    };
    // Provenance and page coordinates define the meaning of every declaration.
    // They are immutable to the ordinary edit contract and cannot be rebased.
    if current.document().format != expected.format
        || current.document().source_bindings != expected.source_bindings
        || current.document().page_size != expected.page_size
    {
        return Err(conflict("document provenance or geometry"));
    }
    let mut document = current.document().clone();
    field(
        &mut document.title,
        &expected.title,
        &replacement.title,
        "title",
    )?;
    field(
        &mut document.slide_order,
        &expected.slide_order,
        &replacement.slide_order,
        "slide order",
    )?;
    macro_rules! maps {
        ($($name:ident),+ $(,)?) => {$(
            map(&mut document.$name, &expected.$name, &replacement.$name, stringify!($name), check)?;
        )+};
    }
    maps!(
        slides, objects, themes, masters, layouts, fonts, resources, timelines
    );
    // The new graph is validated as a whole, including references introduced
    // by later independent edits. Invalid reversal cannot publish partial data.
    finish_preparation(
        current,
        document,
        request.request_id.clone(),
        history_request_digest(request)?,
        anchors,
        limits,
        check,
    )
}

fn cancelled(check: &dyn Fn() -> bool) -> Result<(), EditError> {
    if check() {
        Err(EditError::Cancelled)
    } else {
        Ok(())
    }
}

fn conflict(field: &str) -> EditError {
    EditError::ReferenceConflict(format!("history precondition changed: {field}"))
}

fn field<T: Clone + PartialEq>(
    current: &mut T,
    expected: &T,
    replacement: &T,
    name: &str,
) -> Result<(), EditError> {
    if expected != replacement {
        if current != expected {
            return Err(conflict(name));
        }
        *current = replacement.clone();
    }
    Ok(())
}

fn map<K: Clone + Ord, V: Clone + PartialEq>(
    current: &mut BTreeMap<K, V>,
    expected: &BTreeMap<K, V>,
    replacement: &BTreeMap<K, V>,
    name: &str,
    check: &dyn Fn() -> bool,
) -> Result<(), EditError> {
    let keys: BTreeSet<_> = expected.keys().chain(replacement.keys()).collect();
    for key in keys {
        cancelled(check)?;
        if expected.get(key) == replacement.get(key) {
            continue;
        }
        if current.get(key) != expected.get(key) {
            return Err(conflict(name));
        }
        match replacement.get(key) {
            Some(value) => {
                current.insert(key.clone(), value.clone());
            }
            None => {
                current.remove(key);
            }
        }
    }
    Ok(())
}
