use super::*;
use mo_presentation_compile::source_resource_page::protocol::{
    PptxResourcePageProfile, PptxResourcePageRequest, ResourcePageRasterResponse,
};
type PptxResourcePageRasterResponse = ResourcePageRasterResponse<serde_json::Value>;
use mo_opc::ReaderAt;
use std::{io::Read, process::Command};

struct Counted<'a> {
    reader: &'a dyn ReaderAt,
    bytes: Cell<u64>,
}
impl ReaderAt for Counted<'_> {
    fn read_at(&self, bytes: &mut [u8], offset: u64) -> io::Result<usize> {
        let n = self.reader.read_at(bytes, offset)?;
        self.bytes.set(self.bytes.get() + n as u64);
        Ok(n)
    }
}
fn bytes(content: &Content<'_>) -> Vec<u8> {
    let mut bytes = vec![0; content.byte_length as usize];
    content.reader.read_exact_at(&mut bytes, 0).unwrap();
    bytes
}
fn legacy(
    request: &PreviewRequest,
    source: &Content<'_>,
    fonts: &PreviewFonts<'_>,
) -> SourceResourcePageImage {
    let request = PptxResourcePageRequest {
        profile: PptxResourcePageProfile::NativeResourcesDraftV1,
        page: request.page.clone(),
        image_source: request.image_source,
        sampling: request.sampling,
        fonts: fonts.manifest.cloned(),
    };
    let json = serde_json::to_vec(&request).unwrap();
    let mut header = Vec::new();
    for n in [
        json.len() as u32,
        source.byte_length as u32,
        fonts.content.byte_length as u32,
    ] {
        header.extend_from_slice(&n.to_le_bytes());
    }
    let mut command = Command::new(std::env::var_os("MO_DELIVERY_WORKER").unwrap());
    command.arg("--pptx-resource-page");
    mo_native_worker::exchange(
        command,
        vec![header, json, bytes(source), bytes(&fonts.content)],
        Duration::from_secs(60),
        |stdout| {
            let mut lengths = [0; 8];
            stdout.read_exact(&mut lengths).map_err(|e| e.to_string())?;
            let mut json = vec![0; u32::from_le_bytes(lengths[..4].try_into().unwrap()) as usize];
            let mut pixels = vec![0; u32::from_le_bytes(lengths[4..].try_into().unwrap()) as usize];
            stdout.read_exact(&mut json).map_err(|e| e.to_string())?;
            stdout.read_exact(&mut pixels).map_err(|e| e.to_string())?;
            match serde_json::from_slice(&json).map_err(|e| e.to_string())? {
                PptxResourcePageRasterResponse::Rendered { info } => Ok(SourceResourcePageImage {
                    info: *info,
                    pixels,
                }),
                PptxResourcePageRasterResponse::Error { .. } => Err("legacy page failed".into()),
            }
        },
    )
    .unwrap()
}
struct Compared {
    inner: NativePreviewRenderer,
    batches: usize,
    pages: usize,
}
impl PreviewRenderer for Compared {
    fn identity(&self) -> RendererIdentity {
        self.inner.identity()
    }
    fn render_pages(
        &mut self,
        requests: &[PreviewRequest],
        source: Content<'_>,
        fonts: PreviewFonts<'_>,
        check: &dyn Fn() -> bool,
        emit: &mut dyn FnMut(usize, SourceResourcePageImage) -> Result<(), DeliveryError>,
    ) -> Result<(), DeliveryError> {
        self.batches += 1;
        let expected: Vec<_> = requests
            .iter()
            .map(|r| legacy(r, &source, &fonts))
            .collect();
        let counted_source = Counted {
            reader: source.reader,
            bytes: Cell::new(0),
        };
        let counted_fonts = Counted {
            reader: fonts.content.reader,
            bytes: Cell::new(0),
        };
        let pages = &mut self.pages;
        self.inner.render_pages(
            requests,
            Content {
                reader: &counted_source,
                byte_length: source.byte_length,
            },
            PreviewFonts {
                manifest: fonts.manifest,
                content: Content {
                    reader: &counted_fonts,
                    byte_length: fonts.content.byte_length,
                },
            },
            check,
            &mut |ordinal, image| {
                *pages += 1;
                assert_eq!(image.pixels, expected[ordinal].pixels);
                assert_eq!(
                    serde_json::to_value(&image.info).unwrap(),
                    serde_json::to_value(&expected[ordinal].info).unwrap()
                );
                emit(ordinal, image)
            },
        )?;
        assert_eq!(counted_source.bytes.get(), source.byte_length);
        assert_eq!(counted_fonts.bytes.get(), fonts.content.byte_length);
        Ok(())
    }
}

#[test]
#[ignore = "requires pinned MO_DELIVERY_WORKER and SHA256"]
fn multi_page_batch_transfers_resources_once_and_matches_individual_page_rendering() {
    let (snapshot, settings) = input();
    let mut renderer = Compared {
        inner: renderer(),
        batches: 0,
        pages: 0,
    };
    let candidate = run(
        snapshot,
        &settings,
        &mut Store::default(),
        &mut renderer,
        DeliveryLimits::default(),
    )
    .unwrap();
    assert_eq!(candidate.bundle().previews.len(), 2);
    assert_eq!((renderer.batches, renderer.pages), (1, 2));
}
