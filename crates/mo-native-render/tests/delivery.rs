//! Execute explicitly after building and pinning the real native renderer.
use mo_common::{Digest, ResourceId};
use mo_native_render::NativePreviewRenderer;
use mo_opc::{ResultSink, SealedOutput};
use mo_pptx::{PptxError, ResourceData, Resources};
use mo_presentation_compile::source_resource_page::SourceResourcePageImage;
use mo_presentation_delivery::*;
use mo_presentation_edit::{Snapshot, SnapshotRecord};
use std::{
    cell::Cell,
    io::{self, Write},
    rc::Rc,
    time::Duration,
};

fn input() -> (SnapshotRecord, DeliverySettings) {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/delivery/input.json"
    ))
    .unwrap();
    (
        Snapshot::new(
            serde_json::from_value(v["document"].clone()).unwrap(),
            Default::default(),
        )
        .unwrap()
        .into_record(),
        serde_json::from_value(v["settings"].clone()).unwrap(),
    )
}
struct Images;
impl Resources for Images {
    fn open(&self, id: &ResourceId) -> Result<ResourceData<'_>, PptxError> {
        if id.as_str() != "resource:checker" {
            return Err(PptxError::ResourceRequired(id.clone()));
        }
        static BYTES: &[u8] =
            include_bytes!("../../../fixtures/presentations/native-export/resources.bin");
        Ok(ResourceData {
            reader: &BYTES,
            byte_length: BYTES.len() as u64,
        })
    }
}
#[derive(Default)]
struct Store {
    created: Vec<String>,
    fail: Option<&'static str>,
    corrupt: Option<&'static str>,
    cancel: Rc<Cell<bool>>,
    cancel_on_quality: bool,
}
struct Sink {
    bytes: Vec<u8>,
    max: u64,
    corrupt: bool,
    cancel: Rc<Cell<bool>>,
    cancel_on_seal: bool,
}
impl Write for Sink {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        if self.bytes.len() as u64 + b.len() as u64 > self.max {
            return Err(io::Error::other("test store budget"));
        }
        let n = b.len().min(3071);
        self.bytes.extend_from_slice(&b[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl ResultSink for Sink {
    type Reader = Vec<u8>;
    fn seal(mut self) -> io::Result<SealedOutput<Vec<u8>>> {
        if self.corrupt {
            self.bytes[0] ^= 1;
        }
        if self.cancel_on_seal {
            self.cancel.set(true);
        }
        Ok(SealedOutput {
            byte_length: self.bytes.len() as u64,
            reader: self.bytes,
        })
    }
}
impl OutputStore for Store {
    type Sink = Sink;
    fn create(&mut self, name: &str, _: &str, max: u64) -> Result<Sink, DeliveryError> {
        self.created.push(name.into());
        if self.fail == Some(name) {
            return Err(io::Error::other("injected store failure").into());
        }
        Ok(Sink {
            bytes: vec![],
            max,
            corrupt: self.corrupt == Some(name),
            cancel: self.cancel.clone(),
            cancel_on_seal: self.cancel_on_quality && name == "quality",
        })
    }
}
fn renderer() -> NativePreviewRenderer {
    NativePreviewRenderer::new(
        std::env::var_os("MO_DELIVERY_WORKER")
            .expect("explicit worker path")
            .into(),
        Digest::try_from(
            std::env::var("MO_DELIVERY_WORKER_SHA256").expect("explicit worker digest"),
        )
        .unwrap(),
        Duration::from_secs(60),
    )
    .unwrap()
}
fn run(
    snapshot: SnapshotRecord,
    settings: &DeliverySettings,
    store: &mut Store,
    renderer: &mut dyn PreviewRenderer,
    limits: DeliveryLimits,
) -> Result<DeliveryCandidate<Vec<u8>>, DeliveryError> {
    let font = include_bytes!("../../../fixtures/fonts/owned.ttf").as_slice();
    let cancel = store.cancel.clone();
    build(
        DeliveryInputs {
            snapshot,
            settings,
            resources: &Images,
            fonts: Content {
                reader: &font,
                byte_length: font.len() as u64,
            },
        },
        store,
        renderer,
        limits,
        &|| cancel.get(),
    )
}

#[test]
#[ignore = "requires MO_DELIVERY_WORKER and its explicitly verified SHA256"]
fn actual_two_page_delivery_and_hidden_page_cover_every_bound_artifact() {
    let (mut snapshot, settings) = input();
    let mut store = Store::default();
    let mut renderer = renderer();
    let candidate = run(
        snapshot.clone(),
        &settings,
        &mut store,
        &mut renderer,
        DeliveryLimits::default(),
    )
    .unwrap();
    assert_eq!(candidate.artifacts().len(), 12);
    assert_eq!(candidate.bundle().previews.len(), 2);
    assert_eq!(
        candidate
            .bundle()
            .claims
            .iter()
            .filter(|c| c.status == ClaimStatus::Passed)
            .count(),
        1
    );
    for artifact in candidate.artifacts() {
        use sha2::{Digest as _, Sha256};
        assert_eq!(
            artifact.asset().sha256.as_str(),
            format!("{:x}", Sha256::digest(artifact.reader()))
        );
        assert_eq!(
            artifact.asset().byte_length.get(),
            artifact.reader().len() as u64
        );
    }
    snapshot.document.slides.values_mut().last().unwrap().hidden = true;
    let hidden = Snapshot::new(snapshot.document, Default::default())
        .unwrap()
        .into_record();
    let result = run(
        hidden,
        &settings,
        &mut Store::default(),
        &mut renderer,
        DeliveryLimits::default(),
    )
    .unwrap();
    assert_eq!(result.bundle().previews.len(), 2);
    assert_ne!(
        result.bundle().document.revision,
        candidate.bundle().document.revision
    );
}

struct FaultRenderer {
    inner: NativePreviewRenderer,
    fault: u8,
}
impl PreviewRenderer for FaultRenderer {
    fn identity(&self) -> RendererIdentity {
        self.inner.identity()
    }
    fn render(
        &mut self,
        q: &PreviewRequest,
        source: Content<'_>,
        fonts: Content<'_>,
        check: &dyn Fn() -> bool,
    ) -> Result<SourceResourcePageImage, DeliveryError> {
        let mut image = self.inner.render(q, source, fonts, check)?;
        match self.fault {
            0 => image.info.page.page.source_sha256 = Digest::from_sha256([0; 32]),
            1 => image.pixels[0] ^= 1,
            2 => image.info.page.scene.profile = "wrong".into(),
            3 => image.info.page.page.hidden_slide = !image.info.page.page.hidden_slide,
            _ => unreachable!(),
        }
        Ok(image)
    }
}
#[test]
#[ignore = "requires MO_DELIVERY_WORKER and its explicitly verified SHA256"]
fn failed_or_mismatched_components_never_return_a_complete_candidate() {
    let (snapshot, settings) = input();
    for fault in 0..4 {
        let mut renderer = FaultRenderer {
            inner: renderer(),
            fault,
        };
        let error = run(
            snapshot.clone(),
            &settings,
            &mut Store::default(),
            &mut renderer,
            DeliveryLimits::default(),
        )
        .err()
        .unwrap();
        assert!(matches!(error, DeliveryError::Invalid(_)), "{error:?}");
    }
    for (fail, corrupt, late_cancel) in [
        (Some("preview:1"), None, false),
        (None, Some("model"), false),
        (None, Some("preview:0"), false),
        (None, None, true),
    ] {
        let mut store = Store {
            fail,
            corrupt,
            cancel_on_quality: late_cancel,
            ..Default::default()
        };
        let error = run(
            snapshot.clone(),
            &settings,
            &mut store,
            &mut renderer(),
            DeliveryLimits::default(),
        )
        .err()
        .unwrap();
        if late_cancel {
            assert!(matches!(error, DeliveryError::Cancelled));
        }
        if fail.is_some() {
            assert!(store.created.iter().any(|s| s == "preview-evidence:0"));
        }
        if corrupt.is_some() {
            assert!(matches!(error, DeliveryError::Invalid(_)));
        }
    }
    let mut store = Store::default();
    let result = run(
        snapshot.clone(),
        &settings,
        &mut store,
        &mut renderer(),
        DeliveryLimits {
            max_artifacts: 11,
            ..Default::default()
        },
    );
    assert!(matches!(result, Err(DeliveryError::Limit(_))));
    assert!(store.created.is_empty());
    let mut bad_font = settings.clone();
    bad_font.fonts.as_mut().unwrap().fonts[0].expected_sha256 = Digest::from_sha256([0; 32]);
    let error = run(
        snapshot,
        &bad_font,
        &mut Store::default(),
        &mut renderer(),
        DeliveryLimits::default(),
    )
    .err()
    .unwrap();
    assert!(matches!(
        error,
        DeliveryError::Preview {
            diagnostic: Some(_),
            ..
        }
    ));
}
