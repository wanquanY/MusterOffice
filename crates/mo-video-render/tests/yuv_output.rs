use mo_color::rgb::Primaries;
use mo_video_render::*;
use std::cell::Cell;
fn options() -> I420Options {
    I420Options {
        background_srgb: [0; 3],
        tone_map: None,
        max_bytes: 1 << 20,
        max_temporary_bytes: 1 << 20,
    }
}
fn image(w: u32, h: u32, pixels: Vec<[f32; 4]>, hdr: bool) -> LinearImage {
    LinearImage::from_pixels(w, h, pixels, hdr, PixelLimits::default(), &|| false).unwrap()
}
fn color(rgb: [f64; 3], alpha: f64) -> [f32; 4] {
    let linear = Primaries::Bt709.to(Primaries::Bt2020).apply(rgb);
    [
        (linear[0] * alpha) as f32,
        (linear[1] * alpha) as f32,
        (linear[2] * alpha) as f32,
        alpha as f32,
    ]
}
#[test]
fn video_transfer_range_primary_colors_and_explicit_linear_matte() {
    for (rgb, expected) in [
        ([0.; 3], [16, 128, 128]),
        ([1.; 3], [235, 128, 128]),
        ([1., 0., 0.], [63, 102, 240]),
        ([0., 1., 0.], [173, 42, 26]),
        ([0., 0., 1.], [32, 240, 118]),
        ([0.5; 3], [180, 128, 128]),
    ] {
        let out = image(8, 6, vec![color(rgb, 1.); 48], false)
            .to_i420(options(), &|| false)
            .unwrap();
        for (plane, value) in out.planes().into_iter().zip(expected) {
            assert!(plane.iter().all(|v| v.abs_diff(value) <= 1));
        }
        assert_eq!(out.byte_length(), 72);
        assert_eq!(out.scratch_bytes, 8 * 80);
    }
    let transparent = image(2, 2, vec![[0.; 4]; 4], false)
        .to_i420(
            I420Options {
                background_srgb: [255; 3],
                ..options()
            },
            &|| false,
        )
        .unwrap();
    assert_eq!(transparent.planes()[0], [235; 4]);
    let half = image(2, 2, vec![color([0.; 3], 0.5); 4], false)
        .to_i420(
            I420Options {
                background_srgb: [255; 3],
                ..options()
            },
            &|| false,
        )
        .unwrap();
    assert_eq!(half.planes()[0], [180; 4]); // linear .5 is gamma-coded ~.749, not .5 or sRGB .735.
}
#[test]
fn left_siting_antialias_and_bounded_row_ring_preserve_large_constant_image() {
    let w = 30;
    let h = 28;
    let pixels = (0..w * h)
        .map(|i| {
            color(
                if i % 2 == 0 {
                    [1., 0., 0.]
                } else {
                    [0., 0., 1.]
                },
                1.,
            )
        })
        .collect();
    let out = image(w, h, pixels, false)
        .to_i420(options(), &|| false)
        .unwrap();
    // Interior high-frequency chroma is removed before decimation; it cannot
    // alternate or become all red as a nearest-pixel subsampler would.
    for row in out.planes()[1].chunks(15) {
        for &v in &row[3..12] {
            assert!((170..=172).contains(&v));
        }
    }
    assert!(
        out.planes()[1][0] < out.planes()[1][3],
        "left edge retains red at the left-cosited phase"
    );
    let constant = image(
        w,
        h,
        vec![color([0.2, 0.4, 0.6], 1.); (w * h) as usize],
        false,
    )
    .to_i420(options(), &|| false)
    .unwrap();
    for p in constant.planes() {
        assert!(p.iter().all(|v| *v == p[0]));
    }
    assert_eq!(constant.colors.gamut_clipped_pixels, 0);
}
#[test]
fn hdr_even_dimensions_budgets_and_each_observed_cancel_boundary_fail_without_pixels() {
    assert!(
        image(3, 2, vec![color([0.; 3], 1.); 6], false)
            .to_i420(options(), &|| false)
            .is_err()
    );
    let hdr = image(8, 10, vec![color([4.; 3], 1.); 80], true);
    assert!(matches!(
        hdr.to_i420(options(), &|| false),
        Err(RenderError::Unsupported(_))
    ));
    let o = I420Options {
        tone_map: Some(ToneMap::ReinhardLuminanceV1 {
            source_peak_nits: 1000.,
        }),
        ..options()
    };
    assert_eq!(
        hdr.to_i420(o, &|| false).unwrap().colors.tone_mapped_pixels,
        80
    );
    for options in [
        I420Options {
            max_bytes: 119,
            ..o
        },
        I420Options {
            max_temporary_bytes: 639,
            ..o
        },
    ] {
        assert!(matches!(
            hdr.to_i420(options, &|| false),
            Err(RenderError::Budget)
        ));
    }
    let visits = Cell::new(0);
    hdr.to_i420(o, &|| {
        visits.set(visits.get() + 1);
        false
    })
    .unwrap();
    for fail in 1..=visits.get() {
        let count = Cell::new(0);
        assert!(matches!(
            hdr.to_i420(o, &|| {
                count.set(count.get() + 1);
                count.get() >= fail
            }),
            Err(RenderError::Cancelled)
        ));
    }
}
