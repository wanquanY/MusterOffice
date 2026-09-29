//! Native table text through the real shared paint/retention pipeline. Geometry
//! and fill/outline page integration are separate; no fake source object is used.
use super::*;
#[allow(dead_code)]
#[path = "../../../../tools/test-support/source_timing.rs"]
mod playback_support;
#[allow(dead_code)]
#[path = "../../../../tools/test-support/table_text.rs"]
mod support;
use crate::source_placement::*;
use mo_harfbuzz_sys::NativeShaper;
use mo_presentation_source::source::{
    fill::{colors::*, resolve::*},
    table::SourceCellAddress,
};
use mo_skia_sys::NativeRaster;
use mo_text::{TextError, manifest::FontManifest};
use std::{cell::Cell, collections::BTreeSet};
use support::*;

static FONT_BYTES: &[u8] = include_bytes!("../../../../fixtures/fonts/owned-decorations.ttf");
fn fonts() -> FontManifest {
    serde_json::from_str(include_str!(
        "../../../../fixtures/fonts/decoration-manifest.json"
    ))
    .unwrap()
}
fn request(index: &SourceIndex) -> SourcePageRequest {
    SourcePageRequest {
        expected_source_sha256: index.source_sha256.clone(),
        slide: SLIDE.into(),
        profile: SourcePageProfile::StaticSolidDraftV1,
        color_context: Default::default(),
        viewport: RasterViewport {
            width: 800,
            height: 450,
            origin: Point {
                x: Fixed::ZERO,
                y: Fixed::ZERO,
            },
            scale: mo_raster::PixelScale {
                numerator: 1,
                denominator: 12700,
            },
            coordinate_tolerance: Fixed::from_raw(1 << 24),
            background: [255; 4],
        },
    }
}
fn bindings(
    index: &SourceIndex,
    q: &SourcePageRequest,
    motion: i64,
) -> Vec<SourcePagePaintBinding> {
    let ids: Vec<_> = index.surfaces[SLIDE]
        .objects
        .iter()
        .filter(|o| o.table.is_some())
        .map(|o| o.native_id)
        .collect();
    let transforms = ids
        .iter()
        .map(|id| {
            (
                (SLIDE.to_owned(), *id),
                crate::sampled_properties::SampledProperties {
                    motion: Some([
                        crate::interval::Interval::ratio(motion, 100),
                        crate::interval::Interval::ratio(-motion, 200),
                    ]),
                    ..Default::default()
                },
            )
        })
        .collect();
    let placements = source_placements_sampled(
        index,
        &SourcePlacementQuery {
            expected_source_sha256: index.source_sha256.clone(),
            surface: SLIDE.into(),
            objects: ids.clone(),
            profile: SourcePlacementProfile::DrawingmlSourceDraftV1,
        },
        Default::default(),
        Some(&transforms),
        &|| false,
    )
    .unwrap();
    let fills = mo_presentation_source::source::fill::colors::query(
        index,
        &SourceFillColorQuery {
            expected_source_sha256: index.source_sha256.clone(),
            surface: SLIDE.into(),
            targets: ids
                .iter()
                .map(|id| FillTarget::TableBackground { native_id: *id })
                .collect(),
            fill_profile: FillProfile::Drawingml2024DraftV1,
            color_profile: mo_presentation_source::source::color::ColorProfile::Ecma3762016DraftV1,
            context: q.color_context.clone(),
        },
        Default::default(),
        &|| false,
    )
    .unwrap();
    placements
        .objects
        .into_iter()
        .zip(fills.targets)
        .map(|(p, fill)| {
            let SourcePlacementOutcome::Resolved { placement } = p.outcome else {
                panic!("native table placement")
            };
            SourcePagePaintBinding {
                location: SourcePageLocation {
                    part: SLIDE.into(),
                    object: Some(p.native_id),
                },
                drawing_surface: SLIDE.into(),
                fill,
                picture_fill: None,
                line: None,
                placement: Some(*placement),
                region: None,
                table_stroke: None,
            }
        })
        .collect()
}
#[derive(Default)]
struct Shaper {
    calls: usize,
    inner: NativeShaper,
}
impl TextBackend for Shaper {
    fn shape_batch(&mut self, b: &[u8], words: &[u32]) -> Result<Vec<u32>, TextError> {
        self.calls += 1;
        self.inner.shape_batch(b, words)
    }
    fn measure_batch(&mut self, b: &[u8], words: &[u32]) -> Result<Vec<u32>, TextError> {
        self.calls += 1;
        self.inner.measure_batch(b, words)
    }
    fn outline_batch(&mut self, b: &[u8], words: &[u32]) -> Result<Vec<u32>, TextError> {
        self.calls += 1;
        self.inner.outline_batch(b, words)
    }
    fn invalidate(&mut self) {
        self.inner.invalidate()
    }
}
fn compile(
    index: &SourceIndex,
    q: &SourcePageRequest,
    bindings: &[SourcePagePaintBinding],
) -> (TextPageContent, mo_render::DrawScene) {
    let fonts = fonts();
    let manifest =
        PreparedManifest::load(&fonts, FONT_BYTES, Default::default(), &|| false).unwrap();
    let mut shaper = Shaper::default();
    let mut compiler = Compiler::new(&manifest, &mut shaper, Default::default());
    compiler
        .preflight(index, q, bindings.iter(), &|| false)
        .unwrap();
    let mut builder = SceneBuilder::new();
    for (n, b) in bindings.iter().enumerate() {
        let (p, g) = compiler
            .append(n as u32, b, &mut builder, &q.viewport, &|| false)
            .unwrap();
        assert!(p.checked_add(g).unwrap() < q.viewport.coordinate_tolerance);
    }
    let content = compiler.finish().unwrap();
    assert!(shaper.calls > 0);
    (content, builder.scene)
}
fn raster(q: &SourcePageRequest, scene: mo_render::DrawScene) -> Vec<u8> {
    mo_render::render(
        &mo_render::SceneRasterRequest {
            viewport: q.viewport.clone(),
            scene,
        },
        &mut NativeRaster,
        &|| false,
    )
    .unwrap()
    .pixels
}
fn cell(row: u32, column: u32) -> SourceCellAddress {
    SourceCellAddress { row, column }
}
fn evidence(
    name: &str,
    source: &[u8],
    content: &TextPageContent,
    scene: &mo_render::DrawScene,
    pixels: &[u8],
) {
    let Some(path) = std::env::var_os("MO_TABLE_TEXT_PAGE_EVIDENCE_DIR") else {
        return;
    };
    let path = std::path::PathBuf::from(path);
    assert!(path.is_absolute());
    std::fs::create_dir_all(&path).unwrap();
    for (suffix, bytes) in [
        ("pptx", source.to_vec()),
        (
            "request.json",
            serde_json::to_vec(&request(&read(source))).unwrap(),
        ),
        ("content.json", serde_json::to_vec(content).unwrap()),
        ("scene.json", serde_json::to_vec(scene).unwrap()),
        ("rgba", pixels.to_vec()),
    ] {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path.join(format!("{name}.{suffix}")))
            .unwrap();
        f.write_all(&bytes).unwrap();
    }
}

