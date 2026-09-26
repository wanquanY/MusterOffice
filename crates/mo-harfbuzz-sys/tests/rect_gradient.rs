#[allow(dead_code)]
#[path = "../../../tools/test-support/rect_gradient.rs"]
mod support;
use mo_opc::{Package, PackageLimits};
use mo_presentation_compile::source_resource_page::*;
use mo_skia_sys::NativeRaster;
use support::*;
fn render(name: &str, bytes: &[u8]) -> SourceResourcePageImage {
    let package = Package::open(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })
    .unwrap();
    let index = read(bytes);
    let q = request(&index);
    let before = serde_json::to_value(&index).unwrap();
    let r = prepare(
        &package,
        &index,
        &q,
        &mut NativeRaster,
        None,
        options(),
        &|| false,
    )
    .unwrap_or_else(|e| panic!("{name}: {e:?}"))
    .render(&mut NativeRaster, &|| false)
    .unwrap();
    assert_eq!(before, serde_json::to_value(index).unwrap());
    assert_eq!(r.info.decoded_images.len(), 0);
    assert_eq!(
        r.info.page.scene.raster.profile,
        mo_raster::RECT_GRADIENT_PROFILE
    );
    if let Some(dir) = std::env::var_os("MO_RECT_GRADIENT_DIR") {
        let dir = std::path::Path::new(&dir);
        assert!(dir.is_absolute());
        std::fs::create_dir_all(dir).unwrap();
        for (ext, bytes) in [
            ("pptx", bytes.to_vec()),
            ("rgba", r.pixels.clone()),
            ("request.json", serde_json::to_vec_pretty(&q).unwrap()),
            ("info.json", serde_json::to_vec_pretty(&r.info).unwrap()),
        ] {
            std::fs::write(dir.join(format!("{name}.{ext}")), bytes).unwrap();
        }
    }
    r
}
#[test]
fn rectangular_gradients_keep_focus_fields_and_receiver_transforms() {
    let mut results = std::collections::BTreeMap::new();
    for c in rect_cases() {
        let r = render(&c.name, &c.source);
        if let Some(control) = c.equivalent {
            assert_eq!(
                r.pixels,
                render(&format!("{}-control", c.name), &control).pixels
            );
        }
        results.insert(c.name, r);
    }
    let bg = &results["background"].pixels;
    let win = &results["background-window"].pixels;
    for y in 50..200 {
        for x in 50..300 {
            let i = (y * 400 + x) * 4;
            assert_eq!(&bg[i..i + 4], &win[i..i + 4]);
        }
    }
}

#[test]
fn inverted_rectangular_focus_is_rejected_before_decode() {
    struct Never;
    impl mo_image::ImageDecoder for Never {
        fn decode(&mut self, _: &[u8]) -> Result<mo_image::DecoderReply, mo_image::ImageError> {
            panic!("decode before gradient preflight")
        }
        fn invalidate(&mut self) {}
    }
    for rect in [
        "l=\"80%\" r=\"80%\"",
        "t=\"50%\" b=\"50.000000000000000001%\"",
    ] {
        let fill = rectangular(rect, "", STOPS);
        let source = image_fixture(
            &(picture(41, "", &blip("owned-image", "", STRETCH), "<a:noFill/>")
                + &receiver(42, [0, 0, 800000, 600000], "", &fill)),
        );
        let package = Package::open(
            &source,
            source.len() as u64,
            PackageLimits::default(),
            &|| false,
        )
        .unwrap();
        let index = read(&source);
        let q = request(&index);
        assert!(prepare(&package, &index, &q, &mut Never, None, options(), &|| false).is_err());
    }
}
