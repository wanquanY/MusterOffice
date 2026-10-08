#[path = "stream/support.rs"]
mod fixture;
use fixture::{frame, origin};
use mo_media::{DecodedFrame, TimeBase};
use mo_time::{ExactRatio as R, Rounding};
use mo_video_audio::ByteOrder;
use mo_video_pcm::*;

fn clock(rate: u32) -> MicrosecondPcmClock {
    MicrosecondPcmClock::new(
        R::integer(-7)
            .div(&R::integer(i64::from(rate)), 256)
            .unwrap(),
        rate,
        256,
    )
    .unwrap()
}
fn exact_frame(first: i64, count: u32, rate: u32) -> DecodedFrame {
    let mut value = frame(first, count, 2, first % 2 == 0);
    value.info.sample_rate = Some(rate);
    value.info.time_base = TimeBase {
        numerator: 1,
        denominator: rate,
    };
    value.info.pts_ticks = Some(first - 7);
    value
}
fn platform_frame(first: i64, count: u32, rate: u32) -> DecodedFrame {
    let mut value = exact_frame(first, count, rate);
    value.info.time_base.denominator = 1_000_000;
    value.info.pts_ticks = Some(
        R::integer(first)
            .mul(&R::integer(1_000_000), 256)
            .unwrap()
            .div(&R::integer(i64::from(rate)), 256)
            .unwrap()
            .round_integer(Rounding::NearestTiesEven)
            .unwrap(),
    );
    value
}

#[test]
fn platform_clock_and_native_tiles_agree_without_accumulation_or_first_output_realignment() {
    for rate in [8000, 11025, 22050, 44100, 48000, 96000, 192000, 384000] {
        for leading_discard in [0, 211, 1024, 2049] {
            let limits = PcmStreamLimits::default();
            let mut native = PcmStreamBuilder::new(origin(), 1, limits).unwrap();
            let mut web = MicrosecondPcmStream::new(origin(), 1, clock(rate), limits).unwrap();
            let mut a = vec![];
            let mut b = vec![];
            let mut first = leading_discard;
            for count in [211, 1024, 960, 3, 4096, 8192, 1, 512] {
                a.extend(
                    native
                        .push(&exact_frame(first, count, rate), ByteOrder::Little, &|| {
                            false
                        })
                        .unwrap(),
                );
                b.extend(
                    web.push(
                        &platform_frame(first, count, rate),
                        ByteOrder::Little,
                        &|| false,
                    )
                    .unwrap(),
                );
                first += i64::from(count);
                assert!(web.retained_bytes() <= 4096 * 2 * 4);
            }
            let mut native = native.finish_at_eof(&|| false).unwrap();
            let mut web = web.finish_at_eof(&|| false).unwrap();
            a.extend(native.tail.take());
            b.extend(web.tail.take());
            assert_eq!(
                native.source().identity(),
                web.source().identity(),
                "rate={rate}, discard={leading_discard}"
            );
            assert_eq!(native.decoded().asset(), web.decoded().asset());
            assert_eq!(
                native.decoded().asset().sample_zero,
                R::integer(leading_discard - 7)
                    .div(&R::integer(i64::from(rate)), 256)
                    .unwrap()
            );
            assert_eq!(a.len(), b.len());
            for (a, b) in a.iter().zip(&b) {
                assert_eq!(a.reference, b.reference);
                assert_eq!(a.bytes, b.bytes);
            }
        }
    }
}

#[test]
fn transport_roundtrip_is_absolute_and_accepts_the_two_rounding_bound() {
    for rate in [44100, 48000, 192000, 384000] {
        let c = clock(rate);
        let origin = R::integer(-7)
            .div(&R::integer(i64::from(rate)), 256)
            .unwrap();
        for sample in [0, 1, 960, 1024, 123456789, 20_000_000_000_001] {
            let exact = R::integer(sample - 7)
                .div(&R::integer(i64::from(rate)), 256)
                .unwrap();
            let timestamp = c.packet_timestamp(&exact).unwrap();
            assert_eq!(c.decoded_timestamp(timestamp).unwrap(), exact);
        }
        // One independently rounded trim can add to packet rounding. The
        // bridge accepts it only if the combined error is <= one microsecond.
        let packet = c
            .packet_timestamp(
                &origin
                    .add(
                        &R::integer(1024)
                            .div(&R::integer(i64::from(rate)), 256)
                            .unwrap(),
                        256,
                    )
                    .unwrap(),
            )
            .unwrap();
        let trim = R::integer(211_000_000)
            .div(&R::integer(i64::from(rate)), 256)
            .unwrap()
            .round_integer(Rounding::NearestTiesEven)
            .unwrap();
        assert_eq!(
            c.decoded_timestamp(packet + trim).unwrap(),
            origin
                .add(
                    &R::integer(1235)
                        .div(&R::integer(i64::from(rate)), 256)
                        .unwrap(),
                    256
                )
                .unwrap()
        );
    }
    let c = clock(48000);
    assert!(c.decoded_timestamp(-1).is_err());
    assert!(c.decoded_timestamp(2).is_err());
    assert!(c.packet_timestamp(&R::integer(0)).is_ok());
    assert!(c.packet_timestamp(&R::integer(-1)).is_err());
    assert!(
        c.packet_timestamp(&R::integer(1).div(&R::integer(7), 256).unwrap())
            .is_err()
    );
    assert!(MicrosecondPcmClock::new(R::integer(0), 500000, 256).is_err());
}

#[test]
fn clock_faults_holes_overlaps_format_changes_and_cancel_poison_completion() {
    for kind in 0..9 {
        let mut web =
            MicrosecondPcmStream::new(origin(), 1, clock(48000), PcmStreamLimits::default())
                .unwrap();
        web.push(&platform_frame(0, 1024, 48000), ByteOrder::Little, &|| {
            false
        })
        .unwrap();
        let mut frame = platform_frame(1024, 1024, 48000);
        match kind {
            0 => frame.info.pts_ticks = None,
            1 => frame.info.pts_ticks = Some(21336),
            2 => frame = platform_frame(1025, 1024, 48000),
            3 => frame = platform_frame(1023, 1024, 48000),
            4 => frame.info.time_base.denominator = 48000,
            5 => frame.info.sample_rate = Some(44100),
            6 => frame.info.generation = 2,
            7 => frame.planes[0].bytes[0..4].copy_from_slice(&f32::NAN.to_le_bytes()),
            _ => {}
        }
        assert!(
            web.push(&frame, ByteOrder::Little, &|| kind == 8).is_err(),
            "kind {kind}"
        );
        assert!(
            web.push(&platform_frame(1024, 1, 48000), ByteOrder::Little, &|| {
                false
            })
            .is_err()
        );
        assert!(web.finish_at_eof(&|| false).is_err());
    }
}
