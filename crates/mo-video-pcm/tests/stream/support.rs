#![allow(dead_code)]
use mo_common::{Digest, ResourceId};
use mo_media::{DecodedFrame, FramePlane};
use mo_video_audio::{ByteOrder, ChannelLayout, PcmBlock, PcmLimits};
use mo_video_model::StreamId;
use mo_video_pcm::*;
pub fn origin() -> PcmStreamOrigin {
    PcmStreamOrigin {
        resource: ResourceId::new("a").unwrap(),
        content: Digest::from_sha256([1; 32]),
        stream: StreamId::new("track-2").unwrap(),
        decoder_profile: Digest::from_sha256([2; 32]),
    }
}
pub fn value(sample: i64) -> f32 {
    ((sample * 1637).rem_euclid(64000) - 32000) as f32 / 32768.
}
pub fn frame(first: i64, count: u32, channels: u16, planar: bool) -> DecodedFrame {
    let planes = if planar {
        (0..channels)
            .map(|c| FramePlane {
                row_bytes: count as usize * 4,
                rows: 1,
                bytes: (first..first + i64::from(count))
                    .flat_map(|n| value(n * i64::from(channels) + i64::from(c)).to_le_bytes())
                    .collect(),
            })
            .collect()
    } else {
        vec![FramePlane {
            row_bytes: count as usize * channels as usize * 4,
            rows: 1,
            bytes: (first * i64::from(channels)..(first + i64::from(count)) * i64::from(channels))
                .flat_map(|n| value(n).to_le_bytes())
                .collect(),
        }]
    };
    DecodedFrame {info:serde_json::from_value(serde_json::json!({
        "generation":1,"kind":"audio","timeBase":{"numerator":1,"denominator":16000},
        "ptsTicks":-3+first*2,"durationTicks":0,"width":null,"height":null,
        "sampleRate":8000,"channels":channels,"samples":count,"format":if planar {"fltp"} else {"flt"},
        "channelLayout":if channels==1 {"mono"} else {"stereo"},
        "colors":{"primaries":2,"transfer":2,"matrix":2,"range":0,"chromaLocation":0},
        "sampleAspectRatio":null,"alphaMode":0,"keyFrame":true,"interlaced":false,"topFieldFirst":false
    })).unwrap(),planes}
}
pub fn pcm(first: i64, end: i64, channels: u16) -> PcmBlock {
    PcmBlock::new(
        8000,
        if channels == 1 {
            ChannelLayout::mono()
        } else {
            ChannelLayout::stereo()
        },
        first,
        (first * i64::from(channels)..end * i64::from(channels))
            .map(value)
            .collect(),
        PcmLimits::default(),
        &|| false,
    )
    .unwrap()
}
pub fn streamed(
    total: i64,
    parts: &[u32],
    channels: u16,
) -> (CompletedPcmStream, Vec<UnsealedPcmTile>) {
    let mut builder = PcmStreamBuilder::new(
        origin(),
        1,
        PcmStreamLimits {
            tile_frames: 256,
            ..Default::default()
        },
    )
    .unwrap();
    let mut first = 0;
    let mut step = 0;
    let mut tiles = vec![];
    while first < total {
        let count = parts[step % parts.len()].min((total - first) as u32);
        tiles.extend(
            builder
                .push(
                    &frame(first, count, channels, step % 2 == 0),
                    ByteOrder::Little,
                    &|| false,
                )
                .unwrap(),
        );
        assert!(builder.retained_bytes() <= 256 * channels as usize * 4);
        assert!(tiles.iter().all(|t| t.reference.frames == 256));
        first += i64::from(count);
        step += 1;
    }
    let mut complete = builder.finish_at_eof(&|| false).unwrap();
    if let Some(tail) = complete.tail.take() {
        tiles.push(tail);
    }
    (complete, tiles)
}
