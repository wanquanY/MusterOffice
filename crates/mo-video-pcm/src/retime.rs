use crate::{
    PcmBuildLimits, PcmError, PcmReadLimits, PcmSource, PcmTile, PcmTileBuilder, PcmTileReader,
    PcmTileStore, PreparedPcmTiles, ReadPcm, Result, check,
};
use mo_video_audio::{
    PcmBlock, PcmRange, RetimeRequest, StretchEngine, StretchInput, StretchLimits, StretchStream,
};
use std::sync::Arc;

/// Sequential DSP producer with a retained publication candidate. Failed blob
/// writes never force the caller to run an already consumed DSP step again.
pub struct RetimeMaterializer<E> {
    request: RetimeRequest,
    stream: StretchStream<E>,
    builder: PcmTileBuilder,
    input: Option<PcmBlock>,
    candidate: Option<PreparedPcmTiles>,
}
impl<E: StretchEngine> RetimeMaterializer<E> {
    pub fn new(
        request: RetimeRequest,
        engine: E,
        tile_frames: u32,
        limits: StretchLimits,
        bits: u64,
    ) -> Result<Self> {
        let source = Arc::new(PcmSource::retimed(&request, tile_frames, bits)?);
        let stream = StretchStream::from_schedule(
            engine,
            request.plan().sample_rate(),
            request.layout().clone(),
            request.plan().schedule().clone(),
            limits,
        )?;
        Ok(Self {
            request,
            stream,
            builder: PcmTileBuilder::new(source),
            input: None,
            candidate: None,
        })
    }
    pub fn request(&self) -> &RetimeRequest {
        &self.request
    }
    pub fn source(&self) -> &PcmSource {
        self.builder.source()
    }
    pub fn engine(&self) -> &E {
        self.stream.engine()
    }
    pub fn complete(&self) -> bool {
        self.builder.complete()
    }
    /// Rust buffers/carry/candidate payload only; DSP and host store are separate.
    pub fn retained_bytes(&self) -> usize {
        self.stream.buffer_bytes()
            + self.builder.retained_bytes()
            + self.input.as_ref().map_or(0, PcmBlock::bytes)
            + self.candidate.as_ref().map_or(0, |c| {
                c.next.retained_bytes() + c.tiles.iter().map(|t| t.bytes.len()).sum::<usize>()
            })
    }
    /// Some(empty) is a valid carry-only candidate; accept it before computing
    /// again. None means the exact finite output is complete. A pending candidate
    /// is returned unchanged, allowing retry after a failed store publication.
    pub fn prepare_next(
        &mut self,
        input: &mut dyn StretchInput,
        limits: PcmBuildLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Option<&[PcmTile]>> {
        check(cancelled)?;
        if self.candidate.is_none() && !self.complete() {
            if self.input.is_none() {
                self.input = self.stream.next(input, cancelled)?;
            }
            match &self.input {
                Some(pcm) => self.candidate = Some(self.builder.prepare(pcm, limits, cancelled)?),
                None => {
                    self.builder.verify_eof()?;
                    return Ok(None);
                }
            }
        }
        Ok(self.candidate.as_ref().map(|c| c.tiles.as_slice()))
    }
    /// Host asserts that every immutable blob/ref of this candidate has been
    /// atomically published (or it contains no blobs). This is not a disk receipt.
    pub fn accept_published(&mut self) -> Result<()> {
        let candidate = self
            .candidate
            .take()
            .ok_or(PcmError::Invalid("no derived publication candidate"))?;
        self.builder = candidate.next;
        self.input = None;
        Ok(())
    }
}
/// A bound derived-resource reader; the host composes it with decoded resources
/// and retains revision/pin/authorization/task ownership.
pub struct CanonicalRetimedAudio<S> {
    reader: PcmTileReader,
    store: S,
}
impl<S: PcmTileStore> CanonicalRetimedAudio<S> {
    pub fn new(
        request: &RetimeRequest,
        store: S,
        tile_frames: u32,
        limits: PcmReadLimits,
        bits: u64,
    ) -> Result<Self> {
        Ok(Self {
            reader: PcmTileReader::new(
                Arc::new(PcmSource::retimed(request, tile_frames, bits)?),
                limits,
            )?,
            store,
        })
    }
    pub fn read(
        &mut self,
        request: &RetimeRequest,
        range: PcmRange,
        limits: mo_video_audio::PcmLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> mo_video_audio::Result<ReadPcm> {
        let frames = usize::try_from(i128::from(range.end) - i128::from(range.first))
            .map_err(|_| mo_video_audio::AudioError::Invalid("derived PCM range"))?;
        if frames > limits.max_frames
            || frames
                .checked_mul(request.layout().len())
                .and_then(|n| n.checked_mul(4))
                .is_none_or(|n| n > limits.max_bytes)
        {
            return Err(mo_video_audio::AudioError::Budget);
        }
        self.reader
            .read_retimed(request, range, &mut self.store, cancelled)
            .map_err(|e| match e {
                PcmError::Missing { .. } => mo_video_audio::AudioError::PendingDerived {
                    identity: request.identity().clone(),
                    first_sample: range.first,
                    end_sample: range.end,
                },
                PcmError::Audio(e) => e,
                PcmError::Cancelled => mo_video_audio::AudioError::Cancelled,
                PcmError::Budget => mo_video_audio::AudioError::Budget,
                PcmError::Time(e) => mo_video_audio::AudioError::Time(e),
                PcmError::Canonical(e) => mo_video_audio::AudioError::Canonical(e),
                PcmError::Invalid(e) => mo_video_audio::AudioError::Invalid(e),
                PcmError::Store(e) => mo_video_audio::AudioError::Resource(e),
            })
    }
    pub fn retained_bytes(&self) -> usize {
        self.reader.retained_bytes()
    }
    pub fn store_mut(&mut self) -> &mut S {
        &mut self.store
    }
    pub fn clear(&mut self) {
        self.reader.clear();
    }
}
