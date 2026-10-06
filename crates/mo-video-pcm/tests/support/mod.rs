#![allow(dead_code)]
use mo_common::{Digest, RationalTime, ResourceId};
use mo_time::ExactRatio as R;
use mo_video_audio::{AudioAsset, ChannelLayout, PcmBlock, PcmLimits, PcmRange};
use mo_video_compile::MediaBinding;
use mo_video_model::*;
use mo_video_pcm::*;
use std::{collections::BTreeMap, sync::Arc};
pub fn source(first: i64, end: i64, tile_frames: u32) -> Arc<PcmSource> {
    let binding = MediaBinding {
        resource: ResourceId::new("audio").unwrap(),
        digest: Digest::from_sha256([1; 32]),
        stream: StreamId::new("audio0").unwrap(),
        descriptor: MediaStream::Audio {
            available: TimeRange {
                start: RationalTime::new(first, 8000).unwrap(),
                end: RationalTime::new(end, 8000).unwrap(),
            },
            sample_rate: 8000,
            channels: 1,
        },
    };
    let asset = AudioAsset {
        sample_rate: 8000,
        layout: ChannelLayout::mono(),
        sample_zero: R::integer(0),
        available: PcmRange::new(first, end).unwrap(),
    };
    Arc::new(
        PcmSource::new(
            binding,
            asset,
            &Digest::from_sha256([2; 32]),
            tile_frames,
            256,
        )
        .unwrap(),
    )
}
pub fn value(index: i64) -> f32 {
    ((index * 1637).rem_euclid(64000) - 32000) as f32 / 32768.
}
pub fn block(first: i64, end: i64) -> PcmBlock {
    PcmBlock::new(
        8000,
        ChannelLayout::mono(),
        first,
        (first..end).map(value).collect(),
        PcmLimits::default(),
        &|| false,
    )
    .unwrap()
}
#[derive(Default)]
pub struct Memory {
    pub tiles: BTreeMap<u32, PcmTile>,
    pub reads: usize,
    pub lookups: usize,
    pub wrong_index: bool,
}
impl PcmTileStore for Memory {
    fn lookup(
        &mut self,
        source: &Digest,
        index: u32,
        _: &dyn Fn() -> bool,
    ) -> Result<Option<PcmTileRef>> {
        self.lookups += 1;
        let index = if self.wrong_index { 0 } else { index };
        Ok(self
            .tiles
            .get(&index)
            .filter(|t| &t.reference.source == source)
            .map(|t| t.reference.clone()))
    }
    fn read(
        &mut self,
        reference: &PcmTileRef,
        max_bytes: usize,
        _: &dyn Fn() -> bool,
    ) -> Result<Vec<u8>> {
        self.reads += 1;
        let bytes = &self.tiles[&reference.index].bytes;
        assert!(bytes.len() <= max_bytes);
        Ok(bytes.clone())
    }
}
pub fn materialize(source: Arc<PcmSource>, chunks: &[i64]) -> Memory {
    let mut builder = PcmTileBuilder::new(Arc::clone(&source));
    let mut memory = Memory::default();
    let mut first = source.asset().available.first;
    let mut step = 0;
    while first < source.asset().available.end {
        let end = (first + chunks[step % chunks.len()]).min(source.asset().available.end);
        let candidate = builder
            .prepare(&block(first, end), PcmBuildLimits::default(), &|| false)
            .unwrap();
        for tile in candidate.tiles {
            memory.tiles.insert(tile.reference.index, tile);
        }
        builder = candidate.next;
        first = end;
        step += 1;
    }
    assert!(builder.complete());
    builder.verify_eof().unwrap();
    memory
}
