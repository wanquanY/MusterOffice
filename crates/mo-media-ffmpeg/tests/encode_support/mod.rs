#![allow(dead_code)]
use mo_media_ffmpeg::*;
use std::{cell::RefCell, io, rc::Rc};

pub fn config(profile: EncodeProfile, frames: u32, fractional: bool, audio: bool) -> EncodeConfig {
    let (scale, num, den, rate) = if fractional {
        (30000, 30000, 1001, 44100)
    } else {
        (48000, 25, 1, 48000)
    };
    let period = i64::from(scale) * i64::from(den) / i64::from(num);
    let duration = (i64::from(frames) - 1) * period + period / 3 + 1;
    EncodeConfig {
        profile,
        width: 128,
        height: 96,
        video_time_scale: scale,
        frame_rate_num: num,
        frame_rate_den: den,
        video_duration: duration,
        video_frames: frames,
        video_bit_rate: 2_000_000,
        audio_sample_rate: rate,
        audio_channels: 2,
        audio_bit_rate: 128000,
        gop_frames: 25,
        audio_samples: if audio {
            (duration * i64::from(rate) + i64::from(scale) - 1) / i64::from(scale)
        } else {
            0
        },
    }
}
pub fn planes(index: u32) -> [Vec<u8>; 3] {
    [
        (0..128 * 96)
            .map(|i| (16 + (i % 128 + i / 128 + index * 3) % 220) as u8)
            .collect(),
        (0..64 * 48)
            .map(|i| (80 + (i % 64 + index) % 48) as u8)
            .collect(),
        (0..64 * 48)
            .map(|i| (130 + (i / 64 + index) % 48) as u8)
            .collect(),
    ]
}
pub fn frame(c: EncodeConfig, i: u32, planes: &[Vec<u8>; 3]) -> EncodeVideoFrame<'_> {
    let period =
        i64::from(c.video_time_scale) * i64::from(c.frame_rate_den) / i64::from(c.frame_rate_num);
    let pts = i64::from(i) * period;
    EncodeVideoFrame {
        pts,
        duration: period.min(c.video_duration - pts),
        planes: [&planes[0], &planes[1], &planes[2]],
        strides: [128, 64, 64],
    }
}
pub fn pcm(c: EncodeConfig, start: i64, count: usize) -> Vec<f32> {
    (0..count * 2)
        .map(|i| {
            let frequency = if i % 2 == 0 { 440.0 } else { 880.0 };
            (0.25
                * (std::f64::consts::TAU * frequency * (start + i as i64 / 2) as f64
                    / f64::from(c.audio_sample_rate))
                .sin()) as f32
        })
        .collect()
}
#[derive(Clone, Default)]
pub struct Sink(pub Rc<RefCell<Vec<u8>>>, pub usize);
impl WriteAt for Sink {
    fn write_at(&mut self, offset: u64, bytes: &[u8]) -> io::Result<usize> {
        let n = bytes
            .len()
            .min(if self.1 == 0 { usize::MAX } else { self.1 });
        let start = usize::try_from(offset).unwrap();
        let mut out = self.0.borrow_mut();
        if out.len() < start + n {
            out.resize(start + n, 0);
        }
        out[start..start + n].copy_from_slice(&bytes[..n]);
        Ok(n)
    }
}
pub fn feed(encoder: &mut Encoder<'_>, c: EncodeConfig, chunk: usize) -> Result<(), MediaError> {
    let mut vi = 0;
    let mut ai = 0;
    let period =
        i64::from(c.video_time_scale) * i64::from(c.frame_rate_den) / i64::from(c.frame_rate_num);
    while vi < c.video_frames || ai < c.audio_samples {
        if vi < c.video_frames
            && (ai >= c.audio_samples
                || i64::from(vi) * period * i64::from(c.audio_sample_rate)
                    <= ai * i64::from(c.video_time_scale))
        {
            encoder.video(frame(c, vi, &planes(vi)))?;
            vi += 1;
        } else {
            let n = (c.audio_samples - ai).min(chunk as i64) as usize;
            encoder.audio(ai, &pcm(c, ai, n))?;
            ai += n as i64;
        }
    }
    Ok(())
}
pub fn encode(c: EncodeConfig, short: usize, chunk: usize) -> (Vec<u8>, EncodeStats) {
    let sink = Sink::default();
    let mut encoder = Encoder::open(
        Sink(sink.0.clone(), short),
        c,
        EncodeLimits::default(),
        || false,
    )
    .unwrap();
    feed(&mut encoder, c, chunk).unwrap();
    let stats = encoder.finish().unwrap();
    assert!(encoder.is_closed());
    assert_eq!(encoder.finish().unwrap_err().kind, ErrorKind::State);
    let bytes = sink.0.borrow().clone();
    assert_eq!(bytes.len() as u64, stats.byte_length);
    assert_eq!(stats.video_frames, u64::from(c.video_frames));
    assert_eq!(stats.video_packets, stats.video_frames);
    assert_eq!(stats.audio_samples, c.audio_samples as u64);
    (bytes, stats)
}
pub fn decode(bytes: &[u8], index: u32) -> Decoder<'_> {
    Container::open(
        move |offset: u64, out: &mut [u8]| {
            let start = offset as usize;
            let n = out.len().min(bytes.len() - start).min(113);
            out[..n].copy_from_slice(&bytes[start..start + n]);
            Ok(n)
        },
        bytes.len() as u64,
        DecodeLimits {
            max_input_bytes: 128 << 20,
            max_read_bytes: 256 << 20,
            max_frame_bytes: 16 << 20,
            max_streams: 4,
            max_pixels: 1024 * 1024,
            max_audio_samples: 262144,
            max_channels: 2,
            max_decode_steps: 16384,
        },
        || false,
    )
    .unwrap()
    .select(index)
    .unwrap()
}
pub fn decoded(bytes: &[u8], c: EncodeConfig) -> (Vec<u8>, Vec<f32>) {
    let mut video = decode(bytes, 0);
    assert_eq!(video.stream().start_ticks, Some(0));
    assert_eq!(video.stream().duration_ticks, Some(c.video_duration));
    let period =
        i64::from(c.video_time_scale) * i64::from(c.frame_rate_den) / i64::from(c.frame_rate_num);
    let mut raw = Vec::new();
    let mut count = 0;
    while let Some(frame) = video.next_frame().unwrap() {
        assert_eq!(frame.info.pts_ticks, Some(i64::from(count) * period));
        assert_eq!(
            frame.info.time_base,
            TimeBase {
                numerator: 1,
                denominator: c.video_time_scale
            }
        );
        for plane in frame.planes {
            raw.extend(plane.bytes);
        }
        count += 1;
    }
    assert_eq!(count, c.video_frames);
    let mut samples = Vec::new();
    if c.audio_samples > 0 {
        let mut audio = decode(bytes, 1);
        assert_eq!(audio.stream().start_ticks, Some(0));
        assert_eq!(audio.stream().duration_ticks, Some(c.audio_samples));
        while let Some(frame) = audio.next_frame().unwrap() {
            assert_eq!(frame.info.pts_ticks, Some(samples.len() as i64 / 2));
            assert_eq!(
                frame.info.time_base,
                TimeBase {
                    numerator: 1,
                    denominator: c.audio_sample_rate
                }
            );
            let n = frame.info.samples.unwrap() as usize;
            if frame.info.format == "flt" {
                samples.extend(
                    frame.planes[0]
                        .bytes
                        .chunks_exact(4)
                        .map(|b| f32::from_le_bytes(b.try_into().unwrap())),
                );
            } else {
                assert_eq!(frame.info.format, "fltp");
                for i in 0..n {
                    for ch in 0..2 {
                        samples.push(f32::from_le_bytes(
                            frame.planes[ch].bytes[i * 4..i * 4 + 4].try_into().unwrap(),
                        ));
                    }
                }
            }
        }
        assert_eq!(samples.len() as i64, c.audio_samples * 2);
    }
    (raw, samples)
}
