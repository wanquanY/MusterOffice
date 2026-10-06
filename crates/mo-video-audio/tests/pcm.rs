use mo_media::*;
use mo_time::ExactRatio as R;
use mo_video_audio::*;
fn frame(format: &str, planar: bool, bytes: Vec<Vec<u8>>) -> DecodedFrame {
    DecodedFrame {
        info: FrameInfo {
            generation: 1,
            kind: MediaKind::Audio,
            time_base: TimeBase {
                numerator: 1,
                denominator: 48000,
            },
            pts_ticks: Some(0),
            duration_ticks: Some(2),
            width: None,
            height: None,
            sample_rate: Some(48000),
            channels: Some(2),
            samples: Some(2),
            format: format!("{format}{}", if planar { "p" } else { "" }),
            channel_layout: "stereo".into(),
            colors: CodecColorTags {
                primaries: 2,
                transfer: 2,
                matrix: 2,
                range: 0,
                chroma_location: 0,
            },
            sample_aspect_ratio: None,
            alpha_mode: 0,
            key_frame: true,
            interlaced: false,
            top_field_first: false,
        },
        planes: bytes
            .into_iter()
            .map(|bytes| FramePlane {
                row_bytes: bytes.len(),
                rows: 1,
                bytes,
            })
            .collect(),
    }
}
#[test]
fn all_native_pcm_formats_preserve_layout_and_explicit_byte_order() {
    let formats = [
        ("u8", vec![vec![0], vec![192], vec![128], vec![64]]),
        (
            "s16",
            [i16::MIN, 16384, 0, -16384]
                .iter()
                .map(|v| v.to_le_bytes().to_vec())
                .collect(),
        ),
        (
            "s32",
            [i32::MIN, 1 << 30, 0, -(1 << 30)]
                .iter()
                .map(|v| v.to_le_bytes().to_vec())
                .collect(),
        ),
        (
            "s64",
            [i64::MIN, 1 << 62, 0, -(1 << 62)]
                .iter()
                .map(|v| v.to_le_bytes().to_vec())
                .collect(),
        ),
        (
            "flt",
            [-1f32, 0.5, 0., -0.5]
                .iter()
                .map(|v| v.to_le_bytes().to_vec())
                .collect(),
        ),
        (
            "dbl",
            [-1f64, 0.5, 0., -0.5]
                .iter()
                .map(|v| v.to_le_bytes().to_vec())
                .collect(),
        ),
    ];
    for (format, samples) in formats {
        for planar in [false, true] {
            for big in [false, true] {
                let mut values = samples.clone();
                if big {
                    for bytes in &mut values {
                        bytes.reverse();
                    }
                }
                let planes = if planar {
                    vec![
                        values[0].iter().chain(&values[2]).copied().collect(),
                        values[1].iter().chain(&values[3]).copied().collect(),
                    ]
                } else {
                    vec![values.concat()]
                };
                let frame = frame(format, planar, planes);
                let pcm = normalize_pcm(
                    &frame,
                    if big {
                        ByteOrder::Big
                    } else {
                        ByteOrder::Little
                    },
                    ChannelLayout::stereo(),
                    -2,
                    PcmLimits::default(),
                    &|| false,
                )
                .unwrap();
                assert_eq!(
                    pcm.samples(),
                    [-1., 0.5, 0., -0.5],
                    "{format} planar={planar} big={big}"
                );
                assert_eq!(pcm.end_frame(), 0);
            }
        }
    }
}
#[test]
fn explicit_matrix_does_not_normalize_or_silently_discard_channels() {
    let source = PcmBlock::new(
        48000,
        ChannelLayout::stereo(),
        0,
        vec![1.5, 0.5, -1.5, -0.5],
        PcmLimits::default(),
        &|| false,
    )
    .unwrap();
    let mixed = source
        .map_channels(
            &ChannelMatrix::stereo_to_mono_average(),
            PcmLimits::default(),
            &|| false,
        )
        .unwrap();
    assert_eq!(mixed.samples(), [1., -1.]);
    assert!(
        source
            .map_channels(
                &ChannelMatrix::mono_to_stereo(),
                PcmLimits::default(),
                &|| false
            )
            .is_err()
    );
    assert!(ChannelLayout::from_codec_name("2 channels").is_err());
    assert!(ChannelLayout::new(vec![Channel::FrontLeft, Channel::FrontLeft]).is_err());
    assert!(
        ChannelMatrix::new(ChannelLayout::mono(), ChannelLayout::mono(), vec![f64::NAN]).is_err()
    );
}
#[test]
fn nonfinite_malformed_truncated_and_overbudget_frames_fail() {
    let mut decoded = frame(
        "flt",
        false,
        vec![
            [1f32, 2., 3., 4.]
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect(),
        ],
    );
    let normalize = |f: &DecodedFrame| {
        normalize_pcm(
            f,
            ByteOrder::Little,
            ChannelLayout::stereo(),
            0,
            PcmLimits::default(),
            &|| false,
        )
    };
    assert_eq!(normalize(&decoded).unwrap().samples(), [1., 2., 3., 4.]); // float overshoots retained
    decoded.planes[0].bytes[0..4].copy_from_slice(&f32::NAN.to_le_bytes());
    assert!(normalize(&decoded).is_err());
    decoded.planes[0].bytes[0..4].copy_from_slice(&1f32.to_le_bytes());
    decoded.planes[0].bytes.pop();
    assert!(normalize(&decoded).is_err());
    let pcm = vec![0.; 10];
    assert!(
        PcmBlock::new(
            48000,
            ChannelLayout::mono(),
            0,
            pcm,
            PcmLimits {
                max_frames: 9,
                max_bytes: 40
            },
            &|| false
        )
        .is_err()
    );
    assert!(
        PcmBlock::new(
            48000,
            ChannelLayout::stereo(),
            0,
            vec![0.; 3],
            PcmLimits::default(),
            &|| false
        )
        .is_err()
    );
}
#[test]
fn failed_phase_advance_does_not_change_the_clock() {
    let start = R::new((num_bigint::BigInt::from(i64::MAX) * 2) + 1, 2.into(), 256).unwrap();
    let step = R::new(1.into(), 2.into(), 256).unwrap();
    let mut clock = PhaseClock::new(&start, &step, 256).unwrap();
    let before = clock.position();
    assert!(clock.advance().is_err());
    assert_eq!(clock.position(), before);
    let start = R::new((num_bigint::BigInt::from(i64::MIN) * 2) + 1, 2.into(), 256).unwrap();
    let step = R::new((-1).into(), 2.into(), 256).unwrap();
    let mut clock = PhaseClock::new(&start, &step, 256).unwrap();
    clock.advance().unwrap();
    assert_eq!(clock.position(), (i64::MIN, 0));
}
