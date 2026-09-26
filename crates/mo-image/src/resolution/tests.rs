use super::*;
use std::cell::Cell;

fn chunk(name: &[u8; 4], body: &[u8]) -> Vec<u8> {
    // Metadata unit tests exercise the bounded metadata pass only. Whole-file
    // integrity and pixel decoding are exercised by the real runtime corpus.
    [
        u32::try_from(body.len()).unwrap().to_be_bytes().as_slice(),
        name,
        body,
        &[0; 4],
    ]
    .concat()
}
fn png(parts: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut b = b"\x89PNG\r\n\x1a\n".to_vec();
    for (tag, value) in parts {
        b.extend(chunk(tag, value));
    }
    b.extend(chunk(b"IEND", &[]));
    b
}
fn physical(x: u32, y: u32, unit: u8) -> Vec<u8> {
    [
        x.to_be_bytes().as_slice(),
        y.to_be_bytes().as_slice(),
        &[unit],
    ]
    .concat()
}
fn exif(little: bool, values: &[(u16, u32, u32)]) -> Vec<u8> {
    let u16 = |v: u16| {
        if little {
            v.to_le_bytes()
        } else {
            v.to_be_bytes()
        }
    };
    let u32 = |v: u32| {
        if little {
            v.to_le_bytes()
        } else {
            v.to_be_bytes()
        }
    };
    let mut b = if little {
        b"II".to_vec()
    } else {
        b"MM".to_vec()
    };
    b.extend(u16(42));
    b.extend(u32(8));
    b.extend(u16(values.len() as _));
    let mut data = Vec::new();
    let start = 14 + values.len() * 12;
    for &(tag, n, d) in values {
        b.extend(u16(tag));
        b.extend(u16(if tag == 0x128 { 3 } else { 5 }));
        b.extend(u32(1));
        if tag == 0x128 {
            b.extend(u16(n as u16));
            b.extend([0, 0]);
        } else {
            b.extend(u32((start + data.len()) as u32));
            data.extend(u32(n));
            data.extend(u32(d));
        }
    }
    b.extend(u32(0));
    b.extend(data);
    b
}
fn read_png(b: &[u8], orientation: u32) -> ImageResolution {
    read(b, ImageFormat::Png, orientation, &|| false).unwrap()
}
fn px(n: i64, d: u32) -> PixelExtent {
    PixelExtent {
        numerator: Emu::new(n),
        denominator: d,
    }
}
#[test]
fn physical_units_and_orientation_are_exact_without_rewriting_declarations() {
    for little in [false, true] {
        let e = exif(little, &[(0x11a, 508, 2), (0x11b, 100, 1), (0x128, 2, 1)]);
        let b = png(&[(b"eXIf", e)]);
        for o in 1..=8 {
            let r = read_png(&b, o);
            assert_eq!(
                r.declarations[0].x,
                Some(Density {
                    numerator: 508,
                    denominator: 2
                })
            );
            assert_eq!(r.declarations[0].source_offset, 8);
            let (x, y) = if o >= 5 {
                (px(9144, 1), px(3600, 1))
            } else {
                (px(3600, 1), px(9144, 1))
            };
            assert_eq!(r.physical_pixel_size, PhysicalPixelSize::Known { x, y });
        }
    }
    let r = read_png(&png(&[(b"pHYs", physical(3779, 10000, 1))]), 1);
    assert_eq!(
        r.physical_pixel_size,
        PhysicalPixelSize::Known {
            x: px(36_000_000, 3779),
            y: px(3600, 1)
        }
    );
}
#[test]
fn no_host_dpi_and_exif_specification_defaults_are_distinguished() {
    assert_eq!(
        read_png(&png(&[]), 1).physical_pixel_size,
        PhysicalPixelSize::Unspecified
    );
    assert_eq!(
        read_png(&png(&[(b"pHYs", physical(2, 1, 0))]), 1).physical_pixel_size,
        PhysicalPixelSize::Unspecified
    );
    let r = read_png(&png(&[(b"eXIf", exif(true, &[]))]), 1);
    assert_eq!(
        (
            r.declarations[0].x,
            r.declarations[0].y,
            r.declarations[0].unit
        ),
        (None, None, None)
    );
    assert_eq!(
        r.physical_pixel_size,
        PhysicalPixelSize::Known {
            x: px(12700, 1),
            y: px(12700, 1)
        }
    );
    let r = read_png(&png(&[(b"eXIf", exif(true, &[(0x11a, 144, 1)]))]), 1);
    assert_eq!(
        r.physical_pixel_size,
        PhysicalPixelSize::Known {
            x: px(6350, 1),
            y: px(12700, 1)
        }
    );
}
#[test]
fn consensus_compares_both_absolute_units_and_aspect_ratios() {
    let e = exif(false, &[(0x11a, 254, 1), (0x11b, 254, 1), (0x128, 2, 1)]);
    for (x, y, unit, status) in [
        (10000, 10000, 1, "known"),
        (1, 1, 0, "known"),
        (2, 1, 0, "conflict"),
        (9999, 9999, 1, "conflict"),
    ] {
        let r = read_png(
            &png(&[(b"eXIf", e.clone()), (b"pHYs", physical(x, y, unit))]),
            1,
        );
        assert_eq!(r.declarations.len(), 2);
        assert_eq!(
            r.physical_pixel_size,
            if status == "known" {
                PhysicalPixelSize::Known {
                    x: px(3600, 1),
                    y: px(3600, 1),
                }
            } else {
                PhysicalPixelSize::Conflicting
            }
        );
    }
    let r = read_png(&png(&[(b"pHYs", physical(0, 1, 1))]), 1);
    assert_eq!(r.physical_pixel_size, PhysicalPixelSize::ZeroDensity);
}
fn segment(marker: u8, body: &[u8]) -> Vec<u8> {
    [
        &[0xff, marker],
        ((body.len() + 2) as u16).to_be_bytes().as_slice(),
        body,
    ]
    .concat()
}
fn jfif(unit: u8, x: u16, y: u16) -> Vec<u8> {
    [
        b"JFIF\0\x01\x02".as_slice(),
        &[unit],
        x.to_be_bytes().as_slice(),
        y.to_be_bytes().as_slice(),
        &[0, 0],
    ]
    .concat()
}
#[test]
fn jpeg_metadata_after_entropy_and_stuffed_restart_bytes_is_not_missed() {
    let e = exif(true, &[(0x11a, 254, 1), (0x11b, 254, 1), (0x128, 2, 1)]);
    let b = [
        b"\xff\xd8".to_vec(),
        segment(0xe0, &jfif(2, 100, 100)),
        segment(0xda, &[]),
        vec![7, 0xff, 0, 8, 0xff, 0xd0, 9],
        segment(0xe1, &[b"Exif\0\0", e.as_slice()].concat()),
        vec![0xff, 0xd9],
    ]
    .concat();
    let r = read(&b, ImageFormat::Jpeg, 1, &|| false).unwrap();
    assert_eq!(
        r.declarations
            .iter()
            .map(|d| d.source_offset)
            .collect::<Vec<_>>(),
        vec![2, 31]
    );
    assert_eq!(
        r.physical_pixel_size,
        PhysicalPixelSize::Known {
            x: px(3600, 1),
            y: px(3600, 1)
        }
    );
}
#[test]
fn malformed_resolution_structures_are_errors_not_discarded_metadata() {
    let good = exif(true, &[(0x11a, 300, 1), (0x11b, 300, 1), (0x128, 2, 1)]);
    let mut bad = Vec::new();
    for (position, value) in [(12, 3), (14, 2), (18, 255), (6, 255)] {
        let mut e = good.clone();
        e[position] = value;
        bad.push(e);
    }
    bad.push(exif(true, &[(0x11a, 300, 0)]));
    bad.push(exif(true, &[(0x128, 1, 1)]));
    bad.push(exif(true, &[(0x11a, 300, 1), (0x11a, 300, 1)]));
    bad.push(exif(true, &[(0x128, 2, 1), (0x128, 2, 1)]));
    for e in bad {
        assert!(matches!(
            read(&png(&[(b"eXIf", e)]), ImageFormat::Png, 1, &|| false),
            Err(ImageError::Invalid(_))
        ));
    }
    for parts in [
        vec![(b"pHYs", physical(1, 1, 2))],
        vec![(b"pHYs", vec![1; 8])],
        vec![(b"pHYs", physical(1, 1, 1)), (b"pHYs", physical(1, 1, 1))],
        vec![(b"IDAT", vec![]), (b"pHYs", physical(1, 1, 1))],
    ] {
        assert!(read(&png(&parts), ImageFormat::Png, 1, &|| false).is_err());
    }
    for body in [
        jfif(3, 100, 100),
        b"JFIF\0".to_vec(),
        [jfif(1, 100, 100), vec![7]].concat(),
    ] {
        let b = [vec![0xff, 0xd8], segment(0xe0, &body), vec![0xff, 0xd9]].concat();
        assert!(read(&b, ImageFormat::Jpeg, 1, &|| false).is_err());
    }
}
#[test]
fn every_truncated_span_and_cancellation_checkpoint_is_safe() {
    let b = png(&[(
        b"eXIf",
        exif(true, &[(0x11a, 300, 1), (0x11b, 300, 1), (0x128, 2, 1)]),
    )]);
    for n in 0..b.len() {
        assert!(read(&b[..n], ImageFormat::Png, 1, &|| false).is_err());
    }
    let b = [
        vec![0xff, 0xd8],
        segment(0xda, &[]),
        vec![7; 100_000],
        vec![0xff; 40_000],
        vec![0xd9],
    ]
    .concat();
    let calls = Cell::new(0);
    read(&b, ImageFormat::Jpeg, 1, &|| {
        calls.set(calls.get() + 1);
        false
    })
    .unwrap();
    assert!(calls.get() > 9);
    for stop in 1..=calls.get() {
        let n = Cell::new(0);
        assert!(matches!(
            read(&b, ImageFormat::Jpeg, 1, &|| {
                n.set(n.get() + 1);
                n.get() == stop
            }),
            Err(ImageError::Cancelled)
        ));
    }
}
#[test]
fn maximum_rationals_do_not_overflow_and_marker_work_is_bounded() {
    let e = exif(
        true,
        &[
            (0x11a, 1, u32::MAX),
            (0x11b, u32::MAX, u32::MAX),
            (0x128, 3, 1),
        ],
    );
    let r = read_png(&png(&[(b"eXIf", e)]), 1);
    assert_eq!(
        r.physical_pixel_size,
        PhysicalPixelSize::Known {
            x: px(360_000 * i64::from(u32::MAX), 1),
            y: px(360_000, 1)
        }
    );
    let parts = vec![(b"tEXt", vec![]); 4096];
    assert!(matches!(
        read(&png(&parts), ImageFormat::Png, 1, &|| false),
        Err(ImageError::Limit(_))
    ));
    let b = [
        vec![0xff, 0xd8],
        segment(0xfe, &[]).repeat(4096),
        vec![0xff, 0xd9],
    ]
    .concat();
    assert!(matches!(
        read(&b, ImageFormat::Jpeg, 1, &|| false),
        Err(ImageError::Limit(_))
    ));
}
