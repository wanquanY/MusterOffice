//! Verification of private output ownership using a COPY of the resource_pptx
//! example's database and its binding report. Not a public export operation.
use mo_common::{Digest, RequestId};
use mo_opc::ReaderAt;
use mo_operation_service::*;
use mo_standard_host::{HostLimits, ResultSpec, StandardHost};
use std::{fs::OpenOptions, io::Write, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("prepared output directory required")?,
    );
    let mut host = StandardHost::open(
        root.join("host.sqlite"),
        Digest::from_sha256([0; 32]),
        HostLimits::default(),
    )?;
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/native-export/request.json"
    ))?;
    let document: mo_presentation_model::Document =
        serde_json::from_value(fixture["document"].clone())?;
    let defaults = serde_json::from_value(fixture["defaults"].clone())?;
    let source: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("input-bindings.json"))?)?;
    let bindings: Vec<AssetBinding> = serde_json::from_value(source["bindings"].clone())?;
    let context = CallContext {
        principal: PrincipalId::new("fixture:agent")?,
        scope: ScopeId::new("fixture:scope")?,
        permissions: [
            Permission::Create,
            Permission::ReadAssets,
            Permission::ReadJob,
        ]
        .into_iter()
        .collect(),
    };
    let job = host.submit(
        &context,
        OperationRequest {
            contract_version: ContractVersion::V1,
            request_id: RequestId::new("output-owner-verification")?,
            profile_id: OperationProfile::AuthorModel,
            output_mode: OutputMode::Job,
            action: DocumentAction::Create {
                document: Box::new(document.clone()),
            },
        },
        UnixMillis::new(40)?,
    )?;
    let work = host
        .claim(&context, &job.id, UnixMillis::new(41)?)?
        .ok_or("fresh execution required")?;
    let clock = || UnixMillis::new(50).unwrap();
    let resources = host.bind_resources(&context, &document, &bindings)?;
    let sink = host.create_result(
        &context,
        &work.lease,
        ResultSpec {
            name: RequestId::new("presentation")?,
            media_type: "application/vnd.openxmlformats-officedocument.presentationml.presentation"
                .into(),
            max_bytes: 1_000_000,
        },
        &clock,
    )?;
    let verified = mo_pptx::export_to(
        &document,
        &defaults,
        &resources,
        sink,
        mo_pptx::PptxLimits::default(),
        &|| false,
    )?;
    let receipt = verified.receipt().clone();
    drop(verified);
    drop(resources);
    drop(host);
    let mut reopened = StandardHost::open(
        root.join("host.sqlite"),
        Digest::from_sha256([0; 32]),
        HostLimits::default(),
    )?;
    let reader = reopened.open_result(
        &context,
        &work.lease,
        &RequestId::new("presentation")?,
        &clock,
    )?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(root.join("diagnostic-copy.pptx"))?;
    let mut buffer = vec![0; 64 * 1024];
    let mut offset = 0;
    while offset < reader.byte_length() {
        let n = reader.read_at(&mut buffer, offset)?;
        if n == 0 {
            return Err("unexpected stored end".into());
        }
        file.write_all(&buffer[..n])?;
        offset += n as u64;
    }
    file.sync_all()?;
    let sha256 = reader.sha256().clone();
    drop(reader);
    let job = reopened.get_job(&context, &job.id, UnixMillis::new(51)?)?;
    if job.state != JobState::Running {
        return Err("candidate must not finish job".into());
    }
    let report = serde_json::json!({"format":"musteroffice.private-job-output/1","job":job,"writeReceipt":{"sha256":receipt.sha256,"byteLength":receipt.byte_length},"storedSha256":sha256,"storedBytes":offset,"bindings":bindings,"claim":"Private job-owned library candidate; diagnostic copy only, no public asset, delivery bundle or successful export operation."});
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(root.join("result.json"))?
        .write_all(serde_json::to_string_pretty(&report)?.as_bytes())?;
    println!("{}", report);
    Ok(())
}
