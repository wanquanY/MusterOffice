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
                PptxResourcePageRasterResponse::Error { error } => {
                    Err(format!("legacy page failed: {error}"))
                }
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

struct Original(Vec<u8>);
impl mo_pptx::Resources for Original {
    fn open(&self, id: &ResourceId) -> Result<mo_pptx::ResourceData<'_>, mo_pptx::PptxError> {
        if id.as_str() != "original" {
            return Err(mo_pptx::PptxError::ResourceRequired(id.clone()));
        }
        Ok(mo_pptx::ResourceData {
            reader: &self.0,
            byte_length: self.0.len() as u64,
        })
    }
}
struct RetainedCompared {
    inner: NativePreviewRenderer,
    batches: usize,
}
impl PreviewRenderer for RetainedCompared {
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
        let PreviewInput::Retained { plan, source } = input else {
            panic!("retained semantics required")
        };
        let package =
            mo_opc::Package::open(source.reader, source.byte_length, Default::default(), check)
                .unwrap();
        let output = plan.write(&package, check).unwrap();
        let written = mo_opc::Package::open(
            output.as_slice(),
            output.len() as u64,
            Default::default(),
            check,
        )
        .unwrap();
        let expected: Vec<_> = requests
            .iter()
            .map(|r| {
                let mut request = r.clone();
                request.page.expected_source_sha256 = written.sha256().clone();
                let mut image = legacy(
                    &request,
                    &Content {
                        reader: &output,
                        byte_length: output.len() as u64,
                    },
                    &fonts,
                );
                image.info.page.page.source_sha256 = r.page.expected_source_sha256.clone();
                image
            })
            .collect();
        let counted = Counted {
            reader: source.reader,
            bytes: Cell::new(0),
        };
        self.inner.render_pages(
            requests,
            PreviewInput::Retained {
                plan,
                source: Content {
                    reader: &counted,
                    byte_length: source.byte_length,
                },
            },
            fonts,
            check,
            &mut |ordinal, image| {
                assert_eq!(image.pixels, expected[ordinal].pixels);
                assert_eq!(
                    serde_json::to_value(&image.info).unwrap(),
                    serde_json::to_value(&expected[ordinal].info).unwrap()
                );
                emit(ordinal, image)
            },
        )?;
        assert_eq!(counted.bytes.get(), source.byte_length);
        Ok(())
    }
}
#[test]
#[ignore = "requires pinned MO_DELIVERY_WORKER and SHA256"]
fn retained_revision_renders_directly_and_delivery_evidence_binds_original_and_output() {
    let (snapshot, settings) = input();
    let original = Original(
        mo_pptx::export(
            &snapshot.document,
            &settings.defaults,
            &Images,
            Default::default(),
            &|| false,
        )
        .unwrap(),
    );
    let package = mo_opc::Package::open(
        original.0.as_slice(),
        original.0.len() as u64,
        Default::default(),
        &|| false,
    )
    .unwrap();
    let d = mo_pptx::source::document::import_document(
        &package,
        mo_common::DocumentId::new("retained").unwrap(),
        ResourceId::new("original").unwrap(),
        Default::default(),
        &|| false,
    )
    .unwrap();
    let s = Snapshot::new(d, Default::default()).unwrap();
    let object = s.document().objects.values().find(|o| matches!(&o.content, mo_presentation_model::ObjectContent::RetainedSource { paragraphs, .. } if !paragraphs.is_empty() && !paragraphs[0].runs.is_empty())).unwrap();
    let mo_presentation_model::ObjectContent::RetainedSource { paragraphs, .. } = &object.content
    else {
        unreachable!()
    };
    let tx = mo_presentation_edit::Transaction {
        document_id: s.document().id.clone(),
        base_revision: s.revision().clone(),
        request_id: mo_common::RequestId::new("edit").unwrap(),
        operations: vec![mo_presentation_edit::OperationEntry {
            operation_id: mo_common::OperationId::new("splice").unwrap(),
            operation: mo_presentation_edit::Operation::SpliceText {
                object: object.id.clone(),
                paragraph: paragraphs[0].id.clone(),
                run: paragraphs[0].runs[0].id.clone(),
                start: 0,
                delete: 0,
                insert: "A ".into(),
            },
        }],
    };
    let snapshot = mo_presentation_edit::prepare(&s, &tx, Default::default())
        .unwrap()
        .snapshot
        .into_record();
    let mut renderer = RetainedCompared {
        inner: renderer(),
        batches: 0,
    };
    let fonts: &[u8] = include_bytes!("../../../../fixtures/fonts/owned.ttf");
    let candidate = build(
        DeliveryInputs {
            snapshot: snapshot.clone(),
            settings: &settings,
            resources: &original,
            fonts: Content {
                reader: &fonts,
                byte_length: fonts.len() as u64,
            },
        },
        &mut Store::default(),
        &mut renderer,
        DeliveryLimits::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(renderer.batches, 1);
    struct Assets<'a>(&'a [ProducedArtifact<Vec<u8>>]);
    impl DeliverySource for Assets<'_> {
        fn open(&self, id: &mo_common::RequestId) -> Result<Content<'_>, DeliveryError> {
            let a = self.0.iter().find(|a| &a.asset().id == id).unwrap();
            Ok(Content {
                reader: a.reader(),
                byte_length: a.asset().byte_length.get(),
            })
        }
    }
    let pins = DeliveryExpectation {
        document_id: snapshot.document.id.clone(),
        revision: snapshot.revision.clone(),
        semantic_digest: snapshot.semantic_digest.clone(),
        settings_digest: candidate.settings_digest().clone(),
        renderer: renderer.identity(),
    };
    let received = inspect(
        candidate.bundle(),
        &pins,
        &Assets(candidate.artifacts()),
        Default::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(received.snapshot(), &snapshot);
}
