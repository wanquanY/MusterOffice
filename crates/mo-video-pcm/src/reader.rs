use crate::{PcmError, PcmSource, PcmTileRef, Result, check};
use mo_common::Digest;
use mo_video_audio::{PcmBlock, PcmLimits, PcmRange};
use mo_video_compile::MediaBinding;
use sha2::{Digest as _, Sha256};
use std::{collections::VecDeque, sync::Arc};
/// Authorized host storage. References are atomically published by the canonical
/// decode pipeline, immutable for a given source/index. Missing is pending work,
/// never silence. A read callback must respect the supplied byte bound.
pub trait PcmTileStore {
    fn lookup(
        &mut self,
        source: &Digest,
        index: u32,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Option<PcmTileRef>>;
    fn read(
        &mut self,
        reference: &PcmTileRef,
        max_bytes: usize,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Vec<u8>>;
}
#[derive(Debug, Clone, Copy)]
pub struct PcmReadLimits {
    pub pcm: PcmLimits,
    /// Active output + cached PCM + current encoded bytes + decoded tile.
    /// Excludes host/provider and caller-retained prior outputs.
    pub max_working_bytes: usize,
    pub max_cached_bytes: usize,
    pub max_cached_tiles: usize,
    pub max_tiles_per_read: usize,
}
impl Default for PcmReadLimits {
    fn default() -> Self {
        Self {
            pcm: PcmLimits::default(),
            max_working_bytes: 64 << 20,
            max_cached_bytes: 16 << 20,
            max_cached_tiles: 32,
            max_tiles_per_read: 1024,
        }
    }
}
#[derive(Debug, Default, Clone, Copy)]
pub struct PcmReadStats {
    pub tile_reads: usize,
    pub cache_hits: usize,
    pub peak_working_bytes: usize,
    pub retained_bytes: usize,
}
#[derive(Debug)]
pub struct ReadPcm {
    pub pcm: PcmBlock,
    pub stats: PcmReadStats,
}
pub struct PcmTileReader {
    source: Arc<PcmSource>,
    limits: PcmReadLimits,
    cache: VecDeque<(u32, Arc<PcmBlock>)>,
    retained: usize,
}
impl PcmTileReader {
    pub fn new(source: Arc<PcmSource>, limits: PcmReadLimits) -> Result<Self> {
        if limits.max_working_bytes == 0
            || limits.max_working_bytes > 1 << 30
            || limits.max_cached_bytes > limits.max_working_bytes
            || limits.max_cached_tiles > 65536
            || limits.max_tiles_per_read == 0
            || limits.max_tiles_per_read > 65536
        {
            return Err(PcmError::Invalid("PCM reader limits"));
        }
        Ok(Self {
            source,
            limits,
            cache: VecDeque::new(),
            retained: 0,
        })
    }
    pub fn source(&self) -> &PcmSource {
        &self.source
    }
    pub fn retained_bytes(&self) -> usize {
        self.retained
    }
    pub fn clear(&mut self) {
        self.cache.clear();
        self.retained = 0;
    }
    pub fn read(
        &mut self,
        binding: &MediaBinding,
        range: PcmRange,
        store: &mut dyn PcmTileStore,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<ReadPcm> {
        check(cancelled)?;
        if Some(binding) != self.source.binding() {
            return Err(PcmError::Invalid("changed source binding"));
        }
        self.read_window(range, store, cancelled)
    }
    pub fn read_retimed(
        &mut self,
        request: &mo_video_audio::RetimeRequest,
        range: PcmRange,
        store: &mut dyn PcmTileStore,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<ReadPcm> {
        check(cancelled)?;
        if self.source.retime_identity() != Some(request.identity()) {
            return Err(PcmError::Invalid("changed derived PCM binding"));
        }
        self.read_window(range, store, cancelled)
    }
    fn read_window(
        &mut self,
        range: PcmRange,
        store: &mut dyn PcmTileStore,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<ReadPcm> {
        let asset = self.source.asset();
        if range.first > range.end
            || range.first < asset.available.first
            || range.end > asset.available.end
        {
            return Err(PcmError::Invalid("requested PCM range"));
        }
        let frames = usize::try_from(i128::from(range.end) - i128::from(range.first))
            .map_err(|_| PcmError::Budget)?;
        let channels = asset.layout.len();
        let count = frames.checked_mul(channels).ok_or(PcmError::Budget)?;
        let bytes = count.checked_mul(4).ok_or(PcmError::Budget)?;
        if frames > self.limits.pcm.max_frames
            || bytes > self.limits.pcm.max_bytes
            || bytes > self.limits.max_working_bytes
        {
            return Err(PcmError::Budget);
        }
        let indices = if frames == 0 {
            None
        } else {
            Some((
                self.source.tile_index(range.first)?,
                self.source.tile_index(range.end - 1)?,
            ))
        };
        if indices.is_some_and(|(a, b)| {
            u64::from(b) - u64::from(a) + 1 > self.limits.max_tiles_per_read as u64
        }) {
            return Err(PcmError::Budget);
        }
        self.evict_for(bytes)?;
        let mut samples = Vec::new();
        samples
            .try_reserve_exact(count)
            .map_err(|_| PcmError::Budget)?;
        samples.resize(count, 0f32);
        let mut stats = PcmReadStats {
            peak_working_bytes: bytes + self.retained,
            ..PcmReadStats::default()
        };
        if let Some((a, b)) = indices {
            for index in a..=b {
                check(cancelled)?;
                let block = self.tile(index, store, bytes, &mut stats, cancelled)?;
                let first = range.first.max(block.first_frame());
                let end = range.end.min(block.end_frame());
                let from = (first - block.first_frame()) as usize * channels;
                let to = (first - range.first) as usize * channels;
                let n = (end - first) as usize * channels;
                samples[to..to + n].copy_from_slice(&block.samples()[from..from + n]);
            }
        }
        stats.retained_bytes = self.retained;
        let asset = self.source.asset();
        let pcm = PcmBlock::new(
            asset.sample_rate,
            asset.layout.clone(),
            range.first,
            samples,
            self.limits.pcm,
            cancelled,
        )?;
        Ok(ReadPcm { pcm, stats })
    }
    fn evict_for(&mut self, additional: usize) -> Result<()> {
        while self
            .retained
            .checked_add(additional)
            .is_none_or(|n| n > self.limits.max_working_bytes)
        {
            let Some((_, block)) = self.cache.pop_front() else {
                return Err(PcmError::Budget);
            };
            self.retained -= block.bytes();
        }
        Ok(())
    }
    fn tile(
        &mut self,
        index: u32,
        store: &mut dyn PcmTileStore,
        output_bytes: usize,
        stats: &mut PcmReadStats,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Arc<PcmBlock>> {
        if let Some(at) = self.cache.iter().position(|(n, _)| *n == index) {
            let entry = self.cache.remove(at).expect("located PCM cache entry");
            let block = Arc::clone(&entry.1);
            self.cache.push_back(entry);
            stats.cache_hits += 1;
            return Ok(block);
        }
        let reference = store
            .lookup(self.source.identity(), index, cancelled)?
            .ok_or(PcmError::Missing { index })?;
        check(cancelled)?;
        if reference.index != index {
            return Err(PcmError::Invalid("PCM tile lookup index"));
        }
        self.source.validate_ref(&reference)?;
        let bytes = usize::try_from(reference.byte_length.get()).map_err(|_| PcmError::Budget)?;
        self.evict_for(
            output_bytes
                .checked_add(bytes.checked_mul(2).ok_or(PcmError::Budget)?)
                .ok_or(PcmError::Budget)?,
        )?;
        stats.peak_working_bytes = stats
            .peak_working_bytes
            .max(self.retained + output_bytes + bytes * 2);
        let data = store.read(&reference, bytes, cancelled)?;
        check(cancelled)?;
        if data.len() != bytes {
            return Err(PcmError::Invalid("PCM tile byte length"));
        }
        let mut hash = Sha256::new();
        for chunk in data.chunks(16384) {
            check(cancelled)?;
            hash.update(chunk);
        }
        if Digest::from_sha256(hash.finalize().into()) != reference.sha256 {
            return Err(PcmError::Invalid("PCM tile checksum"));
        }
        let mut samples = Vec::new();
        samples
            .try_reserve_exact(bytes / 4)
            .map_err(|_| PcmError::Budget)?;
        for chunk in data.chunks(16384) {
            check(cancelled)?;
            for word in chunk.chunks_exact(4) {
                samples.push(f32::from_le_bytes(word.try_into().expect("four PCM bytes")));
            }
        }
        let asset = self.source.asset();
        let block = Arc::new(PcmBlock::new(
            asset.sample_rate,
            asset.layout.clone(),
            reference.first_sample.get(),
            samples,
            PcmLimits {
                max_frames: self.source.tile_frames() as usize,
                max_bytes: bytes,
            },
            cancelled,
        )?);
        stats.tile_reads += 1;
        if bytes <= self.limits.max_cached_bytes && self.limits.max_cached_tiles > 0 {
            while self.retained + bytes > self.limits.max_cached_bytes
                || self.cache.len() >= self.limits.max_cached_tiles
            {
                let (_, old) = self.cache.pop_front().expect("cached bytes counted");
                self.retained -= old.bytes();
            }
            self.retained += bytes;
            self.cache.push_back((index, Arc::clone(&block)));
        }
        Ok(block)
    }
}
