#[allow(dead_code)]
#[path = "../../../tools/test-support/source_background_page.rs"]
mod support;
use mo_opc::{Package, PackageLimits};
use mo_presentation_compile::source_resource_page::*;
use mo_skia_sys::NativeRaster;
use std::path::Path;
use support::*;

fn render(name: &str, bytes: &[u8], clear: [u8; 4]) -> SourceResourcePageImage {
    let package = Package::open(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })
    .unwrap();
    let index = read(bytes);
    let mut q = request(&index);
    q.viewport.background = clear;
    let original = serde_json::to_value(&index).unwrap();
    let result = prepare(
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
    .unwrap_or_else(|e| panic!("{name}: {e:?}"));
    assert_eq!(original, serde_json::to_value(index).unwrap());
    assert_eq!(result.info.gather_copy_bytes, 0);
    if let Some(dir) = std::env::var_os("MO_BACKGROUND_EVIDENCE_DIR") {
        let dir = Path::new(&dir);
        assert!(dir.is_absolute());
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(format!("{name}.pptx")), bytes).unwrap();
        std::fs::write(dir.join(format!("{name}.rgba")), &result.pixels).unwrap();
        std::fs::write(
            dir.join(format!("{name}.request.json")),
            serde_json::to_vec_pretty(&q).unwrap(),
        )
        .unwrap();
        std::fs::write(
            dir.join(format!("{name}.info.json")),
            serde_json::to_vec_pretty(&result.info).unwrap(),
        )
        .unwrap();
    }
    result
}
#[test]
fn native_background_windows_restore_background_pixels_without_foreground_leaks() {
    for case in cases() {
        let image = render(case.name, &case.source, case.clear);
        let background = render(
            &format!("{}-background", case.name),
            &case.background,
            case.clear,
        );
        let mask = render(&format!("{}-mask", case.name), &case.mask, [255; 4]);
        assert_eq!(
            image.info.decoded_images.len() as u32,
            case.images,
            "{}",
            case.name
        );
        assert_eq!(image.info.images.draws, case.images);
        let work = &image.info.page.scene.raster.work.compositing;
        if case.captures == 0 {
            assert!(work.is_none());
        } else {
            let work = work.as_ref().unwrap();
            assert_eq!(work.captures, case.captures);
            assert_eq!(work.captured_bytes, 400 * 300 * 4);
            assert_eq!(work.snapshot_draws, case.windows);
            assert_eq!(work.source_draws, case.windows);
        }
        let mut inside = 0;
        for (i, m) in mask.pixels.chunks_exact(4).enumerate() {
            // A quantized black/white mask is not an exact AA coverage oracle.
            // Require a constant 3x3 neighbourhood for this interior check;
            // direct compositor tests separately compare the full AA edges.
            let x = i % 400;
            let y = i / 400;
            if x == 0
                || x == 399
                || y == 0
                || y == 299
                || !(y - 1..=y + 1).all(|yy| {
                    (x - 1..=x + 1)
                        .all(|xx| &mask.pixels[(yy * 400 + xx) * 4..(yy * 400 + xx + 1) * 4] == m)
                })
            {
                continue;
            }
            let pixel = &image.pixels[i * 4..i * 4 + 4];
            if m == [255; 4] {
                assert_eq!(
                    pixel,
                    &background.pixels[i * 4..i * 4 + 4],
                    "{} interior {},{}",
                    case.name,
                    i % 400,
                    i / 400
                );
                inside += 1;
            } else if m == [0, 0, 0, 255] {
                assert_eq!(
                    pixel,
                    [0, 170, 0, 255],
                    "{} outside {},{}",
                    case.name,
                    i % 400,
                    i / 400
                );
            }
        }
        assert_eq!(inside == 0, case.windows == 0);
        let pixel = |x: usize, y: usize| &image.pixels[(y * 400 + x) * 4..(y * 400 + x + 1) * 4];
        match case.name {
            "alpha-clear" => assert_eq!(pixel(50, 75), [128, 0, 0, 128]),
            "alpha-opaque" => assert_eq!(pixel(50, 75), [255, 127, 127, 255]),
            "empty-clear" => assert_eq!(pixel(50, 75), [0; 4]),
            "image-clear" => {
                assert_eq!(pixel(50, 75), [255, 0, 0, 255]);
                assert_eq!(pixel(225, 75), [0; 4]);
                assert_eq!(pixel(50, 175), [0, 0, 255, 255]);
                assert_eq!(pixel(225, 175), [255, 255, 0, 255]);
            }
            _ => {}
        }
    }
}
