use crate::execution::RetainedSpool;
use crate::{storage::*, wire::*, *};
use mo_common::RequestId;
use mo_native_io::{ExecutionSpoolRoot, SpoolRecovery};
use mo_presentation_delivery::{
    Content, DeliveryAsset, DeliveryError, DeliverySource, ReceiptInspection, ReceivedDelivery,
    RendererIdentity,
};
use mo_presentation_edit::SnapshotRecord;
use mo_presentation_operations::{DocumentAction, ExportAssets, ExportReceipt, OperationRequest};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};

pub struct NativeExporter {
    executable: PathBuf,
    root: ExecutionSpoolRoot,
    timeout: Duration,
    identity: RendererIdentity,
}
/// Private, retained, verified bytes. This is a computation candidate, not a
/// job success receipt or authority to publish an Artifact. Fields cannot be
/// replaced by deserializing a producer's report.
pub struct NativeExportCandidate {
    assets: ReceivedAssets,
    received: ReceivedDelivery,
    receipt: ExportReceipt,
    request_digest: Digest,
    expectation: mo_presentation_delivery::DeliveryExpectation,
    // Last field: readers close before directory cleanup, including on Windows.
    workspace: RetainedSpool,
}
impl NativeExportCandidate {
    /// Pins validated against the accepted request, before computation. Use
    /// these to inspect bytes read back from final host storage. This does not
    /// confer product authorization or replace its cancellation/fence checks.
    pub fn expectation(&self) -> &mo_presentation_delivery::DeliveryExpectation {
        &self.expectation
    }
    pub fn request_digest(&self) -> &Digest {
        &self.request_digest
    }
    pub fn receipt(&self) -> &ExportReceipt {
        &self.receipt
    }
    pub fn inspection(&self) -> &ReceiptInspection {
        self.received.report()
    }
    /// Bound renderer observations; these do not upgrade quality declarations.
    pub fn preview_measurements(&self) -> &[mo_presentation_delivery::PreviewMeasurements] {
        self.received.preview_measurements()
    }
    pub fn assets(&self) -> &[DeliveryAsset] {
        &self.receipt.bundle.assets
    }
    /// The host can transfer these retained bytes to its own Content Store,
    /// then recheck invocation/fence/pins/cancellation in its one transaction.
    pub fn open(&self, id: &RequestId) -> Result<Content<'_>, DeliveryError> {
        self.assets.open(id)
    }
    pub fn discard(self) -> Result<(), Failure> {
        let Self {
            assets, workspace, ..
        } = self;
        drop(assets);
        workspace.discard().map_err(storage_failure)
    }
    /// The caller keeps its execution slot while transient registry contention
    /// is resolved. This is resource cleanup, not a business retry or job.
    pub fn discard_with_wait(self, timeout: Duration) -> Result<(), Failure> {
        let Self {
            assets, workspace, ..
        } = self;
        drop(assets);
        workspace
            .discard_with_wait(timeout)
            .map_err(storage_failure)
    }
}
impl NativeExporter {
    /// Operator configuration. The root must be private/protected and have a
    /// host disk reservation for inputs + worker outputs + received outputs.
    /// At current limits this is at most 1.5 GiB of logical file bytes per call;
    /// metadata, filesystem overhead and raster/font memory are separate.
    pub fn new(
        executable: PathBuf,
        expected: Digest,
        root: PathBuf,
        timeout: Duration,
    ) -> Result<Self, Failure> {
        if timeout.is_zero() || timeout > Duration::from_secs(3600) {
            return Err(invalid("export worker deadline"));
        }
        if executable_digest(&executable)? != expected {
            return Err(Failure::new(
                FailureCode::ExecutorMismatch,
                "export worker executable digest differs",
            ));
        }
        let root = ExecutionSpoolRoot::open(&root).map_err(storage_failure)?;
        Ok(Self {
            executable,
            root,
            timeout,
            identity: RendererIdentity {
                implementation_sha256: expected,
                profile: "drawingml-resource-page-q32-v1-draft".into(),
            },
        })
    }
    pub fn renderer_identity(&self) -> RendererIdentity {
        self.identity.clone()
    }
    /// Host startup/maintenance may recover abandoned executions in bounded
    /// batches. A contended registry returns `WouldBlock`; all live candidates
    /// and workers remain protected independently of application task state.
    pub fn recover_spools(&self, max_reclaims: usize) -> std::io::Result<SpoolRecovery> {
        self.root.recover(max_reclaims)
    }
    /// Called inside the existing Runtime task. Input assets are already
    /// authorized immutable capabilities. This does not create a second job.
    pub fn prepare(
        &self,
        request: &OperationRequest,
        snapshot: SnapshotRecord,
        assets: &dyn ExportAssets,
        check: &dyn Fn() -> bool,
    ) -> Result<NativeExportCandidate, Failure> {
        let cancel = || Failure::new(FailureCode::Cancelled, "embedded export cancelled");
        if check() {
            return Err(cancel());
        }
        if executable_digest(&self.executable)? != self.identity.implementation_sha256 {
            return Err(Failure::new(
                FailureCode::ExecutorMismatch,
                "export executable changed",
            ));
        }
        let DocumentAction::Export { settings, .. } = &request.action else {
            return Err(invalid("export action required"));
        };
        let ids: BTreeSet<_> = settings
            .resources
            .iter()
            .map(|b| &b.asset_id)
            .chain(settings.font_asset_id.iter())
            .collect();
        let mut inputs = Vec::new();
        for id in ids {
            if check() {
                return Err(cancel());
            }
            let asset = assets.get(id)?;
            if &asset.info.id != id {
                return Err(invalid("resolved input asset identity differs"));
            }
            inputs.push(asset);
        }
        let request_digest = request
            .digest()
            .map_err(|_| invalid("export request digest"))?;
        let wire = Request {
            version: VERSION.into(),
            request: request.clone(),
            snapshot,
            assets: inputs.iter().map(|a| a.info.clone()).collect(),
        };
        let expected = wire.validate(&self.identity)?;
        let framed = encode(&wire)?;
        let start = Instant::now();
        let workspace = loop {
            if check() {
                return Err(cancel());
            }
            if start.elapsed() >= self.timeout {
                return Err(Failure::new(
                    FailureCode::ExecutionInterrupted,
                    "export spool acquisition timed out",
                ));
            }
            match self.root.create() {
                Ok(workspace) => break workspace,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(error) => return Err(storage_failure(error)),
            }
        };
        let workspace = RetainedSpool::new(workspace, self.timeout);
        let remaining = self
            .timeout
            .checked_sub(start.elapsed())
            .filter(|t| !t.is_zero())
            .ok_or_else(|| {
                Failure::new(
                    FailureCode::ExecutionInterrupted,
                    "export spool acquisition timed out",
                )
            })?;
        let receive_root = workspace.path().to_path_buf();
        let mut command = Command::new(&self.executable);
        // Old workers reject the extra argument before doing any work. New
        // workers require it and acquire a participant lease before I/O.
        command
            .arg("--spool-dir")
            .arg(workspace.path())
            .arg("--execution-lease-v1");
        let mut part = 0usize;
        let mut offset = 0u64;
        let outcome = mo_native_worker::exchange_stream(
            command,
            remaining,
            check,
            || {
                loop {
                    let (reader, length): (&dyn mo_opc::ReaderAt, u64) = if part == 0 {
                        (&framed, framed.len() as u64)
                    } else if let Some(asset) = inputs.get(part - 1) {
                        (asset.reader, asset.info.descriptor.byte_length.get())
                    } else {
                        return Ok(None);
                    };
                    if offset == length {
                        part += 1;
                        offset = 0;
                        continue;
                    }
                    let n = (length - offset).min(mo_native_worker::CHUNK_BYTES as u64) as usize;
                    let mut bytes = vec![0; n];
                    reader
                        .read_exact_at(&mut bytes, offset)
                        .map_err(|_| "embedded input resource read failed")?;
                    offset += n as u64;
                    return Ok(Some(bytes));
                }
            },
            move |stdout| {
                let result = (|| {
                    let response: Response = read(stdout)?;
                    match response {
                        Response::Failed { error } => Err(error),
                        Response::Prepared {
                            request_digest,
                            receipt,
                        } => {
                            output_preflight(&receipt.bundle.assets)?;
                            let mut assets = ReceivedAssets(BTreeMap::new());
                            for asset in &receipt.bundle.assets {
                                let reader =
                                    receive(stdout, &receive_root, asset.byte_length.get())?;
                                assets.0.insert(asset.id.clone(), (asset.clone(), reader));
                            }
                            Ok((request_digest, *receipt, assets))
                        }
                    }
                })();
                // Computational failures remain structured; exchange still checks
                // complete EOF and process success before returning either outcome.
                Ok(result)
            },
        );
        if check() {
            return Err(cancel());
        }
        let (actual_request, receipt, assets) = outcome
            .map_err(|message| Failure::new(FailureCode::ExecutionInterrupted, message))??;
        if actual_request != request_digest
            || receipt.document_id != expected.document_id
            || receipt.revision != expected.revision
            || receipt.semantic_digest != expected.semantic_digest
        {
            return Err(Failure::new(
                FailureCode::ResultMismatch,
                "export response input binding differs",
            ));
        }
        let received = mo_presentation_delivery::inspect(
            &receipt.bundle,
            &expected,
            &assets,
            Default::default(),
            check,
        )
        .map_err(mo_presentation_operations::delivery_failure)?;
        if check() {
            return Err(cancel());
        }
        Ok(NativeExportCandidate {
            assets,
            received,
            receipt,
            request_digest,
            expectation: expected,
            workspace,
        })
    }
}
