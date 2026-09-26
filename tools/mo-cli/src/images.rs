//! Explicit local host for encoded image bundles; core has no path authority.
use super::artifact;
use sha2::{Digest as _, Sha256};
use std::{error::Error, ffi::OsStr, io::Read};

pub fn extract(request: &OsStr, source: &OsStr, output: &OsStr) -> Result<String, Box<dyn Error>> {
    let request = artifact::read_limited(request, mo_kernel_api::MAX_REQUEST_BYTES)?;
    let source = artifact::read_limited(source, mo_kernel_api::MAX_INLINE_RESOURCE_BYTES)?;
    let (metadata, bytes) =
        mo_kernel_api::extract_pptx_images_json(std::str::from_utf8(&request)?, &source);
    let response = mo_common::from_json_str::<mo_kernel_api::PptxImagesResponse>(&metadata)?;
    if matches!(response, mo_kernel_api::PptxImagesResponse::Error { .. }) {
        return Ok(metadata);
    }
    let expected = Sha256::digest(&bytes);
    artifact::publish(&bytes, output, |mut file| {
        if file.metadata()?.len() != bytes.len() as u64 {
            return Err("staged image bundle length".into());
        }
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 16384];
        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hash.update(&buffer[..n]);
        }
        if hash.finalize() != expected {
            return Err("staged image bundle digest".into());
        }
        Ok(())
    })?;
    Ok(metadata)
}
