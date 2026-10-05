use mo_media_ffmpeg::{Container, DecodeLimits, ErrorKind, MediaKind, ReadAt, TimeBase};
use std::{cell::Cell, io, rc::Rc};

fn limits() -> DecodeLimits {
    DecodeLimits {
        max_input_bytes: 1 << 24,
        max_read_bytes: 1 << 28,
        max_frame_bytes: 1 << 24,
        max_streams: 16,
        max_pixels: 1 << 24,
        max_audio_samples: 262144,
        max_channels: 8,
        max_decode_steps: 10000,
    }
}
fn wav() -> (Vec<u8>, Vec<u8>) {
    let pcm: Vec<_> = (0..24000i32)
        .flat_map(|i| ((i * 1637 % 64000 - 32000) as i16).to_le_bytes())
        .collect();
    let mut data = Vec::new();
    data.extend(b"RIFF");
    data.extend((36 + pcm.len() as u32).to_le_bytes());
    data.extend(b"WAVEfmt ");
    data.extend(16u32.to_le_bytes());
    data.extend(1u16.to_le_bytes());
    data.extend(1u16.to_le_bytes());
    data.extend(48000u32.to_le_bytes());
    data.extend(96000u32.to_le_bytes());
    data.extend(2u16.to_le_bytes());
    data.extend(16u16.to_le_bytes());
    data.extend(b"data");
    data.extend((pcm.len() as u32).to_le_bytes());
    data.extend(&pcm);
    (data, pcm)
}
struct Source<'a> {
    bytes: &'a [u8],
    max_read: usize,
    drops: Rc<Cell<usize>>,
}
impl ReadAt for Source<'_> {
    fn read_at(&mut self, offset: u64, out: &mut [u8]) -> io::Result<usize> {
        // The safe callback is allowed to inspect destination bytes. The adapter
        // must initialize native AVIO memory before handing it to Rust.
        assert!(out.iter().all(|b| *b == 0));
        let offset = usize::try_from(offset).unwrap();
        let n = out
            .len()
            .min(self.max_read)
            .min(self.bytes.len().saturating_sub(offset));
        out[..n].copy_from_slice(&self.bytes[offset..offset + n]);
        Ok(n)
    }
}
impl Drop for Source<'_> {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}
fn source(bytes: &[u8], max_read: usize) -> Source<'_> {
    Source {
        bytes,
        max_read,
        drops: Rc::default(),
    }
}

