//! Thin caller-file adapter; all repair semantics and validation live in OPC.
use std::{error::Error, ffi::OsStr, fs::File, path::Path};

pub fn run(source: &OsStr, apply: Option<(&OsStr, &OsStr)>) -> Result<String, Box<dyn Error>> {
    let file = File::open(source)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err("input must be a regular file".into());
    }
    let limits = mo_opc::PackageLimits::default();
    let repair = mo_opc::ContentTypeRepair::inspect(
        rawzip::FileReader::from(file),
        metadata.len(),
        limits,
        &|| false,
    )?;
    let removed: Vec<_> = repair
        .removed_overrides()
        .iter()
        .map(|(part, media)| serde_json::json!({"partName":part.to_string(),"contentType":media}))
        .collect();
    let mut report = serde_json::json!({
        "format":"musteroffice.opc-content-type-repair/1-draft",
        "sourceSha256":repair.source_sha256(),"removedOverrides":removed,
        "applied":false,
    });
    if let Some((digest, output)) = apply {
        let expected: mo_common::Digest = serde_json::from_value(serde_json::Value::String(
            digest.to_str().ok_or("digest must be UTF-8")?.into(),
        ))?;
        let destination = Path::new(output);
        let sink = mo_native_io::FileSpool::create(
            super::artifact::output_directory(destination),
            limits.max_package_bytes,
        )?;
        let verified = repair.write_sealed(&expected, sink, &|| false)?;
        verified.reader().link_new(destination)?;
        report["applied"] = true.into();
        report["outputSha256"] = serde_json::to_value(&verified.receipt().sha256)?;
        report["outputByteLength"] = verified.receipt().byte_length.into();
    }
    Ok(serde_json::to_string(&report)?)
}