#[test]
fn actual_table_frames_keep_native_cells_clips_and_static_retained_pixels() {
    for rtl in [false, true] {
        let bytes = rewrite(&fixture(false), SLIDE, |s| {
            s.replace("<a:tblPr", &format!("<a:tblPr rtl=\"{}\"", u8::from(rtl)))
                .replace("horzOverflow=\"overflow\"", "horzOverflow=\"clip\"")
                .replace("A A", &"A".repeat(32))
        });
        let index = read(&bytes);
        let before = serde_json::to_vec(&index).unwrap();
        let q = request(&index);
        let base = bindings(&index, &q, 0);
        let (content, scene) = compile(&index, &q, &base);
        assert_eq!(content.texts.len(), 6);
        assert_eq!(
            content
                .texts
                .iter()
                .map(|t| t.frame.text.cell.unwrap())
                .collect::<Vec<_>>(),
            vec![
                cell(0, 0),
                cell(0, 2),
                cell(1, 2),
                cell(2, 0),
                cell(2, 1),
                cell(2, 2)
            ]
        );
        assert!(content.texts.iter().all(|t| t.binding == 0
            && t.frame.text.object.native_id == base[0].location.object.unwrap()
            && t.frame.clip.is_some()));
        let visible = BTreeSet::from([(SLIDE.into(), base[0].location.object)]);
        let retained = retained::RetainedText::new(content.clone(), &base, &|| false).unwrap();
        assert_eq!(retained.frames(), 6);
        for motion in [0, 5, 0] {
            let sample = bindings(&index, &q, motion);
            let (_, fresh) = compile(&index, &q, &sample);
            let mut painter = retained.painter(&visible);
            let mut builder = SceneBuilder::new();
            painter
                .append(0, &sample[0], &mut builder, &q.viewport, &|| false)
                .unwrap();
            let (work, capacity) = painter.finish().unwrap();
            assert_eq!(work.component_calls, 0);
            assert_eq!(capacity.frames.len(), 6);
            assert_eq!(work.glyphs, content.text_work.glyphs);
            assert_eq!(
                serde_json::to_vec(&builder.scene).unwrap(),
                serde_json::to_vec(&fresh).unwrap()
            );
            let pixels = raster(&q, builder.scene);
            assert_eq!(pixels, raster(&q, fresh));
            assert!(pixels.chunks_exact(4).filter(|p| *p != [255; 4]).count() > 100);
        }
        let pixels = raster(&q, scene.clone());
        evidence(&format!("rtl-{rtl}"), &bytes, &content, &scene, &pixels);
        assert_eq!(before, serde_json::to_vec(&index).unwrap());
    }
}

