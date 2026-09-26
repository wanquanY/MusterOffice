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
struct CountedResources<'a>(std::collections::BTreeMap<ResourceId, (Counted<'a>, u64)>);
impl Resources for CountedResources<'_> {
    fn open(&self, id: &ResourceId) -> Result<ResourceData<'_>, PptxError> {
        let (reader, byte_length) = &self.0[id];
        Ok(ResourceData {
            reader,
            byte_length: *byte_length,
        })
    }
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
        input: PreviewInput<'_>,
        fonts: PreviewFonts<'_>,
        check: &dyn Fn() -> bool,
        emit: &mut dyn FnMut(usize, SourceResourcePageImage) -> Result<(), DeliveryError>,
    ) -> Result<(), DeliveryError> {
        self.batches += 1;
        let PreviewInput::Author { plan, resources } = input else {
            panic!("delivery must render author semantics directly");
        };
        // Export is exclusively this test's independent source-render oracle.
        let output =
            mo_pptx::export_plan_to(plan, resources, Vec::new(), Default::default(), &|| false)
                .unwrap();
        let source = Content {
            reader: output.reader(),
            byte_length: output.receipt().byte_length,
        };
        let expected: Vec<_> = requests
            .iter()
            .map(|r| {
                let mut source_request = r.clone();
                source_request.page.expected_source_sha256 = output.receipt().sha256.clone();
                let mut image = legacy(&source_request, &source, &fonts);
                // Source identity is intentionally different; every drawing value
                // and pixel must agree without removing any other metadata.
                image.info.page.page.source_sha256 = r.page.expected_source_sha256.clone();
                image
            })
            .collect();
        // Keep the source batch compatibility route under the same oracle.
        let counted_source = Counted {
            reader: source.reader,
            bytes: Cell::new(0),
        };
        let source_fonts = Counted {
            reader: fonts.content.reader,
            bytes: Cell::new(0),
        };
        let mut source_requests = requests.to_vec();
        for request in &mut source_requests {
            request.page.expected_source_sha256 = output.receipt().sha256.clone();
        }
        self.inner.render_pages(
            &source_requests,
            PreviewInput::Source(Content {
                reader: &counted_source,
                byte_length: source.byte_length,
            }),
            PreviewFonts {
                manifest: fonts.manifest,
                content: Content {
                    reader: &source_fonts,
                    byte_length: fonts.content.byte_length,
                },
            },
            check,
            &mut |ordinal, mut image| {
                assert_eq!(image.pixels, expected[ordinal].pixels);
                image.info.page.page.source_sha256 =
                    requests[ordinal].page.expected_source_sha256.clone();
                assert_eq!(
                    serde_json::to_value(image.info).unwrap(),
                    serde_json::to_value(&expected[ordinal].info).unwrap()
                );
                Ok(())
            },
        )?;
        assert_eq!(counted_source.bytes.get(), source.byte_length);
        assert_eq!(source_fonts.bytes.get(), fonts.content.byte_length);
        let mut counted = std::collections::BTreeMap::new();
        for id in plan.bindings().images.keys() {
            let data = resources.open(id)?;
            counted.insert(
                id.clone(),
                (
                    Counted {
                        reader: data.reader,
                        bytes: Cell::new(0),
                    },
                    data.byte_length,
                ),
            );
        }
        let counted = CountedResources(counted);
        let counted_fonts = Counted {
            reader: fonts.content.reader,
            bytes: Cell::new(0),
        };
        let pages = &mut self.pages;
        self.inner.render_pages(
            requests,
            PreviewInput::Author {
                plan,
                resources: &counted,
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
        for (reader, length) in counted.0.values() {
            assert_eq!(reader.bytes.get(), *length);
        }
        assert_eq!(counted_fonts.bytes.get(), fonts.content.byte_length);
        Ok(())
    }
}

#[test]
#[ignore = "requires pinned MO_DELIVERY_WORKER and SHA256"]
fn direct_author_batch_transfers_resources_once_and_matches_native_source_rendering() {
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
