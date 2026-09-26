//! Export participates in the existing job owner and final commit transaction.
use crate::{
    AssetReader, ResultSpec, SqlResultReader, SqlResultSink, StandardHost, WorkItem, WorkLease, db,
    execution::ExecutionCheck, results,
};
use mo_common::RequestId;
use mo_operation_service::*;
use mo_presentation_delivery::{DeliveryError, OutputStore, PreviewRenderer};
use rusqlite::{Transaction, TransactionBehavior};
use std::collections::BTreeMap;

struct Inputs<'a>(BTreeMap<AssetId, AssetReader<'a>>);
impl ExportAssets for Inputs<'_> {
    fn get(&self, id: &AssetId) -> Result<ExportAsset<'_>, Failure> {
        let reader = self.0.get(id).ok_or_else(db::not_found)?;
        Ok(ExportAsset {
            info: reader.info(),
            reader,
        })
    }
}
struct Outputs<'a> {
    host: &'a StandardHost,
    context: CallContext,
    lease: WorkLease,
    clock: &'a dyn Fn() -> UnixMillis,
}
impl<'a> OutputStore for Outputs<'a> {
    type Sink = SqlResultSink<'a>;
    fn create(
        &mut self,
        name: &str,
        mime: &str,
        max_bytes: u64,
    ) -> Result<Self::Sink, DeliveryError> {
        let name = RequestId::new(name).map_err(|_| DeliveryError::Invalid("output name"))?;
        self.host
            .create_result(
                &self.context,
                &self.lease,
                ResultSpec {
                    name,
                    media_type: mime.into(),
                    max_bytes,
                },
                self.clock,
            )
            .map_err(|e| DeliveryError::Io(std::io::Error::other(e)))
    }
}
impl StandardHost {
    /// Trusted runtime configuration; never supplied by operation JSON.
    pub fn set_preview_renderer(&mut self, renderer: Box<dyn PreviewRenderer + Send>) {
        self.renderer = Some(renderer);
    }
    /// Computes into execution-private storage. Does not make assets visible.
    pub fn prepare_export<'h>(
        &'h self,
        context: &CallContext,
        work: &WorkItem,
        renderer: &mut dyn PreviewRenderer,
        clock: &'h dyn Fn() -> UnixMillis,
        check: &dyn Fn() -> bool,
    ) -> Result<ExportCandidate<SqlResultReader<'h>>, Failure> {
        context.authorize(&work.request)?;
        if work.request.digest().map_err(|_| db::corrupt())? != work.lease.request_digest {
            return Err(Failure::new(
                FailureCode::StaleExecution,
                "export work request differs",
            ));
        }
        let monitor = ExecutionCheck::new(self, context, &work.lease, clock, check)?;
        let DocumentAction::Export { settings, .. } = &work.request.action else {
            return Err(Failure::new(
                FailureCode::InputInvalid,
                "export work required",
            ));
        };
        let snapshot = work.snapshot.clone().ok_or_else(db::not_found)?;
        let mut inputs = BTreeMap::new();
        for id in settings
            .resources
            .iter()
            .map(|b| &b.asset_id)
            .chain(settings.font_asset_id.iter())
        {
            if !inputs.contains_key(id) {
                inputs.insert(id.clone(), self.open_asset(context, id)?);
            }
        }
        let mut store = Outputs {
            host: self,
            context: context.clone(),
            lease: work.lease.clone(),
            clock,
        };
        let limits = self.export_limits();
        let candidate = compute_export(
            &work.request,
            snapshot,
            &Inputs(inputs),
            &mut store,
            renderer,
            limits,
            &|| monitor.cancelled(),
        );
        monitor.finish(candidate)
    }
    pub(crate) fn run_export_work(
        &mut self,
        context: &CallContext,
        work: WorkItem,
        clock: &dyn Fn() -> UnixMillis,
        check: &dyn Fn() -> bool,
    ) -> Result<JobInfo, Failure> {
        let Some(mut renderer) = self.renderer.take() else {
            return self.finish_export(
                context,
                &work.lease,
                Err(Failure::new(
                    FailureCode::ExecutorMismatch,
                    "export renderer is not configured",
                )),
                clock(),
            );
        };
        let result = {
            let candidate = self.prepare_export(context, &work, renderer.as_mut(), clock, check);
            self.finish_export(context, &work.lease, candidate, clock())
        };
        self.renderer = Some(renderer);
        result
    }
    /// The candidate borrows private readers; a shared connection transaction
    /// permits commit without copying their payloads or holding an I/O callback.
    pub fn finish_export(
        &self,
        context: &CallContext,
        lease: &WorkLease,
        candidate: Result<ExportCandidate<SqlResultReader<'_>>, Failure>,
        now: UnixMillis,
    ) -> Result<JobInfo, Failure> {
        self.check_lease_owner(context, lease)?;
        let tx = Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let db::StoredJob { request, mut info } = db::job(&tx, context, &lease.id)?;
        context.authorize(&request)?;
        let DocumentAction::Export {
            document_id,
            base_revision,
            ..
        } = &request.action
        else {
            return Err(Failure::new(
                FailureCode::InputInvalid,
                "export operation required",
            ));
        };
        if info.fence != lease.fence
            || info.executor_digest != lease.executor
            || info.request_digest != lease.request_digest
        {
            return Err(Failure::new(
                FailureCode::StaleExecution,
                "export execution differs",
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
                "export is not executing",
            ));
        }
        let result = if info.cancel_requested {
            Err(Failure::new(
                FailureCode::Cancelled,
                "cancelled before export commit",
            ))
        } else {
            candidate
        };
        let result = result.and_then(|candidate| {
            let receipt = candidate.receipt();
            if candidate.request_digest() != &info.request_digest
                || &receipt.document_id != document_id
                || &receipt.revision != base_revision
                || receipt.semantic_digest
                    != db::semantic_digest(&tx, &context.scope, document_id, base_revision)?
            {
                return Err(Failure::new(
                    FailureCode::StaleExecution,
                    "export candidate input differs",
                ));
            }
            // Historical revision exports do not mutate or conflict with head.
            Ok(candidate)
        });
        match result {
            Ok(candidate) => {
                let plan = results::prepare_publication(
                    &tx,
                    context,
                    lease,
                    candidate.delivery().artifacts(),
                    now,
                );
                match plan {
                    Ok(plan) => {
                        results::commit_publication(&tx, plan, now)?;
                        info.state = JobState::Succeeded;
                        info.updated_at = now;
                        info.lease_until = None;
                        info.result = Some(TerminalResult::Succeeded {
                            receipt: Box::new(OperationReceipt::Export(Box::new(
                                candidate.receipt(),
                            ))),
                        });
                    }
                    Err(error)
                        if matches!(
                            error.code,
                            FailureCode::StorageFailure | FailureCode::StorageBusy
                        ) =>
                    {
                        return Err(error);
                    }
                    Err(error) => db::failed(&mut info, now, error),
                }
            }
            Err(error)
                if matches!(
                    error.code,
                    FailureCode::StorageFailure | FailureCode::StorageBusy
                ) =>
            {
                return Err(error);
            }
            Err(error) => db::failed(&mut info, now, error),
        }
        db::save(&tx, context, &info)?;
        tx.commit().map_err(db::error)?;
        Ok(info)
    }
}
