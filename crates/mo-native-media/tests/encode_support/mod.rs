#![allow(dead_code)]
use mo_media::{EncodeConfig, EncodeLimits, WriteAt};
use mo_native_media::{EncoderOptions, NativeContainer};
use std::{cell::RefCell, io, rc::Rc};
pub fn options() -> EncoderOptions {
    let base = crate::support::options();
    EncoderOptions {
        encode: EncodeLimits::default(),
        process: base.process,
        open_timeout: base.open_timeout,
        call_timeout: base.call_timeout,
    }
}
#[derive(Clone, Default)]
pub struct Sink(pub Rc<RefCell<Vec<u8>>>, pub usize);
impl WriteAt for Sink {
    fn write_at(&mut self, offset: u64, bytes: &[u8]) -> io::Result<usize> {
        let n = bytes
            .len()
            .min(if self.1 == 0 { usize::MAX } else { self.1 });
        let mut out = self.0.borrow_mut();
        let offset = offset as usize;
        if out.len() < offset + n {
            out.resize(offset + n, 0);
        }
        out[offset..offset + n].copy_from_slice(&bytes[..n]);
        Ok(n)
    }
}
pub fn decoded(bytes: &[u8], c: EncodeConfig) -> (Vec<u8>, Vec<f32>) {
    let bundle = crate::support::bundle();
    let mut video = vec![];
    let mut audio = vec![];
    for index in 0..if c.audio_samples > 0 { 2 } else { 1 } {
        let input = move |offset, out: &mut [u8]| crate::support::read(bytes, offset, out, 113);
        let mut decoder = NativeContainer::open(
            &bundle,
            input,
            bytes.len() as u64,
            crate::support::options(),
            &|| false,
        )
        .unwrap()
        .select(index, &|| false)
        .unwrap();
        let pid = decoder.process_id().unwrap();
        assert_eq!(decoder.stream().start_ticks, Some(0));
        assert_eq!(
            decoder.stream().duration_ticks,
            Some(if index == 0 {
                c.video_duration
            } else {
                c.audio_samples
            })
        );
        let mut frames = 0;
        while let Some(frame) = decoder.next_frame(&|| false).unwrap() {
            if index == 0 {
                let period = i64::from(c.video_time_scale) * i64::from(c.frame_rate_den)
                    / i64::from(c.frame_rate_num);
                assert_eq!(frame.info.pts_ticks, Some(frames * period));
                for plane in frame.planes {
                    video.extend(plane.bytes);
                }
            } else {
                assert_eq!(frame.info.pts_ticks, Some(audio.len() as i64 / 2));
                let n = frame.info.samples.unwrap() as usize;
                if frame.info.format == "flt" {
                    audio.extend(
                        frame.planes[0]
                            .bytes
                            .chunks_exact(4)
                            .map(|b| f32::from_le_bytes(b.try_into().unwrap())),
                    );
                } else {
                    assert_eq!(frame.info.format, "fltp");
                    for i in 0..n {
                        for ch in 0..2 {
                            audio.push(f32::from_le_bytes(
                                frame.planes[ch].bytes[i * 4..i * 4 + 4].try_into().unwrap(),
                            ));
                        }
                    }
                }
            }
            frames += 1;
        }
        if index == 0 {
            assert_eq!(frames, i64::from(c.video_frames));
        }
        decoder.close(&|| false).unwrap();
        crate::support::assert_reaped(pid);
    }
    assert_eq!(audio.len() as i64, c.audio_samples * 2);
    (video, audio)
}
