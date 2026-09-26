//! Coordinate the real renderer to exercise a busy executor deterministically.
use super::*;
use mo_presentation_compile::source_resource_page::SourceResourcePageImage;
use mo_presentation_delivery::{
    Content, DeliveryError, PreviewFonts, PreviewRequest, RendererIdentity,
};
use mo_standard_host::{NativeRuntime, NativeSession, RuntimeOptions, StandardHostConfig};
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::Instant;

struct Gate {
    entered: AtomicBool,
    ready: mpsc::SyncSender<()>,
    released: Mutex<bool>,
    changed: Condvar,
}
impl Gate {
    fn release(&self) {
        *self.released.lock().unwrap() = true;
        self.changed.notify_all();
    }
}
struct GatedRenderer {
    inner: NativePreviewRenderer,
    gate: Arc<Gate>,
}
impl PreviewRenderer for GatedRenderer {
    fn identity(&self) -> RendererIdentity {
        self.inner.identity()
    }
    fn render_pages(
        &mut self,
        requests: &[PreviewRequest],
        source: Content<'_>,
        fonts: PreviewFonts<'_>,
        check: &dyn Fn() -> bool,
        emit: &mut dyn FnMut(usize, SourceResourcePageImage) -> Result<(), DeliveryError>,
    ) -> Result<(), DeliveryError> {
        let gate = &self.gate;
        self.inner
            .render_pages(requests, source, fonts, check, &mut |ordinal, image| {
                if !gate.entered.swap(true, Ordering::SeqCst) {
                    gate.ready.send(()).unwrap();
                    let (released, _) = self
                        .gate
                        .changed
                        .wait_timeout_while(
                            gate.released.lock().unwrap(),
                            Duration::from_secs(15),
                            |r| !*r,
                        )
                        .unwrap();
                    if !*released {
                        return Err(DeliveryError::Limit("test coordination timeout"));
                    }
                }
                if check() {
                    return Err(DeliveryError::Cancelled);
                }
                emit(ordinal, image)
            })
    }
}
fn job(response: HostResponse) -> JobInfo {
    match response {
        HostResponse::Accepted { job, .. }
        | HostResponse::Succeeded {
            result: HostResult::Job { job },
        }
        | HostResponse::Failed { job: Some(job), .. } => *job,
        other => panic!("expected job: {other:?}"),
    }
}
fn get(control: &mut NativeSession<'_>, id: &JobId) -> JobInfo {
    job(control.dispatch(HostRequest::GetJob { job_id: id.clone() }))
}
fn wait_terminal(control: &mut NativeSession<'_>, id: &JobId) -> JobInfo {
    let until = Instant::now() + Duration::from_secs(15);
    loop {
        let job = get(control, id);
        if job.state.terminal() {
            return job;
        }
        assert!(Instant::now() < until, "export failed to finish");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
#[ignore = "requires pinned MO_DELIVERY_WORKER and SHA256"]
fn busy_real_renderer_keeps_control_available_and_sync_inside_worker_budget() {
    let db = Database::new();
    let (snapshot, settings) = setup(&mut db.host());
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let gate = Arc::new(Gate {
        entered: AtomicBool::new(false),
        ready: ready_tx,
        released: Mutex::new(false),
        changed: Condvar::new(),
    });
    let factory_gate = gate.clone();
    let config = StandardHostConfig::new(
        db.path(),
        Digest::from_sha256([1; 32]),
        HostLimits::default(),
    )
    .with_preview_renderer(move || {
        Ok(Box::new(GatedRenderer {
            inner: renderer(),
            gate: factory_gate.clone(),
        }))
    });
    let runtime = NativeRuntime::start(
        config,
        context(),
        RuntimeOptions {
            workers: 1,
            idle_poll: Duration::from_millis(10),
            sync_wait: Duration::from_millis(20),
            ..RuntimeOptions::default()
        },
        Arc::new(|| time(1000)),
    )
    .unwrap();
    let mut control = runtime.connect().unwrap();
    let first = job(control.dispatch(HostRequest::Submit {
        request: Box::new(request(&snapshot, &settings, "running")),
    }));
    ready_rx.recv_timeout(Duration::from_secs(15)).unwrap();
    assert_eq!(get(&mut control, &first.id).state, JobState::Running);
    assert!(db.count("SELECT count(*) FROM result_spools") > 0);
    assert_eq!(db.count("SELECT count(*) FROM result_assets"), 0);
    let second = job(control.dispatch(HostRequest::Submit {
        request: Box::new(request(&snapshot, &settings, "queued")),
    }));
    assert_eq!(second.state, JobState::Queued);
    let mut sync = request(&snapshot, &settings, "sync");
    sync.output_mode = OutputMode::Sync;
    let third = job(control.dispatch(HostRequest::Submit {
        request: Box::new(sync),
    }));
    // A bypass would render and succeed on the control connection here.
    assert_eq!(third.state, JobState::Queued);
    assert_eq!(
        db.count("SELECT count(*) FROM jobs WHERE json_extract(info,'$.state')='running'"),
        1
    );
    let cancel = job(control.dispatch(HostRequest::CancelJob {
        job_id: first.id.clone(),
    }));
    assert!(cancel.cancel_requested);
    assert_eq!(cancel.state, JobState::Running);
    assert_eq!(
        job(control.dispatch(HostRequest::CancelJob {
            job_id: second.id.clone()
        }))
        .state,
        JobState::Cancelled
    );
    gate.release();
    assert_eq!(
        wait_terminal(&mut control, &first.id).state,
        JobState::Cancelled
    );
    let completed = wait_terminal(&mut control, &third.id);
    assert_eq!(completed.state, JobState::Succeeded, "{completed:?}");
    drop(control);
    runtime.shutdown().unwrap();
    verify_assets(&db.host(), receipt(&completed));
    assert_eq!(db.count("SELECT count(*) FROM result_assets"), 12);
    assert_eq!(
        db.count("SELECT count(*) FROM result_spools WHERE expires_at IS NOT NULL"),
        0
    );
}