#[test]
fn page_limits_count_cell_frames_and_reject_before_any_component_call() {
    let index = read(&fixture(false));
    let q = request(&index);
    let b = bindings(&index, &q, 0);
    let fonts = fonts();
    let manifest =
        PreparedManifest::load(&fonts, FONT_BYTES, Default::default(), &|| false).unwrap();
    for limits in [
        TextPageLimits {
            tables: crate::source_table::TableGeometryLimits {
                max_objects: 0,
                ..Default::default()
            },
            ..Default::default()
        },
        TextPageLimits {
            max_frames: 5,
            ..Default::default()
        },
        TextPageLimits {
            max_paragraphs: 5,
            ..Default::default()
        },
        TextPageLimits {
            max_runs: 4,
            ..Default::default()
        },
        TextPageLimits {
            max_prepared_plan_bytes: 1,
            ..Default::default()
        },
        TextPageLimits {
            tables: crate::source_table::TableGeometryLimits {
                grid: mo_presentation_source::source::table::grid::NativeTableGridLimits {
                    max_cells: 8,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        },
    ] {
        let mut shaper = Shaper::default();
        let mut c = Compiler::new(&manifest, &mut shaper, limits);
        assert!(c.preflight(&index, &q, b.iter(), &|| false).is_err());
        assert!(c.finish().is_err());
        assert_eq!(shaper.calls, 0);
    }
    let mut shaper = Shaper::default();
    let mut c = Compiler::new(&manifest, &mut shaper, Default::default());
    assert!(
        c.preflight(&index, &q, b.iter().chain(b.iter()), &|| false)
            .is_err()
    );
    assert!(c.finish().is_err());
    assert_eq!(shaper.calls, 0);
    let calls = Cell::new(0);
    let mut c = Compiler::new(&manifest, &mut shaper, Default::default());
    assert!(
        c.preflight(&index, &q, b.iter(), &|| {
            calls.set(calls.get() + 1);
            calls.get() > 90
        })
        .is_err()
    );
    assert!(c.finish().is_err());
    assert_eq!(shaper.calls, 0);
}

#[test]
fn retained_frame_identity_visibility_and_atomic_cancellation_are_checked() {
    let index = read(&fixture(false));
    let q = request(&index);
    let b = bindings(&index, &q, 0);
    let (content, _) = compile(&index, &q, &b);
    let mut duplicate = content.clone();
    duplicate.texts[1] = duplicate.texts[0].clone();
    assert!(retained::RetainedText::new(duplicate, &b, &|| false).is_err());
    let mut wrong = content.clone();
    wrong.texts[1].frame.text.object.native_id += 1;
    assert!(retained::RetainedText::new(wrong, &b, &|| false).is_err());
    let retained = retained::RetainedText::new(content, &b, &|| false).unwrap();
    let empty = BTreeSet::new();
    let (work, capacity) = retained.painter(&empty).finish().unwrap();
    assert_eq!(work.glyphs, 0);
    assert!(capacity.frames.is_empty());
    let visible = BTreeSet::from([(SLIDE.into(), b[0].location.object)]);
    assert!(retained.painter(&visible).finish().is_err());
    let mut p = retained.painter(&visible);
    assert!(
        p.append(0, &b[0], &mut SceneBuilder::new(), &q.viewport, &|| true)
            .is_err()
    );
    assert!(p.finish().is_err());
    let mut p = retained.painter(&visible);
    let mut builder = SceneBuilder::new();
    p.append(0, &b[0], &mut builder, &q.viewport, &|| false)
        .unwrap();
    assert!(
        p.append(0, &b[0], &mut builder, &q.viewport, &|| false)
            .is_err()
    );
}

mod cases;
mod context;
mod page;

mod images;
mod retention;
