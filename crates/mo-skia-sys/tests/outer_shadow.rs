use mo_common::Emu;
use mo_geometry::{Fixed, PathCommand as C, Point};
use mo_raster::*;
use mo_skia_sys::NativeRaster;
fn f(v: i64) -> Fixed {
    Fixed::emu(Emu::new(v))
}
fn p(x: i64, y: i64) -> Point {
    Point { x: f(x), y: f(y) }
}
fn request() -> PathRasterRequest {
    PathRasterRequest {
        viewport: RasterViewport {
            width: 64,
            height: 64,
            origin: p(0, 0),
            scale: PixelScale {
                numerator: 1,
                denominator: 1,
            },
            coordinate_tolerance: Fixed::from_raw(1 << 24),
            background: [0; 4],
        },
        paths: vec![FillPath {
            fill_rule: FillRule::Nonzero,
            commands: vec![
                C::Move { to: p(16, 16) },
                C::Line { to: p(32, 16) },
                C::Line { to: p(32, 32) },
                C::Line { to: p(16, 32) },
                C::Close,
            ],
        }],
        clips: vec![],
        draws: vec![PathDraw {
            blend: Default::default(),
            clip: None,
            path: 0,
            origin: p(0, 0),
            brush: Brush::Solid {
                rgba: [200, 40, 20, 255],
            },
            stroke: None,
        }],
        opacity_groups: vec![OpacityGroup {
            first_draw: 0,
            end_draw: 1,
            opacity: 65535,
            outer_shadow: Some(OuterShadow {
                offset: p(8, 8),
                sigma: f(0),
                rgba: [0, 0, 0, 128],
            }),
        }],
    }
}
fn pixels(q: &PathRasterRequest, label: &str) -> Vec<u8> {
    let compiled = compile(q, &|| false).unwrap();
    assert_eq!(compiled.frame()[1], 15);
    let frame = compiled.frame().to_vec();
    let image = render_compiled(compiled, &mut NativeRaster, &|| false).unwrap();
    if let Some(out) = std::env::var_os("MO_SHADOW_FIXTURES") {
        let root = std::path::PathBuf::from(out);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join(format!("{label}.json")),
            serde_json::to_vec(&serde_json::json!({"frame":frame,"pixels":image.pixels})).unwrap(),
        )
        .unwrap();
    }
    image.pixels
}
fn rgba(pixels: &[u8], x: usize, y: usize) -> &[u8] {
    &pixels[(y * 64 + x) * 4..(y * 64 + x + 1) * 4]
}
#[test]
fn shadow_is_behind_complete_object_and_group_opacity_is_applied_once() {
    let mut q = request();
    let image = pixels(&q, "translated");
    assert_eq!(rgba(&image, 20, 20), [200, 40, 20, 255]);
    assert_eq!(rgba(&image, 36, 36), [0, 0, 0, 128]);
    assert_eq!(rgba(&image, 44, 44), [0, 0, 0, 0]);
    q.draws.push(q.draws[0].clone());
    q.opacity_groups[0].end_draw = 2;
    q.opacity_groups[0].opacity = 32768;
    let image = pixels(&q, "whole-object-opacity");
    assert_eq!(rgba(&image, 20, 20), [100, 20, 10, 128]);
    assert_eq!(rgba(&image, 36, 36), [0, 0, 0, 64]);
}
#[test]
fn gaussian_has_symmetric_falloff_and_fractional_translation() {
    let mut q = request();
    let s = q.opacity_groups[0].outer_shadow.as_mut().unwrap();
    s.sigma = f(2);
    s.offset = p(0, 0);
    s.rgba = [0, 0, 255, 255];
    let image = pixels(&q, "gaussian");
    assert!(rgba(&image, 14, 24)[3] > rgba(&image, 12, 24)[3]);
    assert!(rgba(&image, 12, 24)[3] > 0);
    assert_eq!(rgba(&image, 12, 24)[3], rgba(&image, 35, 24)[3]);
    assert_eq!(rgba(&image, 12, 24)[2], rgba(&image, 12, 24)[3]);
    let s = q.opacity_groups[0].outer_shadow.as_mut().unwrap();
    s.sigma = f(0);
    s.offset = Point {
        x: Fixed::from_raw(17 << 31),
        y: f(0),
    };
    let image = pixels(&q, "fractional");
    assert!((127..=128).contains(&rgba(&image, 40, 24)[3]));
}
#[test]
fn nested_and_sibling_scopes_preserve_shadow_and_foreground() {
    let mut q = request();
    q.draws.push(q.draws[0].clone());
    q.draws[1].origin = p(16, 0);
    let mut first = q.opacity_groups[0].clone();
    first.outer_shadow.as_mut().unwrap().rgba = [0, 0, 0, 255];
    let mut second = first.clone();
    second.first_draw = 1;
    second.end_draw = 2;
    q.opacity_groups = vec![
        OpacityGroup {
            first_draw: 0,
            end_draw: 2,
            opacity: 32768,
            outer_shadow: None,
        },
        first,
        second,
    ];
    let image = pixels(&q, "nested-siblings");
    assert_eq!(rgba(&image, 52, 36), [0, 0, 0, 128]);
    assert_eq!(rgba(&image, 36, 24), [100, 20, 10, 128]);
}
#[test]
fn shadow_limits_and_untrusted_wire_are_checked_before_backend_allocations() {
    let mut q = request();
    q.opacity_groups[0].outer_shadow.as_mut().unwrap().sigma = f(33);
    assert!(matches!(compile(&q, &|| false), Err(RasterError::Limit(_))));
    q = request();
    q.viewport.width = 4096;
    q.viewport.height = 2048;
    assert!(matches!(compile(&q, &|| false), Err(RasterError::Limit(_))));
    q = request();
    let mut frame = compile(&q, &|| false).unwrap().frame().to_vec();
    // One path (2+5*7 words), no paints/clips/resources; group follows header.
    let effect = 15 + 2 + 5 * 7 + 3;
    frame[effect + 4] = f32::NAN.to_bits();
    let r = NativeRaster.raster(&frame).unwrap();
    assert_eq!(r.status, 1);
    assert!(r.pixels.is_empty());
}
