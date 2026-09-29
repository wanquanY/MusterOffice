use crate::{NativeExportCandidate, NativeExporter};
use mo_presentation_operations::{
    ComputationReceipt, ComputationResult, DocumentAction, ExportAssets, Failure, FailureCode,
    Invocation, compute_import, compute_inline,
};

/// Owns one computed response and, for exports, its retained binary outputs.
/// There is no queue, retry record, persistent document head or product commit.
pub struct Execution {
    receipt: ComputationReceipt,
    export: Option<NativeExportCandidate>,
}
impl Execution {
    pub fn receipt(&self) -> &ComputationReceipt {
        &self.receipt
    }
    pub fn export(&self) -> Option<&NativeExportCandidate> {
        self.export.as_ref()
    }
    /// Move the response and retained bytes to the caller without cloning a
    /// document. The caller owns release of the optional binary candidate.
    pub fn into_parts(self) -> (ComputationReceipt, Option<NativeExportCandidate>) {
        (self.receipt, self.export)
    }
}

/// Shared direct dispatch for SDK/CLI/MCP. The caller supplies immutable input
/// bytes and an optional native exporter; no filesystem policy enters the input.
pub fn execute(
    invocation: Invocation,
    assets: &dyn ExportAssets,
    exporter: Option<&NativeExporter>,
    cancelled: &dyn Fn() -> bool,
) -> Result<Execution, Failure> {
    if cancelled() {
        return Err(Failure::new(
            FailureCode::Cancelled,
            "computation cancelled",
        ));
    }
    if !matches!(
        invocation.request.action,
        DocumentAction::Import { .. } | DocumentAction::Export { .. }
    ) {
        return Ok(Execution {
            receipt: compute_inline(invocation, cancelled)?,
            export: None,
        });
    }
    invocation.validate_cancellable(cancelled)?;
    let Invocation { request, snapshot } = invocation;
    let computation = request.computation();
    let base = snapshot.map(|s| *s);
    let (request_digest, result, export) = match &request.action {
        DocumentAction::Export { .. } => {
            let exporter = exporter.ok_or_else(|| {
                Failure::new(
                    FailureCode::ExecutorMismatch,
                    "native exporter not configured",
                )
            })?;
            let candidate = exporter.prepare(
                &request,
                base.expect("invocation base validated"),
                assets,
                cancelled,
            )?;
            let digest = candidate.request_digest().clone();
            let result = ComputationResult::Exported {
                receipt: Box::new(candidate.receipt().clone()),
            };
            (digest, result, Some(candidate))
        }
        DocumentAction::Import { source, .. } => {
            let candidate =
                compute_import(&computation, base, assets.get(&source.asset_id)?, cancelled)?;
            let digest = candidate.request_digest().clone();
            let (snapshot, receipt) = candidate.into_parts();
            let result = ComputationResult::Mutated {
                snapshot: Box::new(snapshot),
                receipt,
            };
            (digest, result, None)
        }
        _ => unreachable!("inline computation dispatched above"),
    };
    if cancelled() {
        return Err(Failure::new(
            FailureCode::Cancelled,
            "computation cancelled",
        ));
    }
    Ok(Execution {
        receipt: ComputationReceipt {
            request_id: request.request_id,
            request_digest,
            result,
        },
        export,
    })
}
