#[allow(dead_code)]
#[path = "../../../tools/test-support/source_text_page.rs"]
mod support;
use mo_harfbuzz_sys::NativeShaper;
use mo_presentation_compile::source_text_page::{self, TextPageLimits};
use mo_skia_sys::NativeRaster;
use mo_text::{TextError, backend::TextBackend, manifest::*};
use support::*;
const FONT: &[u8] = include_bytes!("../../../fixtures/fonts/owned.ttf");
struct Probe {
    inner: NativeShaper,
    resident: bool,
    registrations: usize,
    releases: usize,
    batches: usize,
}
impl Probe {
    fn new(resident: bool) -> Self {
        Self {
            inner: NativeShaper::default(),
            resident,
            registrations: 0,
            releases: 0,
            batches: 0,
        }
    }
}
impl TextBackend for Probe {
    fn supports_font_residency(&self) -> bool {
        self.resident
    }
    fn register_font(&mut self, font: &[u8]) -> Result<u32, TextError> {
        self.registrations += 1;
        self.inner.register_font(font)
    }
    fn unregister_font(&mut self, id: u32) -> Result<(), TextError> {
        self.releases += 1;
        self.inner.unregister_font(id)
    }
    fn shape_registered(&mut self, id: u32, words: &[u32]) -> Result<Vec<u32>, TextError> {
        self.batches += 1;
        self.inner.shape_registered(id, words)
    }
    fn measure_registered(&mut self, id: u32, words: &[u32]) -> Result<Vec<u32>, TextError> {
        self.batches += 1;
        self.inner.measure_registered(id, words)
    }
    fn outline_registered(&mut self, id: u32, words: &[u32]) -> Result<Vec<u32>, TextError> {
        self.batches += 1;
        self.inner.outline_registered(id, words)
    }
    fn shape_batch(&mut self, font: &[u8], words: &[u32]) -> Result<Vec<u32>, TextError> {
        self.batches += 1;
        self.inner.shape_batch(font, words)
    }
    fn measure_batch(&mut self, font: &[u8], words: &[u32]) -> Result<Vec<u32>, TextError> {
        self.batches += 1;
        self.inner.measure_batch(font, words)
    }
    fn outline_batch(&mut self, font: &[u8], words: &[u32]) -> Result<Vec<u32>, TextError> {
        self.batches += 1;
        self.inner.outline_batch(font, words)
    }
    fn invalidate(&mut self) {
        self.inner.invalidate();
    }
}
fn without_transfer_counts(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            map.remove("fontUploadBytes");
            for v in map.values_mut() {
                without_transfer_counts(v);
            }
        }
        serde_json::Value::Array(values) => {
            for v in values {
                without_transfer_counts(v);
            }
        }
        _ => {}
    }
}
#[test]
fn page_reuses_verified_font_without_changing_any_geometry_or_pixels() {
    let a = shape(42, 100000, 100000, "", &colored("A A A A A A", "2070C0"));
    let b = shape(43, 100000, 1500000, "", &colored("A A A A A A", "C02040"));
    let bytes = fixture(&(a + &b));
    let index = read(&bytes);
    let author = author();
    let manifest =
        PreparedManifest::load(&author.manifest, FONT, Default::default(), &|| false).unwrap();
    let mut old = Probe::new(false);
    let mut new = Probe::new(true);
    let legacy = source_text_page::render(
        &index,
        &request(&index),
        &manifest,
        &mut old,
        &mut NativeRaster,
        Default::default(),
        &|| false,
    )
    .unwrap();
    let mut limits = TextPageLimits::default();
    limits.work.max_font_upload_bytes = FONT.len() as u64;
    let resident = source_text_page::render(
        &index,
        &request(&index),
        &manifest,
        &mut new,
        &mut NativeRaster,
        limits,
        &|| false,
    )
    .unwrap();
    assert_eq!(legacy.pixels, resident.pixels);
    assert_eq!(old.batches, new.batches);
    assert!(new.batches > 4);
    assert_eq!(old.registrations, 0);
    assert_eq!((new.registrations, new.releases), (1, 1));
    assert_eq!(resident.info.text_work.font_upload_bytes, FONT.len() as u64);
    assert_eq!(
        legacy.info.text_work.font_upload_bytes,
        FONT.len() as u64 * old.batches as u64
    );
    let mut before = serde_json::to_value(&legacy.info).unwrap();
    let mut after = serde_json::to_value(&resident.info).unwrap();
    without_transfer_counts(&mut before);
    without_transfer_counts(&mut after);
    assert_eq!(before, after);
    // A second calculation gets a new scope and the same bounded transfer count.
    let second = source_text_page::compile(
        &index,
        &request(&index),
        &manifest,
        &mut new,
        limits,
        &|| false,
    )
    .unwrap();
    assert_eq!(second.text_work.font_upload_bytes, FONT.len() as u64);
    assert_eq!((new.registrations, new.releases), (2, 2));
}
