#![cfg(any(target_os = "macos", target_os = "linux"))]
#[path = "../../mo-presentation-source/tests/support/charts.rs"]
mod fixture;
use fixture::*;
use mo_geometry::{Fixed, Point};
use mo_harfbuzz_sys::NativeShaper;
use mo_image::{DecoderReply, ImageDecoder, ImageError};
use mo_opc::{Package, PackageLimits};
use mo_presentation_compile::{source_page::*, source_resource_page::*};
use mo_presentation_source::source::{self, SourceIndex};
use mo_text::{
    TextError,
    backend::TextBackend,
    manifest::{FontManifest, PreparedManifest},
};
const PART: &str = "/ppt/charts/chart1.xml";
fn style() -> String {
    r#"<c:txPr><a:bodyPr/><a:lstStyle/><a:p><a:pPr><a:defRPr sz="1000"><a:solidFill><a:srgbClr val="112233"/></a:solidFill><a:latin typeface="MusterOffice Synthetic"/></a:defRPr></a:pPr></a:p></c:txPr>"#.into()
}
fn chart_xml(kind: &str) -> String {
    let style = style();
    let axes = if kind == "barChart" || kind == "lineChart" {
        format!(
            r#"<c:catAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:axPos val="b"/>{style}<c:crossAx val="2"/></c:catAx><c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="1"/><c:axPos val="l"/>{style}<c:crossAx val="1"/></c:valAx>"#
        )
    } else {
        String::new()
    };
    let props = if kind == "barChart" {
        "<c:barDir val=\"col\"/><c:grouping val=\"clustered\"/>"
    } else if kind == "doughnutChart" {
        "<c:firstSliceAng val=\"0\"/><c:holeSize val=\"60\"/>"
    } else if kind == "pieChart" {
        "<c:firstSliceAng val=\"0\"/>"
    } else {
        "<c:grouping val=\"standard\"/>"
    };
    let refs = if axes.is_empty() {
        ""
    } else {
        "<c:axId val=\"1\"/><c:axId val=\"2\"/>"
    };
    format!(
        r#"<c:chartSpace xmlns:c="{C}" xmlns:a="{A}" xmlns:r="{R}"><c:chart><c:plotArea><c:{kind}>{props}<c:ser><c:idx val="0"/><c:order val="0"/><c:spPr><a:solidFill><a:srgbClr val="2288AA"/></a:solidFill><a:ln w="20000"><a:solidFill><a:srgbClr val="2288AA"/></a:solidFill></a:ln></c:spPr><c:cat><c:strLit><c:ptCount val="2"/><c:pt idx="0"><c:v>A</c:v></c:pt><c:pt idx="1"><c:v>α</c:v></c:pt></c:strLit></c:cat><c:val><c:numLit><c:ptCount val="2"/><c:pt idx="0"><c:v>2</c:v></c:pt><c:pt idx="1"><c:v>3</c:v></c:pt></c:numLit></c:val></c:ser>{refs}</c:{kind}>{axes}</c:plotArea></c:chart></c:chartSpace>"#
    )
}
fn native(xml: &str) -> (Package<std::io::Cursor<Vec<u8>>>, SourceIndex) {
    let b = package(xml, &frame(2), CT, "chart", false);
    let len = b.len() as u64;
    let p = Package::open(
        std::io::Cursor::new(b),
        len,
        PackageLimits::default(),
        &|| false,
    )
    .unwrap();
    let i = source::inspect_source(&p, Default::default(), &|| false).unwrap();
    (p, i)
}
fn request(i: &SourceIndex) -> SourcePageRequest {
    SourcePageRequest {
        expected_source_sha256: i.source_sha256.clone(),
        slide: "/ppt/slides/slide1.xml".into(),
        profile: SourcePageProfile::StaticSolidDraftV1,
        color_context: Default::default(),
        viewport: mo_raster::RasterViewport {
            width: 915,
            height: 515,
            origin: Point {
                x: Fixed::ZERO,
                y: Fixed::ZERO,
            },
            scale: mo_raster::PixelScale {
                numerator: 1,
                denominator: 10000,
            },
            coordinate_tolerance: Fixed::from_raw(1 << 20),
            background: [255; 4],
        },
    }
}
struct NoImages;
impl ImageDecoder for NoImages {
    fn decode(&mut self, _: &[u8]) -> Result<DecoderReply, ImageError> {
        panic!("unexpected image decode")
    }
    fn invalidate(&mut self) {}
}
#[derive(Default)]
struct Shaper {
    budget_probe: bool,
    inner: NativeShaper,
    calls: usize,
}
impl TextBackend for Shaper {
    fn shape_batch(&mut self, f: &[u8], w: &[u32]) -> Result<Vec<u32>, TextError> {
        self.calls += 1;
        self.inner.shape_batch(f, w)
    }
    fn measure_batch(&mut self, f: &[u8], w: &[u32]) -> Result<Vec<u32>, TextError> {
        self.calls += 1;
        self.inner.measure_batch(f, w)
    }
    fn outline_batch(&mut self, f: &[u8], w: &[u32]) -> Result<Vec<u32>, TextError> {
        self.calls += 1;
        self.inner.outline_batch(f, w)
    }
    fn invalidate(&mut self) {
        if !self.budget_probe {
            self.inner.invalidate()
        }
    }
}
fn compile(
    xml: &str,
    options: ResourcePageOptions,
    cancel: &dyn Fn() -> bool,
) -> (Result<SourceResourcePagePlan, SourcePageError>, usize) {
    let (p, i) = native(xml);
    let q = request(&i);
    let fonts: FontManifest = serde_json::from_str(include_str!(
        "../../../fixtures/fonts/decoration-manifest.json"
    ))
    .unwrap();
    let m = PreparedManifest::load(
        &fonts,
        include_bytes!("../../../fixtures/fonts/owned-decorations.ttf"),
        Default::default(),
        &|| false,
    )
    .unwrap();
    let mut shaper = Shaper {
        budget_probe: options.text_limits.work.max_component_calls == 0,
        ..Default::default()
    };
    let r = prepare(
        &p,
        &i,
        &q,
        &mut NoImages,
        Some(TextPageContext {
            manifest: &m,
            backend: &mut shaper,
        }),
        options,
        cancel,
    )
    .and_then(|p| p.plan(cancel));
    (r, shaper.calls)
}
#[test]
fn native_bar_line_pie_and_ring_keep_chart_identity() {
    for kind in ["barChart", "lineChart", "pieChart", "doughnutChart"] {
        let (r, _) = compile(&chart_xml(kind), options(), &|| false);
        let p = r.unwrap();
        assert_eq!(p.page.info.charts.len(), 1);
        assert_eq!(p.page.info.charts[0].chart_part, PART);
        assert_eq!(p.page.info.charts[0].object.native_id, 2);
        assert!(
            p.page
                .paint_sources
                .iter()
                .any(|s| s.chart.as_ref().is_some_and(|c| c.part == PART))
        );
    }
}
#[test]
fn unsupported_root_is_rejected_before_font_callbacks() {
    let xml = chart_xml("barChart").replace("<c:chart>", "<c:chart><c:view3D/>");
    let (r, calls) = compile(&xml, options(), &|| false);
    assert!(r.is_err());
    assert_eq!(calls, 0);
}
#[test]
fn unsupported_axis_position_is_rejected_before_chart_shaping() {
    let mut xml = chart_xml("barChart");
    xml = xml.replace("<c:axPos val=\"b\"/>", "<c:axPos val=\"t\"/>");
    let (r, calls) = compile(&xml, options(), &|| false);
    assert!(r.is_err());
    assert_eq!(calls, 0);
}
#[test]
fn cancellation_is_not_reclassified_as_an_unsupported_chart() {
    let (r, calls) = compile(&chart_xml("barChart"), options(), &|| true);
    assert!(matches!(
        r,
        Err(SourcePageError::Raster(mo_raster::RasterError::Cancelled))
    ));
    assert_eq!(calls, 0);
}
#[test]
fn chart_text_obeys_page_component_budget() {
    let mut o = options();
    o.text_limits.work.max_component_calls = 0;
    let (r, calls) = compile(&chart_xml("barChart"), o, &|| false);
    assert!(r.is_err());
    assert_eq!(calls, 0);
}
#[test]
fn negative_and_zero_bars_are_valid_but_log_axes_are_not() {
    let xml = chart_xml("barChart")
        .replace("<c:v>2</c:v>", "<c:v>-2</c:v>")
        .replace("<c:v>3</c:v>", "<c:v>0</c:v>");
    assert!(compile(&xml, options(), &|| false).0.is_ok());
    let xml = xml.replace("<c:scaling>", "<c:scaling><c:logBase val=\"10\"/>");
    assert!(compile(&xml, options(), &|| false).0.is_err());
}

fn options() -> ResourcePageOptions {
    ResourcePageOptions {
        selection: source::images::ImageSourceSelection::EmbeddedSnapshot,
        sampling: mo_raster::ImageSampling::Nearest,
        text_limits: Default::default(),
    }
}
