use super::*;
use mo_common::{Digest, Emu};
use mo_image::{ImageFormat, ImageResolution, SourceColor};
use mo_presentation_source::source::fill::{NativeBlipCompression, resolve::*};
use std::cell::Cell;
mod regions;

fn value<T>(value: T) -> FillValue<T> {
    FillValue {
        value,
        declared_by: FillOrigin::ProfileDefault {},
    }
}
fn rect(edges: [&str; 4]) -> EffectiveFillRect {
    let [l, t, r, b] = edges.map(|s| value(s.to_owned().try_into().unwrap()));
    EffectiveFillRect {
        declared_by: FillOrigin::ProfileDefault {},
        left: l,
        top: t,
        right: r,
        bottom: b,
    }
}
fn fill() -> EffectiveImageFill {
    EffectiveImageFill {
        embed: value("rId1".into()),
        link: value(String::new()),
        compression: value(NativeBlipCompression::None),
        source_rect: rect(["0"; 4]),
        mode: EffectiveImageMode::Stretch {
            declared_by: FillOrigin::ProfileDefault {},
            fill_rect: rect(["0"; 4]),
        },
        dpi: value(0),
        rotate_with_shape: value(true),
    }
}
fn image() -> DecodedImageInfo {
    DecodedImageInfo {
        profile: mo_image::PROFILE.into(),
        source_sha256: Digest::from_sha256([0; 32]),
        pixels_sha256: Digest::from_sha256([0; 32]),
        width: 100,
        height: 80,
        encoded_width: 100,
        encoded_height: 80,
        orientation: 1,
        format: ImageFormat::Png,
        encoded_bit_depth: 8,
        source_color: SourceColor::AssumedSrgb,
        byte_length: 32000,
        resolution: ImageResolution {
            declarations: vec![],
            physical_pixel_size: PhysicalPixelSize::Unspecified,
        },
    }
}
fn size() -> Size {
    Size {
        width: Emu::new(1000),
        height: Emu::new(800),
    }
}
fn raw(n: i64) -> Fixed {
    Fixed::emu(Emu::new(n))
}
fn tile(alignment: Alignment, flip: NativeTileFlip) -> EffectiveImageMode {
    EffectiveImageMode::Tile {
        declared_by: FillOrigin::ProfileDefault {},
        tile: EffectiveFillTile {
            translate_x: value("-0.001in".to_owned().try_into().unwrap()),
            translate_y: value("0.002cm".to_owned().try_into().unwrap()),
            scale_x: value("50%".to_owned().try_into().unwrap()),
            scale_y: value("200000".to_owned().try_into().unwrap()),
            flip: value(flip),
            alignment: value(alignment),
        },
    }
}
#[test]
fn crop_and_stretch_keep_separate_source_and_hard_clip_rectangles() {
    let mut f = fill();
    f.source_rect = rect(["25%", "12500", "25000", "12500"]);
    f.mode = EffectiveImageMode::Stretch {
        declared_by: FillOrigin::ProfileDefault {},
        fill_rect: rect(["10000", "25000", "30000", "25000"]),
    };
    let before = serde_json::to_value(&f).unwrap();
    let p = layout(&f, &image(), size(), &|| false).unwrap();
    assert_eq!(
        [
            p.source_rectangle.left,
            p.source_rectangle.top,
            p.source_rectangle.right,
            p.source_rectangle.bottom
        ],
        [raw(25), raw(10), raw(75), raw(70)]
    );
    assert_eq!(
        [
            p.fill_rectangle.left,
            p.fill_rectangle.top,
            p.fill_rectangle.right,
            p.fill_rectangle.bottom
        ],
        [raw(100), raw(200), raw(700), raw(600)]
    );
    assert_eq!(p.pixel_step.x, raw(12));
    assert_eq!(p.origin.x, raw(-200));
    assert!(p.clip_to_fill_rectangle);
    assert!(matches!(p.density, ImageLayoutDensity::NotRequired));
    assert_eq!(serde_json::to_value(f).unwrap(), before);
    assert!(p.uncertainty.pixel_step.x.raw() <= 1);
    assert_eq!(p.uncertainty.pixel_step.y.raw(), 1);
}
#[test]
fn tile_scales_crop_then_aligns_then_offsets_and_preserves_axis_flips() {
    let aligns = [
        (Alignment::TopLeft, [0, 0]),
        (Alignment::Top, [1, 0]),
        (Alignment::TopRight, [2, 0]),
        (Alignment::Left, [0, 1]),
        (Alignment::Center, [1, 1]),
        (Alignment::Right, [2, 1]),
        (Alignment::BottomLeft, [0, 2]),
        (Alignment::Bottom, [1, 2]),
        (Alignment::BottomRight, [2, 2]),
    ];
    for (a, align) in aligns {
        for flip in [
            NativeTileFlip::None,
            NativeTileFlip::X,
            NativeTileFlip::Y,
            NativeTileFlip::Xy,
        ] {
            let mut f = fill();
            f.dpi = value(91440);
            f.mode = tile(a, flip);
            f.source_rect = rect(["25000", "25000", "25000", "25000"]);
            let p = layout(&f, &image(), size(), &|| false).unwrap();
            assert_eq!(
                p.pixel_step,
                Point {
                    x: raw(5),
                    y: raw(20)
                }
            );
            // Crop has 50x40 pixels -> 250x800 EMU. Offsets are -914.4, +720.
            let left = (i128::from(align[0]) * 375 * 5 - 4572) * (1i128 << 32) / 5;
            let error = (p.fill_rectangle.left.raw() - left).abs();
            assert!(error <= 1);
            assert_eq!(p.fill_rectangle.top, raw(720));
            assert_eq!(
                p.fill_rectangle.right.raw() - p.fill_rectangle.left.raw(),
                raw(250).raw()
            );
            assert_eq!(
                p.origin.x.raw() - p.fill_rectangle.left.raw(),
                raw(-125).raw()
            );
            assert!(!p.clip_to_fill_rectangle);
            assert_eq!(
                p.tile_x,
                if matches!(flip, NativeTileFlip::X | NativeTileFlip::Xy) {
                    ImageTile::Mirror
                } else {
                    ImageTile::Repeat
                }
            );
            assert_eq!(
                p.tile_y,
                if matches!(flip, NativeTileFlip::Y | NativeTileFlip::Xy) {
                    ImageTile::Mirror
                } else {
                    ImageTile::Repeat
                }
            );
        }
    }
}
#[test]
fn physical_density_is_required_only_by_tile_and_explicit_dpi_wins() {
    for state in [
        PhysicalPixelSize::Unspecified,
        PhysicalPixelSize::ZeroDensity,
        PhysicalPixelSize::Conflicting,
    ] {
        let mut im = image();
        im.resolution.physical_pixel_size = state;
        let mut f = fill();
        assert!(layout(&f, &im, size(), &|| false).is_ok());
        f.mode = tile(Alignment::TopLeft, NativeTileFlip::None);
        assert!(
            matches!(layout(&f, &im, size(), &||false), Err(ImageLayoutError::PhysicalSize(v)) if v == state)
        );
        f.dpi = value(100);
        assert!(matches!(
            layout(&f, &im, size(), &|| false).unwrap().density,
            ImageLayoutDensity::Drawingml { dpi: 100 }
        ));
    }
    let mut im = image();
    im.resolution.physical_pixel_size = PhysicalPixelSize::Known {
        x: PixelExtent {
            numerator: Emu::new(21),
            denominator: 2,
        },
        y: PixelExtent {
            numerator: Emu::new(35),
            denominator: 4,
        },
    };
    let mut f = fill();
    f.mode = tile(Alignment::Center, NativeTileFlip::Xy);
    let p = layout(&f, &im, size(), &|| false).unwrap();
    assert_eq!(p.pixel_step.x.raw(), 21 * (1i128 << 32) / 4);
    assert_eq!(p.pixel_step.y.raw(), 35 * (1i128 << 32) / 2);
    assert!(matches!(
        p.density,
        ImageLayoutDensity::EncodedMetadata { .. }
    ));
}
#[test]
fn outsets_and_normalized_axes_survive_without_implicit_orientation_or_clamp() {
    let mut im = image();
    im.width = 80;
    im.height = 100;
    im.orientation = 6;
    let mut f = fill();
    f.rotate_with_shape = value(false);
    f.source_rect = rect(["-25%", "0", "-25%", "0"]);
    let p = layout(&f, &im, size(), &|| false).unwrap();
    assert_eq!(p.source_rectangle.left, raw(-20));
    assert_eq!(p.source_rectangle.right, raw(100));
    assert_eq!(p.source_rectangle.bottom, raw(100));
    assert!(!p.rotate_with_shape);
    assert_eq!(p.profile, PROFILE);
}
#[test]
fn exact_decimal_pipeline_bounds_errors_before_narrow_domain_division() {
    let mut f = fill();
    f.source_rect = rect(["49.999999999%", "0", "49.999999999%", "0"]);
    let p = layout(&f, &image(), size(), &|| false).unwrap();
    // Exact remaining width 1/500,000,000 pixel gives 5e11 EMU/pixel.
    assert!(
        (p.pixel_step.x.raw() - raw(500_000_000_000).raw()).abs()
            <= p.uncertainty.pixel_step.x.raw()
    );
    // Q96 enclosure is amplified by division and explicitly retained.
    assert!(p.uncertainty.pixel_step.x.raw() > 0);
    // Independent exact rational bound for source left.
    let exact = num_bigint::BigInt::from(49_999_999_999i64) << 32usize;
    let delta = num_bigint::BigInt::from(p.source_rectangle.left.raw()) * 1_000_000_000i64 - exact;
    let bound =
        num_bigint::BigInt::from(p.uncertainty.source_rectangle[0].raw()) * 1_000_000_000i64;
    assert!(delta >= -&bound && delta <= bound);
    // An even narrower, valid source rectangle collapses in output Q32 and
    // must not silently become a full image or a zero-width render command.
    f.source_rect = rect(["49.9999999999%", "0", "49.9999999999%", "0"]);
    assert!(matches!(
        layout(&f, &image(), size(), &|| false),
        Err(ImageLayoutError::Precision)
    ));
}
#[test]
fn unusable_dimensions_scales_metadata_and_oversized_lexemes_fail_explicitly() {
    let mut f = fill();
    let mut im = image();
    im.width = 0;
    assert!(matches!(
        layout(&f, &im, size(), &|| false),
        Err(ImageLayoutError::Invalid(_))
    ));
    im = image();
    f.source_rect = rect(["75000", "0", "25000", "0"]);
    assert!(matches!(
        layout(&f, &im, size(), &|| false),
        Err(ImageLayoutError::Invalid(_))
    ));
    f = fill();
    f.dpi = value(100);
    f.mode = tile(Alignment::TopLeft, NativeTileFlip::None);
    if let EffectiveImageMode::Tile { tile, .. } = &mut f.mode {
        tile.scale_x = value("0".to_owned().try_into().unwrap());
    }
    assert!(matches!(
        layout(&f, &im, size(), &|| false),
        Err(ImageLayoutError::Invalid(_))
    ));
    f = fill();
    f.source_rect = rect([&format!("0.{}1%", "0".repeat(257)), "0", "0", "0"]);
    assert!(matches!(
        layout(&f, &im, size(), &|| false),
        Err(ImageLayoutError::LexicalLimit)
    ));
    f = fill();
    f.mode = tile(Alignment::TopLeft, NativeTileFlip::None);
    im.resolution.physical_pixel_size = PhysicalPixelSize::Known {
        x: PixelExtent {
            numerator: Emu::new(1),
            denominator: 0,
        },
        y: PixelExtent {
            numerator: Emu::new(1),
            denominator: 1,
        },
    };
    assert!(matches!(
        layout(&f, &im, size(), &|| false),
        Err(ImageLayoutError::Invalid(_))
    ));
}
#[test]
fn each_layout_checkpoint_cancels_without_returning_a_partial_plan() {
    for tiled in [false, true] {
        let mut f = fill();
        if tiled {
            f.dpi = value(100);
            f.mode = tile(Alignment::Center, NativeTileFlip::Xy);
        }
        let calls = Cell::new(0);
        layout(&f, &image(), size(), &|| {
            calls.set(calls.get() + 1);
            false
        })
        .unwrap();
        for at in 1..=calls.get() {
            let calls = Cell::new(0);
            assert!(matches!(
                layout(&f, &image(), size(), &|| {
                    calls.set(calls.get() + 1);
                    calls.get() == at
                }),
                Err(ImageLayoutError::Cancelled)
            ));
        }
    }
}
