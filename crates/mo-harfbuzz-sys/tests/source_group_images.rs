#[allow(dead_code)]
#[path = "../../../tools/test-support/source_group_images.rs"]
mod support;
use mo_opc::{Package, PackageLimits};
use mo_presentation_compile::source_resource_page::*;
use mo_skia_sys::NativeRaster;
use std::path::Path;
use support::*;

fn render(name: &str, b: &[u8]) -> SourceResourcePageImage {
    let package = Package::open(b, b.len() as u64, PackageLimits::default(), &|| false).unwrap();
    let i = read(b);
    let original = i.clone();
    let result = prepare(
        &package,
        &i,
        &request(&i),
        &mut NativeRaster,
        None,
        options(),
        &|| false,
    )
    .unwrap_or_else(|e| panic!("{name}: {e:?}"))
    .render(&mut NativeRaster, &|| false)
    .unwrap();
    assert_eq!(i, original);
    assert_eq!(result.info.decoded_images.len(), 1);
    assert_eq!(result.info.gather_copy_bytes, 0);
    assert!(
        result.info.page.downstream_coordinate_error_bound
            <= request(&i).viewport.coordinate_tolerance
    );
    if let Some(dir) = std::env::var_os("MO_GROUP_IMAGE_EVIDENCE_DIR") {
        let dir = Path::new(&dir);
        assert!(dir.is_absolute());
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(format!("{name}.pptx")), b).unwrap();
        std::fs::write(dir.join(format!("{name}.rgba")), &result.pixels).unwrap();
        std::fs::write(
            dir.join(format!("{name}.info.json")),
            serde_json::to_vec_pretty(&result.info).unwrap(),
        )
        .unwrap();
    }
    result
}
#[test]
fn inherited_group_properties_render_as_explicit_receiver_fills() {
    for c in cases() {
        let inherited = render(c.name, &c.inherited);
        let explicit = render(&format!("{}-explicit", c.name), &c.explicit);
        assert_eq!(inherited.pixels, explicit.pixels, "{}", c.name);
        assert_eq!(inherited.info.images.draws as usize, c.expected_image_uses);
        if c.name == "siblings" {
            for (x, y, color) in [
                (50, 50, [255, 0, 0, 255]),
                (150, 50, [255; 4]),
                (250, 50, [255, 0, 0, 255]),
                (350, 50, [255; 4]),
                (50, 150, [0, 0, 255, 255]),
                (150, 150, [255, 255, 0, 255]),
                (250, 150, [0, 0, 255, 255]),
                (350, 150, [255, 255, 0, 255]),
            ] {
                assert_eq!(
                    &inherited.pixels[(y * 400 + x) * 4..(y * 400 + x + 1) * 4],
                    color
                );
            }
        }
    }
}
