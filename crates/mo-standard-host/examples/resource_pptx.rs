//! Verification example only: exercises the existing Writer with sealed resource
//! readers. It does not publish a public bundle or a successful export job.
use mo_common::{ByteLength, Digest, RequestId, ResourceId};
use mo_operation_service::*;
use mo_pptx::{ExportDefaults, PptxLimits};
use mo_presentation_model::Document;
use mo_standard_host::{HostLimits, StandardHost};
use sha2::{Digest as _, Sha256};
use std::{fs::OpenOptions, io::Write, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(std::env::args().nth(1).ok_or("output directory required")?);
    std::fs::create_dir(&root)?;
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/native-export/request.json"
    ))?;
    let document: Document = serde_json::from_value(fixture["document"].clone())?;
    let defaults: ExportDefaults = serde_json::from_value(fixture["defaults"].clone())?;
    let bytes = include_bytes!("../../../fixtures/presentations/native-export/resources.bin");
    let context = CallContext {
        principal: PrincipalId::new("fixture:agent")?,
        scope: ScopeId::new("fixture:scope")?,
        permissions: [Permission::WriteAssets, Permission::ReadAssets]
            .into_iter()
            .collect(),
    };
    let mut host = StandardHost::open(
        root.join("host.sqlite"),
        Digest::from_sha256([0; 32]),
        HostLimits::default(),
    )?;
    let mut bindings = Vec::new();
    let mut assets = Vec::new();
    for b in fixture["resourceBindings"]
        .as_array()
        .ok_or("bindings required")?
    {
        let id = ResourceId::new(b["resourceId"].as_str().ok_or("resource id required")?)?;
        let offset = b["byteOffset"]
            .as_str()
            .ok_or("offset required")?
            .parse::<usize>()?;
        let length = b["byteLength"]
            .as_str()
            .ok_or("length required")?
            .parse::<usize>()?;
        let resource = &document.resources[&id];
        let request = UploadRequest {
            request_id: RequestId::new(id.as_str())?,
            descriptor: AssetDescriptor {
                sha256: resource.sha256.clone(),
                byte_length: ByteLength::new(length as u64),
                media_type: resource.media_type.clone(),
            },
        };
        let upload = host.begin_upload(&context, request, UnixMillis::new(10)?)?;
        for (i, chunk) in bytes[offset..offset + length]
            .chunks(ASSET_CHUNK_BYTES)
            .enumerate()
        {
            host.append_upload(
                &context,
                &upload.id,
                ByteLength::new((i * ASSET_CHUNK_BYTES) as u64),
                chunk,
                UnixMillis::new(20)?,
            )?;
        }
        let sealed = host.seal_upload(
            &context,
            &upload.id,
            &|| UnixMillis::new(30).unwrap(),
            &|| false,
        )?;
        let asset = sealed.asset.ok_or("resource not sealed")?;
        bindings.push(AssetBinding {
            resource_id: id,
            asset_id: asset.id.clone(),
        });
        assets.push(asset);
    }
    drop(host);
    let host = StandardHost::open(
        root.join("host.sqlite"),
        Digest::from_sha256([0; 32]),
        HostLimits::default(),
    )?;
    let resources = host.bind_resources(&context, &document, &bindings)?;
    let output = mo_pptx::export(
        &document,
        &defaults,
        &resources,
        PptxLimits::default(),
        &|| false,
    )?;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join("resource-backed.pptx"))?
        .write_all(&output)?;
    let report = serde_json::json!({"format":"musteroffice.resource-backed-pptx/1","documentSemanticDigest":document.semantic_digest()?,"bindings":bindings,"assets":assets,"pptxSha256":format!("{:x}",Sha256::digest(&output)),"pptxBytes":output.len(),"claim":"Library resource-provider integration only; not a public export job or product bundle."});
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join("result.json"))?
        .write_all(serde_json::to_string_pretty(&report)?.as_bytes())?;
    println!("{}", report);
    Ok(())
}
