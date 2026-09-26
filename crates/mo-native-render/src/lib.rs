//! Isolated native previews from explicit bytes. Only the trusted operator
//! configures the worker executable; no path enters core or public arguments.
use mo_common::Digest;
use mo_kernel_api::{
    PptxResourceDocumentRequest, PptxResourcePageProfile, PptxResourcePageRasterResponse,
};
use mo_presentation_compile::source_resource_page::SourceResourcePageImage;
use mo_presentation_delivery::{
    Content, DeliveryError, PreviewFonts, PreviewRenderer, PreviewRequest, RendererIdentity,
};
use sha2::{Digest as _, Sha256};
use std::{io::Read, path::PathBuf, process::Command, time::Duration};

pub struct NativePreviewRenderer {
    executable: PathBuf,
    timeout: Duration,
    identity: RendererIdentity,
}
impl NativePreviewRenderer {
    /// `timeout` bounds a complete document batch, including input transfer and
    /// output consumption. It is not renewed for each page.
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
    fn render_pages(
        &mut self,
        requests: &[PreviewRequest],
        source: Content<'_>,
        fonts: PreviewFonts<'_>,
        check: &dyn Fn() -> bool,
        emit: &mut dyn FnMut(usize, SourceResourcePageImage) -> Result<(), DeliveryError>,
    ) -> Result<(), DeliveryError> {
        if requests.is_empty() {
            return Ok(());
        }
        if requests.len() > 256 {
            return Err(DeliveryError::Limit("preview page count"));
        }
        if executable_digest(&self.executable)? != self.identity.implementation_sha256 {
            return Err(DeliveryError::Invalid("preview worker executable changed"));
        }
        if source.byte_length > mo_kernel_api::MAX_INLINE_RESOURCE_BYTES as u64
            || fonts.content.byte_length > mo_kernel_api::MAX_INLINE_FONT_BYTES as u64
        {
            return Err(DeliveryError::Limit("preview worker input bytes"));
        }
        let batch = PptxResourceDocumentRequest {
            profile: PptxResourcePageProfile::NativeResourcesDraftV1,
            pages: requests.to_vec(),
            fonts: fonts.manifest.cloned(),
        };
        let json = request_json(&batch)?;
        let mut header = Vec::new();
        header.extend_from_slice(&(json.len() as u32).to_le_bytes());
        header.extend_from_slice(&(source.byte_length as u32).to_le_bytes());
        header.extend_from_slice(&(fonts.content.byte_length as u32).to_le_bytes());
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
            fonts.content,
        ];
        let mut part = 0;
        let mut offset = 0;
        let mut command = Command::new(&self.executable);
        command.arg("--preview-document");
        let count = requests.len();
        let mut consumer_error = None;
        let result = mo_native_worker::exchange_events(
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
            move |stdout, send| {
                for ordinal in 0..count {
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
                    send((ordinal, response, pixels))?;
                }
                Ok(())
            },
            |(ordinal, response, pixels)| {
                let image = response_image(response, pixels);
                if let Err(error) = image.and_then(|image| emit(ordinal, image)) {
                    consumer_error = Some(error);
                    return Err("preview output rejected".into());
                }
                Ok(())
            },
        );
        if let Some(error) = consumer_error {
            return Err(error);
        }
        if check() {
            return Err(DeliveryError::Cancelled);
        }
        result.map_err(|message| DeliveryError::Preview {
            message,
            diagnostic: None,
        })?;
        Ok(())
    }
}

fn request_json(request: &PptxResourceDocumentRequest) -> Result<Vec<u8>, DeliveryError> {
    struct Bounded(Vec<u8>);
    impl std::io::Write for Bounded {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > mo_kernel_api::MAX_REQUEST_BYTES.saturating_sub(self.0.len()) {
                return Err(std::io::Error::other("preview request byte limit"));
            }
            self.0
                .try_reserve(bytes.len())
                .map_err(std::io::Error::other)?;
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut output = Bounded(Vec::new());
    serde_json::to_writer(&mut output, request)
        .map_err(|_| DeliveryError::Limit("preview worker request bytes"))?;
    Ok(output.0)
}
fn response_image(
    response: PptxResourcePageRasterResponse,
    pixels: Vec<u8>,
) -> Result<SourceResourcePageImage, DeliveryError> {
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

#[cfg(test)]
mod tests;
