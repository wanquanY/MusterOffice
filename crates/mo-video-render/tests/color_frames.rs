mod support;
use mo_color::rgb::{Primaries, Transfer};
use mo_media::DecodedFrame;
use mo_video_render::*;
use support::*;
#[test]
fn alpha_is_unassociated_before_transfer_then_linear_premultiplied() {
    let raw = rgba(&[[255, 0, 0, 128], [0, 255, 0, 0]]);
    let normalized = normalize_frame(&raw, srgb(), PixelLimits::default(), &|| false).unwrap();
    let out = normalized.image.to_srgb8(output(), &|| false).unwrap();
    assert_eq!(out.rgba, vec![255, 0, 0, 128, 0, 0, 0, 0]);
    let mut color = srgb();
    color.alpha = Alpha::PremultipliedEncoded;
    let premul = normalize_frame(
        &rgba(&[[128, 0, 0, 128]]),
        color,
        PixelLimits::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(
        premul.image.to_srgb8(output(), &|| false).unwrap().rgba,
        vec![255, 0, 0, 128]
    );
    let mut opaque = srgb();
    opaque.alpha = Alpha::Opaque;
    assert!(normalize_frame(&raw, opaque, PixelLimits::default(), &|| false).is_err());
}
#[test]
fn limited_yuv_black_white_and_ten_bit_p010_preserve_ranges() {
    for (format, bits, shift, semi) in [
        ("yuv420p", 8, 0, false),
        ("yuv420p10le", 10, 0, false),
        ("p010le", 10, 6, true),
    ] {
        let code = |v: u16| {
            if bits == 8 {
                vec![v as u8]
            } else {
                (v << (bits - 8 + shift)).to_le_bytes().to_vec()
            }
        };
        let mut y = Vec::new();
        for v in [16, 235, 16, 235] {
            y.extend(code(v));
        }
        let planes = if semi {
            vec![plane(4, 2, y), plane(4, 1, [code(128), code(128)].concat())]
        } else {
            vec![
                plane(if bits == 8 { 2 } else { 4 }, 2, y),
                plane(if bits == 8 { 1 } else { 2 }, 1, code(128)),
                plane(if bits == 8 { 1 } else { 2 }, 1, code(128)),
            ]
        };
        let f = DecodedFrame {
            info: info(format, 2, 2),
            planes,
        };
        let c = InputColor::from_codec(&f.info, false, None).unwrap();
        let out = normalize_frame(&f, c, PixelLimits::default(), &|| false)
            .unwrap()
            .image
            .to_srgb8(output(), &|| false)
            .unwrap();
        assert_eq!(
            out.rgba,
            vec![
                0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 255, 255, 255, 255
            ],
            "{format}"
        );
    }
}
#[test]
fn odd_plane_extents_and_chroma_siting_are_validated_not_guessed() {
    let f = DecodedFrame {
        info: info("yuv420p", 3, 3),
        planes: vec![
            plane(3, 3, vec![128; 9]),
            plane(2, 2, vec![64, 192, 64, 192]),
            plane(2, 2, vec![128; 4]),
        ],
    };
    let mut c = InputColor::from_codec(&f.info, false, None).unwrap();
    let left = normalize_frame(&f, c, PixelLimits::default(), &|| false).unwrap();
    c.chroma = Some(ChromaLocation::Center);
    let center = normalize_frame(&f, c, PixelLimits::default(), &|| false).unwrap();
    assert_ne!(left.image.pixels()[1], center.image.pixels()[1]);
    c.chroma = None;
    assert!(matches!(
        normalize_frame(&f, c, PixelLimits::default(), &|| false),
        Err(RenderError::Unsupported(_))
    ));
    let mut malformed = f;
    malformed.planes[1].bytes.pop();
    assert!(normalize_frame(&malformed, c, PixelLimits::default(), &|| false).is_err());
}
#[test]
fn hdr_needs_an_explicit_output_curve_and_rgb_cannot_be_yuv_converted_twice() {
    let mut c = srgb();
    c.encoding.primaries = Primaries::Bt2020;
    c.encoding.transfer = Transfer::Pq;
    let n = normalize_frame(
        &rgba(&[[192, 192, 192, 255]]),
        c,
        PixelLimits::default(),
        &|| false,
    )
    .unwrap();
    assert!(n.image.has_hdr());
    assert!(matches!(
        n.image.to_srgb8(output(), &|| false),
        Err(RenderError::Unsupported(_))
    ));
    let mut options = output();
    options.tone_map = Some(ToneMap::ReinhardLuminanceV1 {
        source_peak_nits: 1000.,
    });
    assert_eq!(
        n.image
            .to_srgb8(options, &|| false)
            .unwrap()
            .tone_mapped_pixels,
        1
    );
    c.matrix = Matrix::Bt709;
    assert!(
        normalize_frame(
            &rgba(&[[10, 20, 30, 255]]),
            c,
            PixelLimits::default(),
            &|| false
        )
        .is_err()
    );
    let mut f = rgba(&[[0; 4]]);
    f.info.colors.primaries = 2;
    assert!(InputColor::from_codec(&f.info, true, None).is_err());
}
#[test]
fn normalization_and_output_cancel_without_a_partial_image() {
    let count = std::cell::Cell::new(0);
    let f = rgba(&[[255, 0, 0, 255]; 8]);
    normalize_frame(&f, srgb(), PixelLimits::default(), &|| {
        count.set(count.get() + 1);
        false
    })
    .unwrap();
    for at in 1..=count.get() {
        let seen = std::cell::Cell::new(0);
        assert!(matches!(
            normalize_frame(&f, srgb(), PixelLimits::default(), &|| {
                seen.set(seen.get() + 1);
                seen.get() == at
            }),
            Err(RenderError::Cancelled)
        ));
    }
    let n = normalize_frame(&f, srgb(), PixelLimits::default(), &|| false).unwrap();
    assert!(matches!(
        n.image.to_srgb8(output(), &|| true),
        Err(RenderError::Cancelled)
    ));
}
