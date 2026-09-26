use crate::{StandardHost, db};
use mo_common::Digest;
use mo_operation_service::*;
use mo_presentation_edit::SnapshotRecord;
use rusqlite::{OptionalExtension, TransactionBehavior, params};

/// Shared by explicit claims and queue claims, always inside the caller's
/// immediate transaction. Snapshot decoding happens after this transaction.
pub(crate) fn claim_record(
    tx: &rusqlite::Connection,
    context: &CallContext,
    id: &JobId,
    executor: &Digest,
    lease_ms: i64,
    now: UnixMillis,
) -> Result<Option<db::StoredJob>, Failure> {
    let mut job = db::job(tx, context, id)?;
    context.authorize(&job.request)?;
    if db::expire(&mut job.info, now)? {
        db::save(tx, context, &job.info)?;
    }
    if job.info.state != JobState::Queued {
        return Ok(None);
    }
    if &job.info.executor_digest != executor {
        return Err(Failure::new(
            FailureCode::ExecutorMismatch,
            "queued operation requires its pinned executor",
        ));
    }
    job.info.state = JobState::Running;
    job.info.updated_at = now;
    job.info.fence = job
        .info
        .fence
        .checked_add(1)
        .ok_or_else(|| Failure::new(FailureCode::LimitExceeded, "execution fence exhausted"))?;
    job.info.lease_until = Some(
        now.checked_add(lease_ms)
            .ok_or_else(|| Failure::new(FailureCode::LimitExceeded, "execution lease overflow"))?,
    );
    db::save(tx, context, &job.info)?;
    Ok(Some(job))
}

