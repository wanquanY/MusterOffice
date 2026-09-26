use super::*;
use mo_common::{Digest, DocumentId, Emu, RequestId};
use mo_presentation_model::{Document, Size};
use std::cell::Cell;

thread_local! { static READS: Cell<usize> = const { Cell::new(0) }; }

#[test]
fn checkpoints_bound_storage_reads_and_observe_cross_connection_cancel_with_frozen_clock() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory =
        std::env::temp_dir().join(format!("mo-checkpoints-{}-{nonce}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("host.sqlite");
    let mut host = StandardHost::open(
        &path,
        Digest::from_sha256([1; 32]),
        crate::HostLimits::default(),
    )
    .unwrap();
    let mut other = StandardHost::open(
        &path,
        Digest::from_sha256([1; 32]),
        crate::HostLimits::default(),
    )
    .unwrap();
    let context = CallContext {
        principal: PrincipalId::new("agent").unwrap(),
        scope: ScopeId::new("scope").unwrap(),
        permissions: [Permission::Create, Permission::CancelJob]
            .into_iter()
            .collect(),
    };
    let now = UnixMillis::new(1000).unwrap();
    let request = OperationRequest {
        contract_version: ContractVersion::V1,
        request_id: RequestId::new("create").unwrap(),
        profile_id: OperationProfile::AuthorModel,
        output_mode: OutputMode::Job,
        action: DocumentAction::Create {
            document: Box::new(Document::empty(
                DocumentId::new("doc").unwrap(),
                Size {
                    width: Emu::new(914400),
                    height: Emu::new(914400),
                },
            )),
        },
    };
    let accepted = host.submit(&context, request, now).unwrap();
    let work = host.claim(&context, &accepted.id, now).unwrap().unwrap();
    host.connection.trace_v2(
        rusqlite::trace::TraceEventCodes::SQLITE_TRACE_STMT,
        Some(|event| {
            if let rusqlite::trace::TraceEvent::Stmt(_, sql) = event
                && sql.starts_with("SELECT j.info")
            {
                READS.with(|n| n.set(n.get() + 1));
            }
        }),
    );
    READS.with(|n| n.set(0));
    let clock = || now;
    let monitor = ExecutionCheck::new(&host, &context, &work.lease, &clock, &|| false).unwrap();
    for _ in 0..1000 {
        assert!(!monitor.cancelled());
    }
    let reads = READS.with(Cell::get);
    assert!(
        reads < 20,
        "1000 cheap checkpoints issued {reads} persistent reads"
    );
    other.cancel_job(&context, &accepted.id, now).unwrap();
    // The host clock is deliberately frozen: monotonic time must still drive
    // external cancellation observation instead of caching the lease forever.
    std::thread::sleep(Duration::from_millis(110));
    assert!(monitor.cancelled());
    assert_eq!(
        monitor.finish(Ok(())).unwrap_err().code,
        FailureCode::Cancelled
    );
    drop(other);
    drop(host);
    std::fs::remove_dir_all(directory).unwrap();
}
