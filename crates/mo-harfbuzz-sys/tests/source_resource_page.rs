#[allow(dead_code)]
#[path = "../../../tools/test-support/source_resource_page.rs"]
mod support;
use mo_harfbuzz_sys::NativeShaper;
use mo_image::{DecoderReply, ImageDecoder, ImageError};
use mo_opc::{Package, PackageLimits};
use mo_presentation_compile::source_resource_page::*;
use mo_skia_sys::NativeRaster;
use mo_text::manifest::{ManifestLimits, PreparedManifest};
use std::path::Path;
use support::*;
#[derive(Default)]
struct Decoder {
    calls: u32,
}
impl ImageDecoder for Decoder {
    fn decode(&mut self, b: &[u8]) -> Result<DecoderReply, ImageError> {
        self.calls += 1;
        NativeRaster.decode(b)
    }
    fn invalidate(&mut self) {
        NativeRaster.invalidate()
    }
}
struct Probe {
    plan: SourceResourcePagePlan,
    pixels: Vec<u8>,
}
fn image(name: &str, b: &[u8]) -> Probe {
    let package = Package::open(b, b.len() as u64, PackageLimits::default(), &|| false).unwrap();
    let index = read(b);
    let original = index.clone();
    let q = request(&index);
    let author = author();
    let manifest = PreparedManifest::load(
        &author.manifest,
        include_bytes!("../../../fixtures/fonts/owned.ttf"),
        ManifestLimits::default(),
        &|| false,
    )
    .unwrap();
    let mut decoder = Decoder::default();
    let mut text = NativeShaper::default();
    let plan = prepare(
        &package,
        &index,
        &q,
        &mut decoder,
        Some(TextPageContext {
            manifest: &manifest,
            backend: &mut text,
        }),
        options(),
        &|| false,
    )
    .unwrap()
    .plan(&|| false)
    .unwrap();
    assert_eq!(decoder.calls as usize, plan.images.decoded.len());
    let mut decoder = Decoder::default();
    let mut text = NativeShaper::default();
    let rendered = prepare(
        &package,
        &index,
        &q,
        &mut decoder,
        Some(TextPageContext {
            manifest: &manifest,
            backend: &mut text,
        }),
        options(),
        &|| false,
    )
    .unwrap()
    .render(&mut NativeRaster, &|| false)
    .unwrap();
    assert_eq!(decoder.calls as usize, plan.images.decoded.len());
    assert_eq!(rendered.info.resources_sha256, plan.resources_sha256);
    assert_eq!(
        rendered.info.page.downstream_coordinate_error_bound,
        plan.page.downstream_coordinate_error_bound
    );
    assert!(
        rendered.info.page.downstream_coordinate_error_bound <= q.viewport.coordinate_tolerance
    );
    assert_eq!(
        rendered.info.text_frames as usize,
        plan.text.as_ref().unwrap().texts.len()
    );
    assert_eq!(index, original);
    if let Some(dir) = std::env::var_os("MO_RESOURCE_PAGE_EVIDENCE_DIR") {
        let dir = Path::new(&dir);
        assert!(dir.is_absolute());
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(format!("{name}.pptx")), b).unwrap();
        std::fs::write(
            dir.join(format!("{name}.plan.json")),
            serde_json::to_vec_pretty(&plan).unwrap(),
        )
        .unwrap();
        std::fs::write(
            dir.join(format!("{name}.info.json")),
            serde_json::to_vec_pretty(&rendered.info).unwrap(),
        )
        .unwrap();
        std::fs::write(dir.join(format!("{name}.rgba")), &rendered.pixels).unwrap();
    }
    Probe {
        plan,
        pixels: rendered.pixels,
    }
}
fn pixel(probe: &Probe, x: usize, y: usize) -> &[u8] {
    &probe.pixels[(400 * y + x) * 4..(400 * y + x + 1) * 4]
}
fn pic(id: u32) -> String {
    picture(id, "", &blip("owned-image", "", STRETCH), &solid("00AA00"))
}
#[test]
fn real_native_images_shapes_text_and_clips_share_one_page() {
    // The pinned text component has process-global font state; test in an
    // isolated process as all other native text/page integration tests do.
    let name = "real_native_images_shapes_text_and_clips_share_one_page";
    if std::env::var("MO_RESOURCE_PAGE_CHILD").ok().as_deref() != Some(name) {
        let r = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", name, "--test-threads=1"])
            .env("MO_RESOURCE_PAGE_CHILD", name)
            .output()
            .unwrap();
        assert!(
            r.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&r.stdout),
            String::from_utf8_lossy(&r.stderr)
        );
        return;
    }
    let b = image_fixture(&pic(42));
    let p = image("dual-fill", &b);
    // Independent interior colors from the original 2x2 PNG and XML fill.
    for (x, y, color) in [
        (25, 25, [255, 0, 0, 255]),
        (125, 25, [0, 170, 0, 255]),
        (25, 125, [0, 0, 255, 255]),
        (125, 125, [255, 255, 0, 255]),
        (250, 250, [255, 255, 255, 255]),
    ] {
        assert_eq!(pixel(&p, x, y), color, "dual fill pixel {x},{y}");
    }
    let clipped = picture(
        42,
        "",
        &blip(
            "owned-image",
            "",
            "<a:stretch><a:fillRect l=\"25000\" r=\"25000\"/></a:stretch>",
        ),
        &solid("00AA00"),
    );
    let p = image("stretch-clip", &image_fixture(&clipped));
    for (x, y, color) in [
        (25, 25, [0, 170, 0, 255]),
        (75, 25, [255, 0, 0, 255]),
        (125, 25, [0, 170, 0, 255]),
        (175, 125, [0, 170, 0, 255]),
        (75, 125, [0, 0, 255, 255]),
    ] {
        assert_eq!(pixel(&p, x, y), color, "clip pixel {x},{y}");
    }
    let image_fill = blip("owned-copy", "", STRETCH);
    let label = shape(43, 900000, 100000, "", &colored("AA", "C02080"))
        .replace(&solid("F4EADC"), &image_fill);
    let p = image("image-text", &image_fixture(&(pic(42) + &label)));
    assert_eq!(p.plan.images.decoded.len(), 1);
    assert_eq!(p.plan.images.bindings.len(), 2);
    assert_eq!(p.plan.text.as_ref().unwrap().texts.len(), 1);
    let first_glyph = p.plan.text.as_ref().unwrap().text_sources[0].instance;
    let shape_image = p
        .plan
        .page
        .paint_sources
        .iter()
        .filter(|s| s.binding == 2 && s.path.is_some())
        .map(|s| s.instance)
        .max()
        .unwrap();
    assert!(first_glyph > shape_image);
    assert!(p.pixels.chunks_exact(4).any(|c| c == [192, 32, 128, 255]));
    let foreground = shape(44, 0, 0, "", &colored("A", "F08000"));
    let p = image("foreground-text", &image_fixture(&(pic(42) + &foreground)));
    assert_eq!(pixel(&p, 200, 140), [244, 234, 220, 255]);
    assert!(p.pixels.chunks_exact(4).any(|c| c == [240, 128, 0, 255]));
    // Two different encoded resources, with one packed bundle and a common
    // scene, still respect source object order at an opaque foreground picture.
    let cyan = picture(43, "", &blip("owned-cyan", "", STRETCH), "<a:noFill/>");
    let p = image("distinct-images", &image_fixture(&(pic(42) + &cyan)));
    assert_eq!(p.plan.images.decoded.len(), 2);
    assert_eq!(p.plan.images.gather_copy_bytes, 32);
    assert_eq!(pixel(&p, 25, 25), [0, 255, 255, 255]);
    // Non-rectangular shape masks must still clip the image independently of
    // the stretch rectangle; the path remains native and editable in source.
    let ellipse = pic(42).replace("prst=\"rect\"", "prst=\"ellipse\"");
    let p = image("ellipse", &image_fixture(&ellipse));
    assert_eq!(pixel(&p, 5, 5), [255, 255, 255, 255]);
    assert_eq!(pixel(&p, 80, 80), [255, 0, 0, 255]);
    for (i, extra) in [
        "rot=\"2700000\"",
        "rot=\"5400000\" flipH=\"1\"",
        "rot=\"16200000\" flipV=\"1\"",
    ]
    .into_iter()
    .enumerate()
    {
        for (mode, fill) in [
            ("stretch", blip("owned-image", "", STRETCH)),
            (
                "tile",
                blip(
                    "owned-image",
                    "dpi=\"10\"",
                    "<a:tile algn=\"ctr\" flip=\"xy\"/>",
                ),
            ),
        ] {
            let body = picture(42, extra, &fill, &solid("00AA00"));
            let p = image(&format!("orientation-{i}-{mode}"), &image_fixture(&body));
            assert!(p.pixels.chunks_exact(4).any(|v| v == [255, 0, 0, 255]));
            assert_eq!(p.plan.image_work.resources, 1);
        }
    }
    let b = image_fixture(&pic(42));
    let b = rewrite(&b, SLIDE, |s| {
        s.replace(
            &format!("<p:bgPr>{}</p:bgPr>", solid("FFFFFF")),
            &format!(
                "<p:bgPr>{}</p:bgPr>",
                blip("owned-copy", "rotWithShape=\"0\"", STRETCH)
            ),
        )
    });
    let p = image("background-shared", &b);
    assert_eq!(p.plan.images.decoded.len(), 1);
    assert_eq!(p.plan.images.bindings.len(), 2);
    assert_eq!(pixel(&p, 250, 250), [255, 255, 0, 255]);
    let group = format!(
        "<p:grpSp><p:nvGrpSpPr><p:cNvPr id=\"90\" name=\"Owned group\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm rot=\"1800000\" flipH=\"1\"><a:off x=\"100000\" y=\"200000\"/><a:ext cx=\"1000000\" cy=\"800000\"/><a:chOff x=\"-200000\" y=\"100000\"/><a:chExt cx=\"1200000\" cy=\"1000000\"/></a:xfrm></p:grpSpPr>{}</p:grpSp>",
        pic(42)
    );
    let p = image("group-direct-image", &image_fixture(&group));
    assert_eq!(p.plan.images.bindings.len(), 1);
    assert!(p.pixels.chunks_exact(4).any(|c| c == [255, 0, 0, 255]));
    let mut b = image_fixture(&pic(42));
    b = rewrite(&b, SLIDE, |s| {
        s.replace("showMasterSp=\"0\"", "showMasterSp=\"1\"")
    });
    for (part, id) in [
        ("/ppt/slideLayouts/slideLayout2.xml", 52),
        ("/ppt/slideMasters/slideMaster2.xml", 53),
    ] {
        b = rewrite(&b, part, |mut s| {
            let start = s.find("<p:spTree>").unwrap();
            let end = s.find("</p:spTree>").unwrap() + 11;
            s.replace_range(start..end,&format!("<p:spTree><p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>{}</p:spTree>",pic(id)));
            s
        });
    }
    let p = image("master-layout-slide-images", &b);
    assert_eq!(p.plan.images.bindings.len(), 3);
    assert_eq!(p.plan.images.decoded.len(), 1);
    assert_eq!(
        p.plan
            .page
            .bindings
            .iter()
            .skip(1)
            .map(|b| b.location.object.unwrap())
            .collect::<Vec<_>>(),
        [53, 52, 42]
    );
}