/// Not a bearer credential and never deserialized from public tool arguments.
#[derive(Debug, Clone)]
pub struct WorkLease {
    pub(crate) id: JobId,
    pub(crate) principal: PrincipalId,
    pub(crate) scope: ScopeId,
    pub(crate) fence: JobFence,
    pub(crate) executor: Digest,
    pub(crate) request_digest: Digest,
}
impl WorkLease {
    pub fn job_id(&self) -> &JobId {
        &self.id
    }
    pub fn fence(&self) -> JobFence {
        self.fence
    }
}
pub struct WorkItem {
    pub lease: WorkLease,
    pub request: OperationRequest,
    pub snapshot: Option<SnapshotRecord>,
}
impl StandardHost {
    pub fn submit(
        &mut self,
        context: &CallContext,
        request: OperationRequest,
        now: UnixMillis,
    ) -> Result<JobInfo, Failure> {
        self.submit_with_admission(context, request, now, &|| Ok(()))
    }
    /// Admission applies only to a new record. Idempotent recovery of a prior
    /// acceptance must remain possible while an execution pool is unavailable.
    pub(crate) fn submit_with_admission(
        &mut self,
        context: &CallContext,
        request: OperationRequest,
        now: UnixMillis,
        admit: &dyn Fn() -> Result<(), Failure>,
    ) -> Result<JobInfo, Failure> {
        context.authorize(&request)?;
        let encoded = db::encode(&request)?;
        if encoded.len() > MAX_OPERATION_BYTES {
            return Err(Failure::new(
                FailureCode::LimitExceeded,
                "operation request bytes",
            ));
        }
        let digest = request
            .digest()
            .map_err(|_| Failure::new(FailureCode::InputInvalid, "canonical operation request"))?;
        let id = mo_common::digest(
            "musteroffice.host.job-id/1",
            &(
                &context.scope,
                &context.principal,
                request.action.name(),
                &request.request_id,
            ),
        )
        .map_err(|_| db::corrupt())?;
        let id = JobId::new(format!("job:{id}")).map_err(|_| db::corrupt())?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let existing:Option<String>=tx.query_row("SELECT id FROM jobs WHERE scope=?1 AND principal=?2 AND operation=?3 AND request_id=?4",params![context.scope.as_str(),context.principal.as_str(),request.action.name(),request.request_id.as_str()],|r|r.get(0)).optional().map_err(db::error)?;
        if let Some(existing) = existing {
            let mut job = db::job(
                &tx,
                context,
                &JobId::new(existing).map_err(|_| db::corrupt())?,
            )?
            .info;
            if job.request_digest != digest {
                return Err(Failure::new(
                    FailureCode::RequestIdReused,
                    "request id already identifies different arguments",
                ));
            }
            if db::expire(&mut job, now)? {
                db::save(&tx, context, &job)?;
            }
            tx.commit().map_err(db::error)?;
            return Ok(job);
        }
        admit()?;
        let count: u32 = tx
            .query_row(
                "SELECT count(*) FROM jobs WHERE scope=?1 AND principal=?2",
                params![context.scope.as_str(), context.principal.as_str()],
                |r| r.get(0),
            )
            .map_err(db::error)?;
        if count >= self.limits.max_jobs_per_principal {
            return Err(Failure::new(
                FailureCode::LimitExceeded,
                "durable operation receipt quota",
            ));
        }
        let info = JobInfo {
            contract_version: ContractVersion::V1,
            id,
            request_id: request.request_id.clone(),
            operation: request.action.name().into(),
            document_id: request.action.document_id().clone(),
            request_digest: digest,
            executor_digest: self.executor.clone(),
            state: JobState::Queued,
            cancel_requested: false,
            fence: JobFence::new(0).expect("zero"),
            created_at: now,
            updated_at: now,
            lease_until: None,
            result: None,
        };
        tx.execute("INSERT INTO jobs(scope,principal,id,operation,request_id,request,info) VALUES (?1,?2,?3,?4,?5,?6,?7)",params![context.scope.as_str(),context.principal.as_str(),info.id.as_str(),info.operation,info.request_id.as_str(),encoded,db::encode(&info)?]).map_err(db::error)?;
        tx.commit().map_err(db::error)?;
        Ok(info)
    }
    pub fn get_job(
        &mut self,
        context: &CallContext,
        id: &JobId,
        now: UnixMillis,
    ) -> Result<JobInfo, Failure> {
        context.require(Permission::ReadJob)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let mut job = db::job(&tx, context, id)?.info;
        if db::expire(&mut job, now)? {
            db::save(&tx, context, &job)?;
        }
        tx.commit().map_err(db::error)?;
        Ok(job)
    }
    /// Return the caller's own operation receipt while waiting for an accepted
    /// mutation. This uses execution authority, not the separate query grant.
    pub(crate) fn operation_status(
        &mut self,
        context: &CallContext,
        id: &JobId,
        now: UnixMillis,
    ) -> Result<JobInfo, Failure> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let db::StoredJob { request, mut info } = db::job(&tx, context, id)?;
        context.authorize(&request)?;
        if db::expire(&mut info, now)? {
            db::save(&tx, context, &info)?;
        }
        tx.commit().map_err(db::error)?;
        Ok(info)
    }
    pub fn cancel_job(
        &mut self,
        context: &CallContext,
        id: &JobId,
        now: UnixMillis,
    ) -> Result<JobInfo, Failure> {
        context.require(Permission::CancelJob)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let mut job = db::job(&tx, context, id)?.info;
        db::expire(&mut job, now)?;
        if !job.state.terminal() {
            job.cancel_requested = true;
            job.updated_at = now;
            if job.state == JobState::Queued {
                db::failed(
                    &mut job,
                    now,
                    Failure::new(FailureCode::Cancelled, "cancelled before execution"),
                );
            }
        }
        db::save(&tx, context, &job)?;
        tx.commit().map_err(db::error)?;
        Ok(job)
    }
    pub fn claim(
        &mut self,
        context: &CallContext,
        id: &JobId,
        now: UnixMillis,
    ) -> Result<Option<WorkItem>, Failure> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let claimed = claim_record(&tx, context, id, &self.executor, self.limits.lease_ms, now)?;
        tx.commit().map_err(db::error)?;
        claimed.map(|job| self.load_work(context, job)).transpose()
    }
    pub(crate) fn load_work(
        &self,
        context: &CallContext,
        job: db::StoredJob,
    ) -> Result<WorkItem, Failure> {
        let db::StoredJob { request, info } = job;
        // Immutable revisions can be decoded/validated outside the write lock.
        let snapshot = match &request.action {
            DocumentAction::Create { .. } => None,
            DocumentAction::Apply {
                document_id,
                base_revision,
                ..
            }
            | DocumentAction::Export {
                document_id,
                base_revision,
                ..
            } => db::snapshot(&self.connection, &context.scope, document_id, base_revision)?,
        };
        Ok(WorkItem {
            lease: WorkLease {
                id: info.id,
                principal: context.principal.clone(),
                scope: context.scope.clone(),
                fence: info.fence,
                executor: self.executor.clone(),
                request_digest: info.request_digest,
            },
            request,
            snapshot,
        })
    }
    pub fn renew(
        &self,
        context: &CallContext,
        lease: &WorkLease,
        now: UnixMillis,
    ) -> Result<JobInfo, Failure> {
        self.check_lease_owner(context, lease)?;
        let tx =
            rusqlite::Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(db::error)?;
        let db::StoredJob { request, mut info } = db::job(&tx, context, &lease.id)?;
        context.authorize(&request)?;
        if info.fence != lease.fence || info.executor_digest != lease.executor {
            return Err(Failure::new(
                FailureCode::StaleExecution,
                "execution fence differs",
            ));
        }
        if db::expire(&mut info, now)? {
            db::save(&tx, context, &info)?;
        }
        if info.state == JobState::Running && info.fence == lease.fence && !info.cancel_requested {
            info.updated_at = now;
            info.lease_until = Some(now.checked_add(self.limits.lease_ms).ok_or_else(|| {
                Failure::new(FailureCode::LimitExceeded, "execution lease overflow")
            })?);
            db::save(&tx, context, &info)?;
        }
        tx.commit().map_err(db::error)?;
        Ok(info)
    }
    pub(crate) fn check_lease_owner(
        &self,
        context: &CallContext,
        lease: &WorkLease,
    ) -> Result<(), Failure> {
        if lease.principal != context.principal || lease.scope != context.scope {
            return Err(db::not_found());
        }
        if lease.executor != self.executor {
            return Err(Failure::new(
                FailureCode::ExecutorMismatch,
                "execution owner differs",
            ));
        }
        Ok(())
    }
    pub fn finish(
        &mut self,
        context: &CallContext,
        lease: &WorkLease,
        candidate: Result<MutationCandidate, Failure>,
        now: UnixMillis,
    ) -> Result<JobInfo, Failure> {
        self.check_lease_owner(context, lease)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let db::StoredJob { request, mut info } = db::job(&tx, context, &lease.id)?;
        context.authorize(&request)?;
        if info.fence != lease.fence || info.executor_digest != lease.executor {
            return Err(Failure::new(
                FailureCode::StaleExecution,
                "execution fence differs",
            ));
        }
        if db::expire(&mut info, now)? {
            db::save(&tx, context, &info)?;
        }
        if info.state.terminal() {
            tx.commit().map_err(db::error)?;
            return Ok(info);
        }
        if info.state != JobState::Running {
            return Err(Failure::new(
                FailureCode::StaleExecution,
                "operation is not executing",
            ));
        }
        let result = if info.cancel_requested {
            Err(Failure::new(
                FailureCode::Cancelled,
                "cancelled before candidate commit",
            ))
        } else {
            candidate
        };
        let result = result.and_then(|candidate| {
            if candidate.request_digest() != &info.request_digest
                || candidate.document_id() != request.action.document_id()
            {
                return Err(Failure::new(
                    FailureCode::StaleExecution,
                    "candidate does not match accepted operation",
                ));
            }
            let current = db::head(&tx, &context.scope, candidate.document_id())?;
            if current.as_ref() != candidate.base_revision() {
                return Err(Failure::new(
                    if candidate.base_revision().is_none() {
                        FailureCode::DocumentExists
                    } else {
                        FailureCode::RevisionConflict
                    },
                    "document changed before candidate commit",
                ));
            }
            // A revision label alone does not prove the computation used the
            // authorized immutable snapshot. Bind its validated content too.
            if let Some(revision) = &current {
                let stored =
                    db::semantic_digest(&tx, &context.scope, candidate.document_id(), revision)?;
                if candidate.base_semantic_digest() != Some(&stored) {
                    return Err(Failure::new(
                        FailureCode::StaleExecution,
                        "candidate base content differs",
                    ));
                }
            }
            if current.is_none() {
                let count: u32 = tx
                    .query_row(
                        "SELECT count(*) FROM heads WHERE scope=?1",
                        [context.scope.as_str()],
                        |r| r.get(0),
                    )
                    .map_err(db::error)?;
                if count >= self.limits.max_documents_per_scope {
                    return Err(Failure::new(
                        FailureCode::LimitExceeded,
                        "document scope quota",
                    ));
                }
            }
            Ok(candidate)
        });
        match result {
            Ok(candidate) => {
                let snapshot = candidate.snapshot();
                let body = candidate.snapshot_json();
                tx.execute("INSERT INTO revisions(scope,document_id,revision,semantic_digest,snapshot) VALUES (?1,?2,?3,?4,?5)",params![context.scope.as_str(),snapshot.document.id.as_str(),snapshot.revision.as_str(),snapshot.semantic_digest.as_str(),body]).map_err(db::error)?;
                tx.execute("INSERT INTO heads(scope,document_id,revision) VALUES (?1,?2,?3) ON CONFLICT(scope,document_id) DO UPDATE SET revision=excluded.revision",params![context.scope.as_str(),snapshot.document.id.as_str(),snapshot.revision.as_str()]).map_err(db::error)?;
                info.state = JobState::Succeeded;
                info.updated_at = now;
                info.lease_until = None;
                info.result = Some(TerminalResult::Succeeded {
                    receipt: Box::new(OperationReceipt::Mutation(candidate.receipt().clone())),
                });
            }
            Err(error) => {
                if error.code == FailureCode::StorageFailure {
                    return Err(error);
                }
                db::failed(&mut info, now, error);
            }
        }
        db::save(&tx, context, &info)?;
        tx.commit().map_err(db::error)?;
        Ok(info)
    }
}
