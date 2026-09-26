//! Isolated native previews from explicit bytes. Only the trusted operator
//! configures the worker executable; no path enters core or public arguments.
use mo_common::Digest;
use mo_kernel_api::{
    PptxResourcePageProfile, PptxResourcePageRasterResponse, PptxResourcePageRequest,
};
use mo_presentation_compile::source_resource_page::SourceResourcePageImage;
use mo_presentation_delivery::{
    Content, DeliveryError, PreviewRenderer, PreviewRequest, RendererIdentity,
};
use sha2::{Digest as _, Sha256};
use std::{io::Read, path::PathBuf, process::Command, time::Duration};

pub struct NativePreviewRenderer {
    executable: PathBuf,
    timeout: Duration,
    identity: RendererIdentity,
}
impl NativePreviewRenderer {
    pub fn new(
        executable: PathBuf,
        expected_sha256: Digest,
        timeout: Duration,
    ) -> Result<Self, DeliveryError> {
        if timeout.is_zero() || timeout > Duration::from_secs(3600) {
            return Err(DeliveryError::Invalid("preview worker deadline"));
        }
        if executable_digest(&executable)? != expected_sha256 {
            return Err(DeliveryError::Invalid("preview worker executable digest"));
        }
        Ok(Self {
            executable,
            timeout,
            identity: RendererIdentity {
                implementation_sha256: expected_sha256,
                profile: mo_presentation_compile::source_resource_page::PROFILE.into(),
            },
        })
    }
}
impl PreviewRenderer for NativePreviewRenderer {
    fn identity(&self) -> RendererIdentity {
        self.identity.clone()
    }
    fn render(
        &mut self,
        request: &PreviewRequest,
        source: Content<'_>,
        fonts: Content<'_>,
        check: &dyn Fn() -> bool,
    ) -> Result<SourceResourcePageImage, DeliveryError> {
        if executable_digest(&self.executable)? != self.identity.implementation_sha256 {
            return Err(DeliveryError::Invalid("preview worker executable changed"));
        }
        if source.byte_length > mo_kernel_api::MAX_INLINE_RESOURCE_BYTES as u64
            || fonts.byte_length > mo_kernel_api::MAX_INLINE_FONT_BYTES as u64
        {
            return Err(DeliveryError::Limit("preview worker input bytes"));
        }
        let request = PptxResourcePageRequest {
            profile: PptxResourcePageProfile::NativeResourcesDraftV1,
            page: request.page.clone(),
            image_source: request.image_source,
            sampling: request.sampling,
            fonts: request.fonts.clone(),
        };
        let json = serde_json::to_vec(&request).map_err(|_| DeliveryError::Serialization)?;
        if json.len() > mo_kernel_api::MAX_REQUEST_BYTES {
            return Err(DeliveryError::Limit("preview worker request bytes"));
        }
        let mut header = Vec::new();
        header.extend_from_slice(&(json.len() as u32).to_le_bytes());
        header.extend_from_slice(&(source.byte_length as u32).to_le_bytes());
        header.extend_from_slice(&(fonts.byte_length as u32).to_le_bytes());
        let parts = [
            Content {
                reader: &header,
                byte_length: header.len() as u64,
            },
            Content {
                reader: &json,
                byte_length: json.len() as u64,
            },
            source,
            fonts,
        ];
        let mut part = 0;
        let mut offset = 0;
        let mut command = Command::new(&self.executable);
        command.arg("--pptx-resource-page");
        let result = mo_native_worker::exchange_stream(
            command,
            self.timeout,
            check,
            || {
                while part < parts.len() && offset == parts[part].byte_length {
                    part += 1;
                    offset = 0;
                }
                if part == parts.len() {
                    return Ok(None);
                }
                let n = (parts[part].byte_length - offset).min(mo_native_worker::CHUNK_BYTES as u64)
                    as usize;
                let mut bytes = vec![0; n];
                parts[part]
                    .reader
                    .read_exact_at(&mut bytes, offset)
                    .map_err(|e| e.to_string())?;
                offset += n as u64;
                Ok(Some(bytes))
            },
            |stdout| {
                let mut lengths = [0; 8];
                stdout.read_exact(&mut lengths).map_err(|e| e.to_string())?;
                let meta = u32::from_le_bytes(lengths[..4].try_into().unwrap()) as usize;
                let pixel = u32::from_le_bytes(lengths[4..].try_into().unwrap()) as usize;
                if meta > 64 * 1024 * 1024 || pixel > mo_raster::MAX_PIXEL_BYTES {
                    return Err("preview worker response budget".into());
                }
                let mut metadata = vec![0; meta];
                let mut pixels = vec![0; pixel];
                stdout
                    .read_exact(&mut metadata)
                    .map_err(|e| e.to_string())?;
                stdout.read_exact(&mut pixels).map_err(|e| e.to_string())?;
                let text = std::str::from_utf8(&metadata).map_err(|e| e.to_string())?;
                let response = mo_common::from_json_str::<PptxResourcePageRasterResponse>(text)
                    .map_err(|e| e.to_string())?;
                Ok((response, pixels))
            },
        );
        if check() {
            return Err(DeliveryError::Cancelled);
        }
        let (response, pixels) = result.map_err(|message| DeliveryError::Preview {
            message,
            diagnostic: None,
        })?;
        match response {
            PptxResourcePageRasterResponse::Rendered { info } => Ok(SourceResourcePageImage {
                info: *info,
                pixels,
            }),
            PptxResourcePageRasterResponse::Error { error } => {
                if !pixels.is_empty() {
                    return Err(DeliveryError::Invalid("failed preview contains pixels"));
                }
                Err(DeliveryError::Preview {
                    message: "native page rendering failed".into(),
                    diagnostic: Some(Box::new(
                        serde_json::to_value(error).map_err(|_| DeliveryError::Serialization)?,
                    )),
                })
            }
        }
    }
}

fn executable_digest(path: &std::path::Path) -> Result<Digest, DeliveryError> {
    let mut file = std::fs::File::open(path)?;
    if file.metadata()?.len() > 128 * 1024 * 1024 {
        return Err(DeliveryError::Limit("preview executable bytes"));
    }
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(Digest::from_sha256(hash.finalize().into()))
}
