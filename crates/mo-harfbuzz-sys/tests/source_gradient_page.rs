#[allow(dead_code)]
#[path = "../../../tools/test-support/source_gradient_page.rs"]
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
        if name == "hard-stop" {
            mo_raster::GRADIENT_PLANE_PROFILE
        } else {
            mo_raster::OFFICE_GRADIENT_PROFILE
        }
    );
    if let Some(dir) = std::env::var_os("MO_GRADIENT_EVIDENCE_DIR") {
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
fn native_gradients_keep_receiver_coordinates_stops_and_background_space() {
    let mut results = std::collections::BTreeMap::new();
    for c in cases() {
        let r = render(&c.name, &c.source);
        if let Some(control) = c.equivalent {
            assert_eq!(
                r.pixels,
                render(&format!("{}-control", c.name), &control).pixels
            );
        }
        results.insert(c.name, r);
    }
    for name in ["tile-none", "tile-x"] {
        assert_eq!(results[name].pixels, results["tile-xy"].pixels);
    }
    assert_ne!(results["scaled-30"].pixels, results["unscaled-30"].pixels);
    let bg = &results["background"].pixels;
    let win = &results["background-window"].pixels;
    for y in 50..200 {
        for x in 50..300 {
            let i = (y * 400 + x) * 4;
            assert_eq!(&bg[i..i + 4], &win[i..i + 4]);
        }
    }
    for (name, color) in [("right", [255u8, 0, 0, 255]), ("left", [0, 0, 255, 255])] {
        let r = &results[name].pixels;
        let i = (100 * 400 + 25) * 4;
        for c in 0..4 {
            assert!((i16::from(r[i + c]) - i16::from(color[c])).abs() <= 1);
        }
    }
}
#[test]
fn unsupported_gradient_geometry_is_rejected_before_decode_and_raster() {
    struct Never;
    impl mo_image::ImageDecoder for Never {
        fn decode(&mut self, _: &[u8]) -> Result<mo_image::DecoderReply, mo_image::ImageError> {
            panic!("decode before gradient preflight")
        }
        fn invalidate(&mut self) {}
    }
    let fill = linear(0, false, "rotWithShape=\"0\"", "", STOPS);
    for fill in [
        fill.clone(),
        fill.replace(
            "<a:lin ang=\"0\" scaled=\"0\"/>",
            "<a:path path=\"circle\"/>",
        ),
    ] {
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
