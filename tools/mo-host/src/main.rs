//! Operator-configured local host. Authority and storage paths are process
//! configuration; none are accepted inside operation JSON.
use mo_common::{ByteLength, Digest};
use mo_opc::ReaderAt;
use mo_operation_service::*;
use mo_standard_host::{HostLimits, NativeRuntime, RuntimeOptions, StandardHostConfig};
use sha2::{Digest as _, Sha256};
use std::{
    fs::File,
    io::{self, BufRead, Read, Write},
    path::PathBuf,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
fn now() -> UnixMillis {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_millis();
    UnixMillis::new(i64::try_from(millis).expect("host clock overflow")).expect("nonnegative clock")
}
fn executor() -> Result<Digest, Box<dyn std::error::Error>> {
    let mut f = File::open(std::env::current_exe()?)?;
    let mut hash = Sha256::new();
    let mut bytes = [0; 65536];
    loop {
        let n = f.read(&mut bytes)?;
        if n == 0 {
            break;
        }
        hash.update(&bytes[..n]);
    }
    Ok(Digest::from_sha256(hash.finalize().into()))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let db = PathBuf::from(
        args.next()
            .ok_or("usage: mo-host <operator-database> <principal> <scope> [--preview-worker <path> <sha256>] [--scheduled <workers> | run <job-id> | append <upload-id> <offset> | read-asset <asset-id> <offset> <length>]")?,
    );
    let principal = PrincipalId::new(args.next().ok_or("missing operator principal")?)?;
    let scope = ScopeId::new(args.next().ok_or("missing operator scope")?)?;
    let context = CallContext {
        principal,
        scope,
        permissions: [
            Permission::Create,
            Permission::Edit,
            Permission::Export,
            Permission::ReadDocument,
            Permission::ReadJob,
            Permission::CancelJob,
            Permission::WriteAssets,
            Permission::ReadAssets,
        ]
        .into_iter()
        .collect(),
    };
    let mut config = StandardHostConfig::new(db, executor()?, HostLimits::default());
    let mut rest: Vec<_> = args.collect();
    let mut scheduled = None;
    let mut renderer_configured = false;
    while rest.first().is_some_and(|s| s.starts_with("--")) {
        match rest[0].as_str() {
            "--preview-worker" => {
                if rest.len() < 3 || renderer_configured {
                    return Err("--preview-worker requires one operator path and SHA256".into());
                }
                let path = PathBuf::from(&rest[1]);
                let digest = Digest::try_from(rest[2].clone())?;
                config = config.with_preview_renderer(move || {
                    Ok(Box::new(
                        mo_native_render::NativePreviewRenderer::new(
                            path.clone(),
                            digest.clone(),
                            Duration::from_secs(60),
                        )
                        .map_err(delivery_failure)?,
                    ))
                });
                renderer_configured = true;
                rest.drain(..3);
            }
            "--scheduled" => {
                if rest.len() < 2 || scheduled.is_some() {
                    return Err("--scheduled requires one worker count".into());
                }
                scheduled = Some(rest[1].parse::<usize>()?);
                rest.drain(..2);
            }
            _ => return Err("unknown operator option".into()),
        }
    }
    if let Some(workers) = scheduled {
        if !rest.is_empty() {
            return Err("--scheduled is a persistent NDJSON control session".into());
        }
        let runtime = NativeRuntime::start(
            config,
            context,
            RuntimeOptions {
                workers,
                ..RuntimeOptions::default()
            },
            Arc::new(now),
        )?;
        let mut session = runtime.connect()?;
        let result = read_commands(|input| session.dispatch_json(input));
        drop(session);
        let stopped = runtime.shutdown();
        result?;
        stopped?;
        return Ok(());
    }
    let mut host = config.connect()?;
    let mut out = io::stdout().lock();
    match rest.as_slice() {
        [] => {
            drop(out);
            read_commands(|input| host.dispatch_json(&context, input, &now, &|| false))?;
        }
        [command, id] if command == "run" => {
            let id = JobId::new(id)?;
            let result = host.run_job(&context, &id, now(), &now, &|| false);
            let response = match result {
                Ok(job) => HostResponse::job(job),
                Err(error) => HostResponse::Failed { error, job: None },
            };
            writeln!(out, "{}", serde_json::to_string(&response)?)?;
        }
        [command, id, offset] if command == "append" => {
            let id = UploadId::new(id)?;
            let offset = ByteLength::try_from(offset.clone())?;
            let mut bytes = Vec::new();
            io::stdin()
                .lock()
                .take((ASSET_CHUNK_BYTES + 1) as u64)
                .read_to_end(&mut bytes)?;
            let result = host.append_upload(&context, &id, offset, &bytes, now());
            let response = match result {
                Ok(upload) => HostResponse::upload(upload),
                Err(error) => HostResponse::Failed { error, job: None },
            };
            writeln!(out, "{}", serde_json::to_string(&response)?)?;
        }
        [command, id, offset, length] if command == "read-asset" => {
            let id = AssetId::new(id)?;
            let offset = ByteLength::try_from(offset.clone())?.get();
            let length = ByteLength::try_from(length.clone())?.get();
            let reader = host.open_asset(&context, &id)?;
            if offset
                .checked_add(length)
                .is_none_or(|end| end > reader.info().descriptor.byte_length.get())
            {
                return Err("requested range exceeds resource".into());
            }
            // Binary response goes to stdout; failure exits nonzero on stderr.
            // Receivers must discard partial data on failure and verify the
            // declared full-resource digest before treating a download as sealed.
            let mut buf = vec![0; ASSET_CHUNK_BYTES];
            let mut copied = 0;
            while copied < length {
                let n = (length - copied).min(buf.len() as u64) as usize;
                reader.read_exact_at(&mut buf[..n], offset + copied)?;
                out.write_all(&buf[..n])?;
                copied += n as u64;
            }
            out.flush()?;
        }
        _ => return Err("unknown operator command".into()),
    }
    Ok(())
}

fn read_commands(
    mut dispatch: impl FnMut(&str) -> String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut input = io::stdin().lock();
    let mut out = io::stdout().lock();
    loop {
        let mut bytes = Vec::new();
        let n = (&mut input)
            .take((MAX_OPERATION_BYTES + 2) as u64)
            .read_until(b'\n', &mut bytes)?;
        if n == 0 {
            break;
        }
        if bytes.len() > MAX_OPERATION_BYTES + 1 {
            return Err("operation line exceeds byte budget".into());
        }
        if bytes.last() == Some(&b'\n') {
            bytes.pop();
        }
        let response = dispatch(std::str::from_utf8(&bytes)?);
        writeln!(out, "{response}")?;
        out.flush()?;
    }
    Ok(())
}
