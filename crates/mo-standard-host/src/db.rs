use mo_common::{Digest, DocumentId};
use mo_operation_service::*;
use mo_presentation_edit::{Snapshot, SnapshotRecord};
use mo_presentation_model::ValidationLimits;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

pub(crate) fn error(_: rusqlite::Error) -> Failure {
    Failure::new(
        FailureCode::StorageFailure,
        "operation storage is unavailable",
    )
}
pub(crate) fn corrupt() -> Failure {
    Failure::new(
        FailureCode::StorageFailure,
        "operation storage integrity failure",
    )
}
pub(crate) fn not_found() -> Failure {
    Failure::new(FailureCode::NotFound, "requested object is unavailable")
}
pub(crate) fn encode<T: serde::Serialize>(value: &T) -> Result<String, Failure> {
    serde_json::to_string(value).map_err(|_| corrupt())
}
pub(crate) fn validate_media_type(mime: &str) -> Result<(), Failure> {
    if mime.len() > 128
        || !mime.is_ascii()
        || mime.bytes().any(|b| b <= 32 || b >= 127)
        || mime.split('/').count() != 2
        || mime.split('/').any(str::is_empty)
    {
        return Err(Failure::new(
            FailureCode::InputInvalid,
            "invalid resource media type declaration",
        ));
    }
    Ok(())
}
pub(crate) fn decode<T: serde::de::DeserializeOwned>(value: &str) -> Result<T, Failure> {
    if value.len() > MAX_OPERATION_BYTES * 2 {
        return Err(corrupt());
    }
    mo_common::from_json_str(value).map_err(|_| corrupt())
}
pub(crate) fn initialize(c: &mut Connection) -> Result<(), Failure> {
    let tx = c
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(error)?;
    let version: i64 = tx
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(error)?;
    let application: i64 = tx
        .query_row("PRAGMA application_id", [], |r| r.get(0))
        .map_err(error)?;
    match (version, application) {
        (0, 0) => {
            let tables: i64 = tx
                .query_row(
                    "SELECT count(*) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%'",
                    [],
                    |r| r.get(0),
                )
                .map_err(error)?;
            if tables != 0 {
                return Err(corrupt());
            }
            tx.execute_batch("CREATE TABLE jobs (scope TEXT NOT NULL, principal TEXT NOT NULL, id TEXT NOT NULL, operation TEXT NOT NULL, request_id TEXT NOT NULL, request TEXT NOT NULL, info TEXT NOT NULL, PRIMARY KEY(scope,principal,id), UNIQUE(scope,principal,operation,request_id)) STRICT;
CREATE TABLE revisions(scope TEXT NOT NULL, document_id TEXT NOT NULL, revision TEXT NOT NULL, semantic_digest TEXT NOT NULL, snapshot TEXT NOT NULL, PRIMARY KEY(scope,document_id,revision)) STRICT;
CREATE TABLE heads(scope TEXT NOT NULL, document_id TEXT NOT NULL, revision TEXT NOT NULL, PRIMARY KEY(scope,document_id), FOREIGN KEY(scope,document_id,revision) REFERENCES revisions(scope,document_id,revision)) STRICT;
PRAGMA application_id=1297041478; PRAGMA user_version=1;").map_err(error)?;
        }
        (1, 1297041478) => (),
        (2, 1297041478) => (),
        (3, 1297041478) => (),
        (4, 1297041478) => (),
        (5, 1297041478) => (),
        _ => return Err(corrupt()),
    }
    if version <= 1 {
        super::assets::migrate(&tx)?;
        tx.execute_batch("PRAGMA user_version=2;").map_err(error)?;
    }
    if version <= 2 {
        super::results::migrate(&tx)?;
        tx.execute_batch("PRAGMA user_version=3;").map_err(error)?;
    }
    if version <= 3 {
        super::results::migrate_published(&tx)?;
        tx.execute_batch("PRAGMA user_version=4;").map_err(error)?;
    }
    if version <= 4 {
        super::queue::migrate(&tx)?;
        tx.execute_batch("PRAGMA user_version=5;").map_err(error)?;
    }
    tx.commit().map_err(error)
}
pub(crate) struct StoredJob {
    pub request: OperationRequest,
    pub info: JobInfo,
}
pub(crate) fn job(c: &Connection, context: &CallContext, id: &JobId) -> Result<StoredJob, Failure> {
    let row:Option<(String,String,String,String)>=c.query_row("SELECT request,info,operation,request_id FROM jobs WHERE scope=?1 AND principal=?2 AND id=?3",params![context.scope.as_str(),context.principal.as_str(),id.as_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(error)?;
    let (request, info, operation, request_id) = row.ok_or_else(not_found)?;
    let request: OperationRequest = decode(&request)?;
    request.validate_profile().map_err(|_| corrupt())?;
    let info: JobInfo = decode(&info)?;
    if info.id != *id
        || info.request_id != request.request_id
        || info.request_id.as_str() != request_id
        || info.operation != operation
        || request.action.name() != operation
        || info.document_id != *request.action.document_id()
        || request.digest().map_err(|_| corrupt())? != info.request_digest
    {
        return Err(corrupt());
    }
    let state_valid = match (&info.state, &info.result) {
        (JobState::Queued | JobState::Running, None) => true,
        (JobState::Succeeded, Some(TerminalResult::Succeeded { receipt })) => {
            receipt.document_id() == &info.document_id
                && !info.cancel_requested
                && match (&request.action, receipt.as_ref()) {
                    (
                        DocumentAction::Create { .. } | DocumentAction::Apply { .. },
                        OperationReceipt::Mutation(_),
                    ) => true,
                    (DocumentAction::Export { base_revision, .. }, OperationReceipt::Export(r)) => {
                        &r.revision == base_revision
                            && r.bundle.document.document_id == r.document_id
                            && r.bundle.document.revision == r.revision
                            && r.bundle.profile_id == mo_presentation_delivery::PROFILE
                    }
                    _ => false,
                }
        }
        (JobState::Failed, Some(TerminalResult::Failed { error })) => {
            error.code != FailureCode::Cancelled
        }
        (JobState::Cancelled, Some(TerminalResult::Failed { error })) => {
            error.code == FailureCode::Cancelled
        }
        _ => false,
    };
    if !state_valid
        || info.updated_at < info.created_at
        || (info.state == JobState::Queued && (info.cancel_requested || info.fence.get() != 0))
        || (info.state == JobState::Running && info.fence.get() == 0)
        || (info.state == JobState::Running) != info.lease_until.is_some()
        || info
            .lease_until
            .is_some_and(|until| until <= info.updated_at)
    {
        return Err(corrupt());
    }
    Ok(StoredJob { request, info })
}
pub(crate) fn save(c: &Connection, context: &CallContext, job: &JobInfo) -> Result<(), Failure> {
    let n = c
        .execute(
            "UPDATE jobs SET info=?1 WHERE scope=?2 AND principal=?3 AND id=?4",
            params![
                encode(job)?,
                context.scope.as_str(),
                context.principal.as_str(),
                job.id.as_str()
            ],
        )
        .map_err(error)?;
    if n != 1 {
        return Err(corrupt());
    }
    super::results::job_changed(c, context, job)?;
    Ok(())
}

/// Shared logical reservation, counted inside the admission write transaction.
/// Filesystem pages, WAL, metadata and CPU memory have separate host budgets.
pub(crate) fn scope_reserved(c: &Connection, scope: &ScopeId) -> Result<u64, Failure> {
    let n: i64 = c.query_row(
        "SELECT (SELECT coalesce(sum(reserved_bytes),0) FROM asset_uploads WHERE scope=?1) + (SELECT coalesce(sum(reserved_bytes),0) FROM result_spools WHERE scope=?1)",
        [scope.as_str()], |r| r.get(0),
    ).map_err(error)?;
    u64::try_from(n).map_err(|_| corrupt())
}
pub(crate) fn head(
    c: &Connection,
    scope: &ScopeId,
    id: &DocumentId,
) -> Result<Option<Digest>, Failure> {
    let value: Option<String> = c
        .query_row(
            "SELECT revision FROM heads WHERE scope=?1 AND document_id=?2",
            params![scope.as_str(), id.as_str()],
            |r| r.get(0),
        )
        .optional()
        .map_err(error)?;
    value
        .map(|v| Digest::try_from(v).map_err(|_| corrupt()))
        .transpose()
}
pub(crate) fn snapshot(
    c: &Connection,
    scope: &ScopeId,
    id: &DocumentId,
    revision: &Digest,
) -> Result<Option<SnapshotRecord>, Failure> {
    let value: Option<(String, String)> = c
        .query_row(
            "SELECT snapshot,semantic_digest FROM revisions WHERE scope=?1 AND document_id=?2 AND revision=?3",
            params![scope.as_str(), id.as_str(), revision.as_str()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(error)?;
    value
        .map(|(v, semantic_digest)| {
            let s: SnapshotRecord = decode(&v)?;
            if &s.document.id != id
                || &s.revision != revision
                || s.semantic_digest.as_str() != semantic_digest
            {
                return Err(corrupt());
            }
            Snapshot::restore(s, ValidationLimits::default())
                .map(|s| s.into_record())
                .map_err(|_| corrupt())
        })
        .transpose()
}
pub(crate) fn semantic_digest(
    c: &Connection,
    scope: &ScopeId,
    id: &DocumentId,
    revision: &Digest,
) -> Result<Digest, Failure> {
    let value: String = c.query_row("SELECT semantic_digest FROM revisions WHERE scope=?1 AND document_id=?2 AND revision=?3", params![scope.as_str(),id.as_str(),revision.as_str()], |r| r.get(0)).map_err(error)?;
    Digest::try_from(value).map_err(|_| corrupt())
}
pub(crate) fn check_time(job: &JobInfo, now: UnixMillis) -> Result<(), Failure> {
    if now < job.updated_at {
        Err(Failure::new(
            FailureCode::InputInvalid,
            "host clock moved backwards",
        ))
    } else {
        Ok(())
    }
}
pub(crate) fn failed(job: &mut JobInfo, now: UnixMillis, error: Failure) {
    job.state = if error.code == FailureCode::Cancelled {
        JobState::Cancelled
    } else {
        JobState::Failed
    };
    job.updated_at = now;
    job.lease_until = None;
    job.result = Some(TerminalResult::Failed { error });
}
pub(crate) fn expire(job: &mut JobInfo, now: UnixMillis) -> Result<bool, Failure> {
    // Immutable terminal receipts remain readable even after a wall-clock reset.
    if job.state.terminal() {
        return Ok(false);
    }
    check_time(job, now)?;
    if job.state == JobState::Running && job.lease_until.is_some_and(|t| now >= t) {
        failed(
            job,
            now,
            if job.cancel_requested {
                Failure::new(FailureCode::Cancelled, "cancelled execution lease ended")
            } else {
                Failure::new(
                    FailureCode::ExecutionInterrupted,
                    "execution lease expired; no candidate was committed",
                )
            },
        );
        Ok(true)
    } else {
        Ok(false)
    }
}
