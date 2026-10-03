use super::*;
use crate::source_resource_page::*;
#[path = "../../../mo-presentation-source/tests/support/charts.rs"]
mod fixture;
use mo_image::{DecoderReply, ImageDecoder, ImageError};
use mo_presentation_source::source::{images::ImageSourceSelection, inspect_source};
use mo_raster::{ImageSampling, PixelScale, RasterViewport};
use mo_skia_sys::NativeRaster;
struct Decoder;
impl ImageDecoder for Decoder {
    fn decode(&mut self, _: &[u8]) -> Result<DecoderReply, ImageError> {
        panic!("chart must not rasterize during preparation")
    }
    fn invalidate(&mut self) {}
}
#[test]
fn retained_chart_reuses_native_paths_and_respects_hidden_state_and_motion() {
    let xml = fixture::chart(&fixture::series())
        .replace("barChart", "pieChart")
        .replace(
            "<c:barDir val=\"col\"/><c:grouping val=\"clustered\"/>",
            "<c:firstSliceAng val=\"0\"/>",
        )
        .replace("<c:axId val=\"11\"/><c:axId val=\"12\"/>", "");
    let start = xml.find("<c:ser>").unwrap();
    let end = xml.find("</c:ser>").unwrap() + 8;
    let mut xml = xml;
    xml.replace_range(start..end,r#"<c:ser><c:idx val="0"/><c:order val="0"/><c:spPr><a:solidFill><a:srgbClr val="229966"/></a:solidFill></c:spPr><c:cat><c:strLit><c:ptCount val="1"/><c:pt idx="0"><c:v>A</c:v></c:pt></c:strLit></c:cat><c:val><c:numLit><c:ptCount val="1"/><c:pt idx="0"><c:v>1</c:v></c:pt></c:numLit></c:val></c:ser>"#);
    let b = fixture::package(&xml, &fixture::frame(2), fixture::CT, "chart", false);
    let package =
        mo_opc::Package::open(b.as_slice(), b.len() as u64, Default::default(), &|| false).unwrap();
    let index = inspect_source(&package, Default::default(), &|| false).unwrap();
    let q = SourcePageRequest {
        expected_source_sha256: index.source_sha256.clone(),
        slide: "/ppt/slides/slide1.xml".into(),
        profile: SourcePageProfile::StaticSolidDraftV1,
        color_context: Default::default(),
        viewport: RasterViewport {
            width: 915,
            height: 515,
            origin: point(0., 0.).unwrap(),
            scale: PixelScale {
                numerator: 1,
                denominator: 10000,
            },
            coordinate_tolerance: Fixed::from_raw(1 << 24),
            background: [255; 4],
        },
    };
    let options = ResourcePageOptions {
        selection: ImageSourceSelection::EmbeddedSnapshot,
        sampling: ImageSampling::Nearest,
        text_limits: Default::default(),
    };
    let expected = crate::source_resource_page::prepare(
        &package,
        &index,
        &q,
        &mut Decoder,
        None,
        options,
        &|| false,
    )
    .unwrap()
    .render(&mut NativeRaster::default(), &|| false)
    .unwrap();
    let plan =
        ResourcePagePlan::new(&package, index, q, &mut Decoder, None, options, &|| false).unwrap();
    let empty = crate::source_placement::SourceProperties::new();
    let base = plan
        .prepare_sampled(&empty, &|| false)
        .unwrap()
        .render(&mut NativeRaster::default(), &|| false)
        .unwrap();
    assert_eq!(expected.pixels, base.pixels);
    let mut hidden = empty.clone();
    hidden.insert(
        ("/ppt/slides/slide1.xml".into(), 2),
        crate::sampled_properties::SampledProperties {
            visibility: Some(mo_timeline::Visibility::Hidden),
            ..Default::default()
        },
    );
    let hidden = plan
        .prepare_sampled(&hidden, &|| false)
        .unwrap()
        .render(&mut NativeRaster::default(), &|| false)
        .unwrap();
    assert!(hidden.info.page.page.charts.is_empty());
    assert_ne!(hidden.pixels, base.pixels);
    let mut moved = empty;
    moved.insert(
        ("/ppt/slides/slide1.xml".into(), 2),
        crate::sampled_properties::SampledProperties {
            motion: Some([
                crate::interval::Interval::ratio(1, 10),
                crate::interval::Interval::ratio(0, 1),
            ]),
            ..Default::default()
        },
    );
    let moved = plan
        .prepare_sampled(&moved, &|| false)
        .unwrap()
        .render(&mut NativeRaster::default(), &|| false)
        .unwrap();
    assert_eq!(
        moved.info.page.page.charts[0].chart_sha256,
        base.info.page.page.charts[0].chart_sha256
    );
    assert_ne!(moved.pixels, base.pixels);
}
