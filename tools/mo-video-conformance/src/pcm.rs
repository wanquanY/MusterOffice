//! Fixture-only in-memory store, exercising the actual canonical byte format.
use mo_common::{Digest, Ticks, from_json_str_with_path};
use mo_time::ExactRatio as R;
use mo_video_audio::{AudioAsset, ChannelLayout, PcmBlock, PcmLimits, PcmRange};
use mo_video_compile::MediaBinding;
use mo_video_model::MediaStream;
use mo_video_pcm::*;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};
use wasm_bindgen::prelude::*;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Fixture {
    binding: MediaBinding,
    first: Ticks,
    pcm: Vec<f32>,
    decode_frames: usize,
    queries: Vec<Query>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Query {
    first: Ticks,
    frames: usize,
}
#[derive(Default)]
struct Store {
    tiles: BTreeMap<u32, PcmTile>,
}
impl PcmTileStore for Store {
    fn lookup(
        &mut self,
        source: &Digest,
        index: u32,
        _: &dyn Fn() -> bool,
    ) -> Result<Option<PcmTileRef>> {
        Ok(self
            .tiles
            .get(&index)
            .filter(|v| &v.reference.source == source)
            .map(|v| v.reference.clone()))
    }
    fn read(
        &mut self,
        reference: &PcmTileRef,
        max_bytes: usize,
        _: &dyn Fn() -> bool,
    ) -> Result<Vec<u8>> {
        let tile = &self.tiles[&reference.index];
        if tile.bytes.len() > max_bytes {
            return Err(PcmError::Budget);
        }
        Ok(tile.bytes.clone())
    }
}
fn run(input: &str) -> std::result::Result<Value, String> {
    if input.len() > 4 << 20 {
        return Err("fixture too large".into());
    }
    let f: Fixture = from_json_str_with_path(input).map_err(|e| e.to_string())?;
    if f.queries.len() > 32
        || f.pcm.len() > 65536
        || f.decode_frames > f.pcm.len()
        || f.first.get().checked_add(f.pcm.len() as i64).is_none()
    {
        return Err("fixture limits".into());
    }
    let MediaStream::Audio {
        sample_rate,
        channels: 1,
        ..
    } = f.binding.descriptor
    else {
        return Err("mono fixture required".into());
    };
    let source = Arc::new(
        PcmSource::new(
            f.binding,
            AudioAsset {
                sample_rate,
                layout: ChannelLayout::mono(),
                sample_zero: R::integer(0),
                available: PcmRange::new(f.first.get(), f.first.get() + f.pcm.len() as i64)
                    .map_err(|e| e.to_string())?,
            },
            &Digest::from_sha256([7; 32]),
            256,
            256,
        )
        .map_err(|e| e.to_string())?,
    );
    let mut builder = PcmTileBuilder::new(Arc::clone(&source));
    let mut store = Store::default();
    let mut first = 0usize;
    let mut step = 0;
    while first < f.decode_frames {
        let end = (first + [13, 997, 37, 257][step % 4]).min(f.decode_frames);
        let block = PcmBlock::new(
            sample_rate,
            ChannelLayout::mono(),
            source.asset().available.first + first as i64,
            f.pcm[first..end].to_vec(),
            PcmLimits::default(),
            &|| false,
        )
        .map_err(|e| e.to_string())?;
        let candidate = builder
            .prepare(&block, PcmBuildLimits::default(), &|| false)
            .map_err(|e| e.to_string())?;
        for tile in candidate.tiles {
            store.tiles.insert(tile.reference.index, tile);
        }
        builder = candidate.next;
        first = end;
        step += 1;
    }
    let refs: Vec<_> = store
        .tiles
        .values()
        .map(|tile| tile.reference.clone())
        .collect();
    let mut reader = PcmTileReader::new(
        Arc::clone(&source),
        PcmReadLimits {
            max_cached_tiles: 2,
            max_cached_bytes: 2048,
            ..PcmReadLimits::default()
        },
    )
    .map_err(|e| e.to_string())?;
    let mut results = vec![];
    for q in f.queries {
        if q.frames > 4096 {
            return Err("fixture output limits".into());
        }
        let end = q
            .first
            .get()
            .checked_add(q.frames as i64)
            .ok_or("query overflow")?;
        match reader.read(source.binding().unwrap(),PcmRange::new(q.first.get(),end).map_err(|e|e.to_string())?,&mut store,&||false) {
            Ok(read)=>results.push(json!({"status":"read","bits":read.pcm.samples().iter().map(|v|v.to_bits()).collect::<Vec<_>>(),"tileReads":read.stats.tile_reads,"cacheHits":read.stats.cache_hits,"peakBytes":read.stats.peak_working_bytes})),
            Err(error)=>results.push(json!({"status":"rejected","error":error.to_string()})),
        }
    }
    Ok(
        json!({"status":"complete","sourceIdentity":source.identity(),"sourceComplete":builder.complete(),"tiles":refs,"results":results}),
    )
}
#[wasm_bindgen]
pub fn video_pcm_conformance(input: &str) -> String {
    match run(input) {
        Ok(value) => value.to_string(),
        Err(error) => json!({"status":"invalidFixture","error":error}).to_string(),
    }
}
