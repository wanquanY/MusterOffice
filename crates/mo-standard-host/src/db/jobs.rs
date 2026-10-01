//! Immutable admission bindings and bounded control reads. Request bodies are
//! validated at admission, migration, execution load and final publication.
use super::{corrupt, decode, error, not_found};
use mo_common::Digest;
use mo_operation_service::*;
use rusqlite::{Connection, OptionalExtension, params};

pub(crate) struct StoredJob {
    pub request: OperationRequest,
    pub info: JobInfo,
}

fn base_revision(request: &OperationRequest) -> Option<&Digest> {
    match &request.action {
        DocumentAction::Import { .. }
        | DocumentAction::Create { .. }
        | DocumentAction::Compose { .. }
        | DocumentAction::DescribeTemplate { .. }
        | DocumentAction::InstantiateTemplate { .. } => None,
        DocumentAction::Append { base_revision, .. }
        | DocumentAction::Apply { base_revision, .. }
        | DocumentAction::Export { base_revision, .. } => Some(base_revision),
    }
}

pub(crate) fn authorize_job(context: &CallContext, info: &JobInfo) -> Result<(), Failure> {
    operation(&info.operation)?.authorize(context)
}

fn operation(name: &str) -> Result<ServiceOperation, Failure> {
    match name {
        "presentations.import" => Ok(ServiceOperation::Import),
        "presentations.create" => Ok(ServiceOperation::Create),
        "presentations.apply" => Ok(ServiceOperation::Apply),
        "presentations.export" => Ok(ServiceOperation::Export),
        _ => Err(corrupt()),
    }
}

fn validate_info(info: &JobInfo, base: Option<&Digest>) -> Result<(), Failure> {
    let operation = operation(&info.operation)?;
    if matches!(
        operation,
        ServiceOperation::Create | ServiceOperation::Import
    ) != base.is_none()
    {
        return Err(corrupt());
    }
    let valid = match (&info.state, &info.result) {
        (JobState::Queued | JobState::Running, None) => true,
        (JobState::Succeeded, Some(TerminalResult::Succeeded { receipt })) => {
            receipt.document_id() == &info.document_id
                && !info.cancel_requested
                && match (operation, receipt.as_ref()) {
                    (
                        ServiceOperation::Import
                        | ServiceOperation::Create
                        | ServiceOperation::Apply,
                        OperationReceipt::Mutation(_),
                    ) => true,
                    (ServiceOperation::Export, OperationReceipt::Export(r)) => {
                        Some(&r.revision) == base
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
    if !valid
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
    Ok(())
}

pub(crate) fn insert_binding(
    c: &Connection,
    context: &CallContext,
    info: &JobInfo,
    request: &OperationRequest,
) -> Result<(), Failure> {
    c.execute(
        "INSERT INTO job_bindings(scope,principal,id,document_id,request_digest,base_revision) VALUES (?1,?2,?3,?4,?5,?6)",
        params![context.scope.as_str(), context.principal.as_str(), info.id.as_str(),
            info.document_id.as_str(), info.request_digest.as_str(), base_revision(request).map(Digest::as_str)],
    ).map_err(error)?;
    Ok(())
}

pub(crate) fn job_info(
    c: &Connection,
    context: &CallContext,
    id: &JobId,
) -> Result<JobInfo, Failure> {
    let row = c
        .query_row(
            "SELECT j.info,j.operation,j.request_id,b.document_id,b.request_digest,b.base_revision
         FROM jobs j LEFT JOIN job_bindings b USING(scope,principal,id)
         WHERE j.scope=?1 AND j.principal=?2 AND j.id=?3",
            params![
                context.scope.as_str(),
                context.principal.as_str(),
                id.as_str()
            ],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, Option<String>>(5)?,
                ))
            },
        )
        .optional()
        .map_err(error)?
        .ok_or_else(not_found)?;
    let info: JobInfo = decode(&row.0)?;
    if info.id != *id
        || info.operation != row.1
        || info.request_id.as_str() != row.2
        || Some(info.document_id.as_str()) != row.3.as_deref()
        || Some(info.request_digest.as_str()) != row.4.as_deref()
    {
        return Err(corrupt());
    }
    let base = row
        .5
        .map(|s| Digest::try_from(s).map_err(|_| corrupt()))
        .transpose()?;
    validate_info(&info, base.as_ref())?;
    Ok(info)
}

pub(crate) fn request(
    c: &Connection,
    context: &CallContext,
    info: &JobInfo,
) -> Result<OperationRequest, Failure> {
    let body: String = c
        .query_row(
            "SELECT request FROM jobs WHERE scope=?1 AND principal=?2 AND id=?3",
            params![
                context.scope.as_str(),
                context.principal.as_str(),
                info.id.as_str()
            ],
            |r| r.get(0),
        )
        .map_err(error)?;
    let request: OperationRequest = decode(&body)?;
    validate_request(&request, info)?;
    Ok(request)
}

fn validate_request(request: &OperationRequest, info: &JobInfo) -> Result<(), Failure> {
    request.validate_profile().map_err(|_| corrupt())?;
    if info.request_id != request.request_id
        || info.operation != request.action.name()
        || info.document_id != *request.action.document_id()
        || request.digest().map_err(|_| corrupt())? != info.request_digest
    {
        return Err(corrupt());
    }
    validate_info(info, base_revision(request))
}

pub(crate) fn job(c: &Connection, context: &CallContext, id: &JobId) -> Result<StoredJob, Failure> {
    let info = job_info(c, context, id)?;
    let request = request(c, context, &info)?;
    Ok(StoredJob { request, info })
}

pub(super) fn migrate(c: &Connection) -> Result<(), Failure> {
    c.execute_batch("CREATE TABLE job_bindings(
        scope TEXT NOT NULL, principal TEXT NOT NULL, id TEXT NOT NULL,
        document_id TEXT NOT NULL, request_digest TEXT NOT NULL, base_revision TEXT,
        PRIMARY KEY(scope,principal,id), FOREIGN KEY(scope,principal,id) REFERENCES jobs(scope,principal,id) ON DELETE CASCADE
    ) STRICT;").map_err(error)?;
    // Stream old rows inside the migration transaction: no document-sized
    // collection and no partially published projection on a corrupt row.
    let mut query = c
        .prepare("SELECT scope,principal,id,operation,request_id,request,info FROM jobs")
        .map_err(error)?;
    let mut rows = query.query([]).map_err(error)?;
    while let Some(row) = rows.next().map_err(error)? {
        let context = CallContext {
            scope: ScopeId::new(row.get::<_, String>(0).map_err(error)?).map_err(|_| corrupt())?,
            principal: PrincipalId::new(row.get::<_, String>(1).map_err(error)?)
                .map_err(|_| corrupt())?,
            permissions: Default::default(),
        };
        let info: JobInfo = decode(&row.get::<_, String>(6).map_err(error)?)?;
        if info.id.as_str() != row.get::<_, String>(2).map_err(error)?
            || info.operation != row.get::<_, String>(3).map_err(error)?
            || info.request_id.as_str() != row.get::<_, String>(4).map_err(error)?
        {
            return Err(corrupt());
        }
        let request: OperationRequest = decode(&row.get::<_, String>(5).map_err(error)?)?;
        validate_request(&request, &info)?;
        insert_binding(c, &context, &info, &request)?;
    }
    Ok(())
}
