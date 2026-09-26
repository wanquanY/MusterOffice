#[allow(dead_code)]
#[path = "../../../tools/test-support/office_gradient.rs"]
mod support;
use mo_opc::{Package, PackageLimits};
use mo_presentation_compile::source_resource_page::*;
use mo_skia_sys::NativeRaster;
use support::*;

#[test]
fn source_selects_gamma_only_for_exact_eligible_ramps_and_keeps_native_content() {
    for (name, source, office) in office_cases() {
        let package = Package::open(
            &source,
            source.len() as u64,
            PackageLimits::default(),
            &|| false,
        )
        .unwrap();
        let index = read(&source);
        let q = request(&index);
        let before = serde_json::to_value(&index).unwrap();
        let image = prepare(
            &package,
            &index,
            &q,
            &mut NativeRaster,
            None,
            options(),
            &|| false,
        )
        .unwrap()
        .render(&mut NativeRaster, &|| false)
        .unwrap();
        assert_eq!(before, serde_json::to_value(&index).unwrap());
        assert_eq!(
            image.info.page.scene.raster.profile,
            if office {
                mo_raster::OFFICE_GRADIENT_PROFILE
            } else {
                mo_raster::GRADIENT_PLANE_PROFILE
            },
            "{name}"
        );
        if let Some(dir) = std::env::var_os("MO_OFFICE_GRADIENT_DIR") {
            let dir = std::path::Path::new(&dir);
            assert!(dir.is_absolute());
            std::fs::create_dir_all(dir).unwrap();
            for (ext, bytes) in [
                ("pptx", source),
                ("request.json", serde_json::to_vec(&q).unwrap()),
                ("info.json", serde_json::to_vec(&image.info).unwrap()),
                ("rgba", image.pixels),
            ] {
                std::fs::write(dir.join(format!("{name}.{ext}")), bytes).unwrap();
            }
        }
    }
}