#[test]
fn real_pcm_roundtrip_short_reads_and_owned_frames_survive_seek_and_close() {
    let (bytes, expected) = wav();
    for max_read in [13, usize::MAX] {
        let input = source(&bytes, max_read);
        let drops = input.drops.clone();
        let container = Container::open(input, bytes.len() as u64, limits(), || false).unwrap();
        assert_eq!(container.streams().len(), 1);
        let stream = &container.streams()[0];
        assert_eq!(
            (stream.kind, stream.codec.as_str()),
            (MediaKind::Audio, "pcm_s16le")
        );
        assert_eq!(
            stream.time_base,
            Some(TimeBase {
                numerator: 1,
                denominator: 48000
            })
        );
        let mut decoder = container.select(0).unwrap();
        let first = decoder.next_frame().unwrap().unwrap();
        let saved = first.planes[0].bytes.clone();
        let mut actual = saved.clone();
        let mut samples = first.info.samples.unwrap() as i64;
        while let Some(frame) = decoder.next_frame().unwrap() {
            assert_eq!(frame.info.pts_ticks, Some(samples));
            assert_eq!(frame.info.generation, 1);
            assert_eq!(frame.info.format, "s16");
            assert_eq!(frame.planes.len(), 1);
            samples += frame.info.samples.unwrap() as i64;
            actual.extend(&frame.planes[0].bytes);
        }
        assert_eq!(actual, expected);
        assert_eq!(samples, 24000);
        assert!(decoder.next_frame().unwrap().is_none());
        decoder.seek(0).unwrap();
        let again = decoder.next_frame().unwrap().unwrap();
        assert_eq!(again.info.generation, 2);
        assert_eq!(again.info.pts_ticks, first.info.pts_ticks);
        assert_eq!(again.planes, first.planes);
        assert_eq!(drops.get(), 0);
        drop(decoder);
        assert_eq!(drops.get(), 1);
        assert_eq!(first.planes[0].bytes, saved);
    }
}
#[test]
fn failure_paths_release_owned_source_once() {
    let (bytes, _) = wav();
    for phase in 0..4 {
        let input = source(&bytes, usize::MAX);
        let drops = input.drops.clone();
        let mut budget = limits();
        if phase == 0 {
            budget.max_input_bytes = 1;
        }
        let result = Container::open(input, bytes.len() as u64, budget, move || phase == 1);
        if phase < 2 {
            assert!(result.is_err());
        } else {
            let container = result.unwrap();
            if phase == 2 {
                assert!(container.select(100).is_err());
            } else {
                drop(container);
            }
        }
        assert_eq!(drops.get(), 1);
    }
}
#[test]
fn invalid_callback_counts_io_and_panics_do_not_cross_c() {
    for kind in 0..4 {
        let error = Container::open(
            move |_: u64, out: &mut [u8]| -> io::Result<usize> {
                match kind {
                    0 => Ok(out.len() + 1),
                    1 => Ok(0),
                    2 => Err(io::Error::other("host source failed")),
                    _ => panic!("source panic must be caught"),
                }
            },
            50000,
            limits(),
            || false,
        )
        .err()
        .unwrap();
        assert_eq!(
            error.kind,
            match kind {
                0 | 1 => ErrorKind::CallbackContract,
                2 => ErrorKind::Io,
                _ => ErrorKind::CallbackPanic,
            }
        );
    }
}
#[test]
fn panic_payload_destructor_cannot_unwind_through_callback() {
    struct BadPayload;
    impl Drop for BadPayload {
        fn drop(&mut self) {
            panic!("must never drop panic payload in C");
        }
    }
    let error = Container::open(
        |_: u64, _: &mut [u8]| -> io::Result<usize> { std::panic::panic_any(BadPayload) },
        50000,
        limits(),
        || false,
    )
    .err()
    .unwrap();
    assert_eq!(error.kind, ErrorKind::CallbackPanic);
}
#[test]
fn cancellation_panic_inside_native_open_is_contained() {
    let (bytes, _) = wav();
    let checks = Cell::new(0);
    let error = Container::open(source(&bytes, 13), bytes.len() as u64, limits(), || {
        checks.set(checks.get() + 1);
        // First check is the Rust boundary, second is C's open admission;
        // the third runs within the native probing/callback path.
        if checks.get() == 3 {
            panic!("native cancellation callback failed");
        }
        false
    })
    .err()
    .unwrap();
    assert_eq!(error.kind, ErrorKind::CallbackPanic);
    assert_eq!(
        checks.get(),
        3,
        "a poisoned callback must never be called again"
    );
}
#[test]
fn cancellation_panic_and_late_read_failure_close_decoder() {
    let (bytes, _) = wav();
    let trigger = Cell::new(false);
    let container = Container::open(source(&bytes, 13), bytes.len() as u64, limits(), || {
        if trigger.get() {
            panic!("cancel source failed");
        }
        false
    })
    .unwrap();
    let mut decoder = container.select(0).unwrap();
    trigger.set(true);
    assert_eq!(
        decoder.next_frame().err().unwrap().kind,
        ErrorKind::CallbackPanic
    );
    assert!(decoder.is_closed());
    assert_eq!(decoder.seek(0).err().unwrap().kind, ErrorKind::State);

    let fault = Cell::new(false);
    let mut input = source(&bytes, 13);
    let container = Container::open(
        |offset, out: &mut [u8]| {
            if fault.get() {
                panic!("read source failed after opening");
            }
            input.read_at(offset, out)
        },
        bytes.len() as u64,
        limits(),
        || false,
    )
    .unwrap();
    let mut decoder = container.select(0).unwrap();
    fault.set(true);
    let error = loop {
        match decoder.next_frame() {
            Ok(Some(_)) => (),
            Ok(None) => panic!("expected uncached read"),
            Err(e) => break e,
        }
    };
    assert_eq!(error.kind, ErrorKind::CallbackPanic);
    assert!(decoder.is_closed());
}
#[test]
fn cancellation_at_every_decode_boundary_never_returns_a_partial_frame() {
    let (bytes, _) = wav();
    let counter = Cell::new(0);
    let threshold = Cell::new(usize::MAX);
    let cancel = || {
        counter.set(counter.get() + 1);
        counter.get() >= threshold.get()
    };
    let container = Container::open(
        source(&bytes, usize::MAX),
        bytes.len() as u64,
        limits(),
        cancel,
    )
    .unwrap();
    let mut decoder = container.select(0).unwrap();
    counter.set(0);
    decoder.next_frame().unwrap().unwrap();
    let checks = counter.get();
    drop(decoder);
    assert!(checks > 3);
    for point in 1..=checks {
        threshold.set(usize::MAX);
        let container = Container::open(
            source(&bytes, usize::MAX),
            bytes.len() as u64,
            limits(),
            cancel,
        )
        .unwrap();
        let mut decoder = container.select(0).unwrap();
        counter.set(0);
        threshold.set(point);
        assert_eq!(
            decoder.next_frame().err().unwrap().kind,
            ErrorKind::Cancelled,
            "point {point}"
        );
        assert!(decoder.is_closed());
        threshold.set(usize::MAX);
        assert_eq!(decoder.next_frame().err().unwrap().kind, ErrorKind::State);
    }
}
#[test]
fn decoded_frame_byte_budget_poison_is_not_eof() {
    let (bytes, _) = wav();
    let mut budget = limits();
    budget.max_frame_bytes = 1;
    let mut decoder = Container::open(
        source(&bytes, usize::MAX),
        bytes.len() as u64,
        budget,
        || false,
    )
    .unwrap()
    .select(0)
    .unwrap();
    assert_eq!(decoder.next_frame().err().unwrap().kind, ErrorKind::Limit);
    assert!(decoder.is_closed());
    assert_eq!(decoder.next_frame().err().unwrap().kind, ErrorKind::State);
}
