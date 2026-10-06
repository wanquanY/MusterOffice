use mo_color::rgb::*;
fn close(a: f64, b: f64, t: f64) {
    assert!((a - b).abs() <= t, "{a} != {b}");
}
#[test]
fn primary_matrices_preserve_d65_white_and_known_2020_red() {
    for p in [
        Primaries::Bt709,
        Primaries::Bt2020,
        Primaries::DisplayP3,
        Primaries::SmpteC,
        Primaries::Bt470Bg,
    ] {
        let xyz = p.to_xyz().apply([1.; 3]);
        for (a, b) in xyz
            .into_iter()
            .zip([0.9504559270516716, 1., 1.0890577507598784])
        {
            close(a, b, 2e-14);
        }
        let back = p
            .to(Primaries::Bt709)
            .apply(Primaries::Bt709.to(p).apply([0.25, 0.5, 0.75]));
        for (a, b) in back.into_iter().zip([0.25, 0.5, 0.75]) {
            close(a, b, 2e-14);
        }
    }
    let red = Primaries::Bt2020.to(Primaries::Bt709).apply([1., 0., 0.]);
    for (a, b) in
        red.into_iter()
            .zip([1.6604910021084345, -0.1245504745215907, -0.0181507633549053])
    {
        close(a, b, 1e-13);
    }
}
#[test]
fn sdr_curves_preserve_superwhites_and_negative_values() {
    for transfer in [
        Transfer::Srgb,
        Transfer::Gamma22,
        Transfer::Gamma28,
        Transfer::Linear,
    ] {
        let t = RgbTransform::new(
            Encoding {
                primaries: Primaries::Bt709,
                transfer,
            },
            Primaries::Bt709,
        )
        .unwrap();
        let v = t.decode([-0.5, 0.5, 1.1]).unwrap();
        close(v.rgb[0], -v.rgb[1], 1e-14);
        assert!(v.rgb[2] > 1.);
        assert!(!v.signal_clipped);
        let white = t.decode([1.; 3]).unwrap();
        for v in white.rgb {
            close(v, 1., 1e-14);
        }
    }
    let t = RgbTransform::new(
        Encoding {
            primaries: Primaries::Bt709,
            transfer: Transfer::Bt1886,
        },
        Primaries::Bt709,
    )
    .unwrap();
    close(
        t.decode([0.5; 3]).unwrap().rgb[0],
        0.18946457081379978,
        1e-14,
    );
    let low = t.decode([-0.1, 0.5, 1.1]).unwrap();
    assert_eq!(low.rgb[0], 0.);
    assert!(low.signal_clipped);
    assert!(low.rgb[2] > 1.);
}
#[test]
fn pq_absolute_luminance_and_hlg_display_ootf_are_explicit() {
    let pq = RgbTransform::new(
        Encoding {
            primaries: Primaries::Bt2020,
            transfer: Transfer::Pq,
        },
        Primaries::Bt2020,
    )
    .unwrap();
    for (code, nits) in [
        (0., 0.),
        (0.508078421517399, 100.),
        (0.5806888810416109, 203.),
        (0.751827096247041, 1000.),
        (1., 10000.),
    ] {
        close(
            pq.decode([code; 3]).unwrap().rgb[0] * REFERENCE_WHITE_NITS,
            nits,
            1e-8,
        );
    }
    assert!(pq.decode([-0.1, 0.5, 1.1]).unwrap().signal_clipped);
    let hlg = RgbTransform::new(
        Encoding {
            primaries: Primaries::Bt2020,
            transfer: Transfer::Hlg {
                peak_nits: 1000.,
                system_gamma: 1.2,
            },
        },
        Primaries::Bt2020,
    )
    .unwrap();
    close(
        hlg.decode([0.5; 3]).unwrap().rgb[0] * REFERENCE_WHITE_NITS,
        50.69702849110048,
        1e-9,
    );
    close(
        hlg.decode([1.; 3]).unwrap().rgb[0] * REFERENCE_WHITE_NITS,
        1000.,
        5e-5, // BT.2100 rounded HLG a constant leaves < 0.00005 nit endpoint error.
    );
    assert!(
        RgbTransform::new(
            Encoding {
                primaries: Primaries::Bt2020,
                transfer: Transfer::Hlg {
                    peak_nits: f64::NAN,
                    system_gamma: 1.2
                }
            },
            Primaries::Bt709
        )
        .is_err()
    );
    assert!(pq.decode([f64::INFINITY, 0., 0.]).is_err());
}
