//! Thin native implementation; the default crate builds without worker/codec linkage.
use crate::VideoDecoder;
use mo_media::{DecodedFrame, ReadAt, StreamInfo};
use mo_native_media::{NativeContainer, NativeDecoder, WorkerBundle, WorkerError, WorkerOptions};

pub struct NativeVideoDecoder<'a> {
    decoder: NativeDecoder<'a>,
    generation: u64,
}
impl<'a> NativeVideoDecoder<'a> {
    /// Always opens a fresh decoder at source origin. Host binds this selected
    /// stream index and immutable bytes to the sampler's resource/stream identity.
    pub fn open(
        bundle: &WorkerBundle,
        source: impl ReadAt + 'a,
        length: u64,
        options: WorkerOptions,
        stream_index: u32,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Self, WorkerError> {
        let decoder = NativeContainer::open(bundle, source, length, options, cancelled)?
            .select(stream_index, cancelled)?;
        Ok(Self {
            decoder,
            generation: 1,
        })
    }
}
impl VideoDecoder for NativeVideoDecoder<'_> {
    type Error = WorkerError;
    fn stream(&self) -> &StreamInfo {
        self.decoder.stream()
    }
    fn generation(&self) -> u64 {
        self.generation
    }
    fn next(&mut self, cancelled: &dyn Fn() -> bool) -> Result<Option<DecodedFrame>, WorkerError> {
        self.decoder.next_frame(cancelled)
    }
    fn seek(&mut self, ticks: i64, cancelled: &dyn Fn() -> bool) -> Result<(), WorkerError> {
        self.decoder.seek(ticks, cancelled)?;
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(WorkerError::Invalid("generation overflow"))?;
        Ok(())
    }
}
