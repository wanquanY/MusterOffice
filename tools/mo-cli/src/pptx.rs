use super::artifact::{self, read_limited};
use std::{error::Error, ffi::OsStr, fs::File, path::Path};

/// Local development host: stage, sync, actually read back, then publish without
/// overwriting an existing path. No partial output is placed at the requested name.
pub fn export(request: &OsStr, bundle: &OsStr, output: &OsStr) -> Result<String, Box<dyn Error>> {
    let request = read_limited(request, mo_kernel_api::MAX_REQUEST_BYTES)?;
    let bundle = File::open(bundle)?;
    let metadata = bundle.metadata()?;
    if !metadata.is_file() {
        return Err("input must be a regular file".into());
    }
    let destination = Path::new(output);
    let directory = artifact::output_directory(destination);
    let sink = mo_native_io::FileSpool::create(
        directory,
        mo_opc::PackageLimits::default().max_package_bytes,
    )?;
    let verified = mo_kernel_api::export_pptx_json_to(
        std::str::from_utf8(&request)?,
        &rawzip::FileReader::from(bundle),
        metadata.len(),
        sink,
        &|| false,
    )?;
    let response = mo_kernel_api::package_response_json(
        &mo_kernel_api::PackageInspectionResponse::Inspected {
            report: mo_kernel_api::PackageReport::from_package(verified.package()),
        },
    );
    verified.reader().link_new(destination)?;
    Ok(response)
}

pub enum EditMode {
    Text,
    Transforms,
}
pub fn edit(
    request: &OsStr,
    source: &OsStr,
    output: &OsStr,
    mode: EditMode,
) -> Result<String, Box<dyn Error>> {
    let request = read_limited(request, mo_kernel_api::MAX_REQUEST_BYTES)?;
    let source = read_limited(source, mo_kernel_api::MAX_INLINE_RESOURCE_BYTES)?;
    let execute = match mode {
        EditMode::Text => mo_kernel_api::edit_pptx_text_json,
        EditMode::Transforms => mo_kernel_api::edit_pptx_transforms_json,
    };
    let bytes = execute(std::str::from_utf8(&request)?, &source)?;
    publish(&bytes, output)
}

fn publish(bytes: &[u8], output: &OsStr) -> Result<String, Box<dyn Error>> {
    let expected = mo_kernel_api::inspect_package_json(bytes);
    let mut actual = String::new();
    artifact::publish(bytes, output, |file| {
        let length = file.metadata()?.len();
        let report = mo_kernel_api::inspect_package(
            rawzip::FileReader::from(file),
            length,
            mo_opc::PackageLimits::default(),
            &|| false,
        );
        actual = mo_kernel_api::package_response_json(&report);
        if actual != expected {
            return Err("staged PPTX differs from computed output".into());
        }
        Ok(())
    })?;
    Ok(actual)
}
