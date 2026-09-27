//! Direct native computation. All paths are explicit CLI inputs chosen by the
//! caller; they never enter the document operation or grant product authority.
use mo_embedded_sdk::{NativeExporter, common::Digest};
use std::{error::Error, ffi::OsString, path::Path, time::Duration};

pub fn failure_json(error: Box<dyn Error>) -> String {
    use mo_embedded_sdk::{
        delivery::DeliveryError,
        operation::{Failure, FailureCode, delivery_failure},
    };
    fn classify(error: Box<dyn Error>) -> Failure {
        let error = match error.downcast::<Failure>() {
            Ok(failure) => return *failure,
            Err(error) => error,
        };
        let error = match error.downcast::<DeliveryError>() {
            Ok(failure) => return delivery_failure(*failure),
            Err(error) => error,
        };
        let code = if error.is::<std::io::Error>() {
            FailureCode::IoFailure
        } else {
            FailureCode::InputInvalid
        };
        Failure::new(code, error.to_string())
    }
    serde_json::json!({"outcome":"failed", "error":classify(error)}).to_string()
}

pub fn run(args: &[OsString]) -> Result<String, Box<dyn Error>> {
    let (paths, worker) = match args {
        [request, inputs, spool, output] => ([request, inputs, spool, output], None),
        [request, inputs, spool, output, worker, digest] => {
            ([request, inputs, spool, output], Some((worker, digest)))
        },
        _ => return Err("usage: mo-cli compute <invocation.json> <inputs.json> <protected-spool-directory> <new-output-directory> [worker worker-sha256]".into()),
    };
    let exporter = worker
        .map(|(path, hash)| {
            let hash = hash.to_str().ok_or("worker digest must be UTF-8")?;
            Ok::<_, Box<dyn Error>>(NativeExporter::new(
                path.into(),
                Digest::try_from(hash.to_owned())?,
                paths[2].into(),
                Duration::from_secs(60),
            )?)
        })
        .transpose()?;
    let input_path = Path::new(paths[1]);
    let base = input_path.parent().unwrap_or(Path::new("."));
    let response = mo_native_compute::compute(
        mo_native_compute::FileCall {
            invocation: std::fs::File::open(paths[0])?,
            inputs: std::fs::File::open(input_path)?,
            temporary_directory: Path::new(paths[2]),
            output_directory: Path::new(paths[3]),
            exporter: exporter.as_ref(),
        },
        &|name| std::fs::File::open(base.join(name)),
        &|| false,
    )?;
    Ok(serde_json::to_string(&response)?)
}
