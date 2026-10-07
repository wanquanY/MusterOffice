use mo_video_runtime::{
    authoring::model::*,
    common::*,
    compile::MediaBinding,
    media::*,
    native::*,
    process_limits::*,
    render::*,
    source::{indexed::*, native::*, *},
    visual_source::*,
};
use sha2::{Digest as Hash, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    time::Duration,
};
pub type Source = IndexedVisualSource<NativeVideoDecoder<'static>, NativeVideoTimeline<'static>>;
pub fn reader(path: &Path) -> impl ReadAt + 'static {
    let mut file = File::open(path).unwrap();
    move |offset, bytes: &mut [u8]| {
        file.seek(SeekFrom::Start(offset))?;
        let count = bytes.len().min(4093);
        file.read(&mut bytes[..count])
    }
}
pub fn options() -> WorkerOptions {
    WorkerOptions {
        decode: DecodeLimits {
            max_input_bytes: 1 << 30,
            max_read_bytes: 1 << 32,
            max_frame_bytes: 1 << 20,
            max_streams: 16,
            max_pixels: 1 << 24,
            max_audio_samples: 262144,
            max_channels: 8,
            max_decode_steps: 10000,
        },
        process: ProcessLimits {
            additional_address_bytes: 512 << 20,
            cpu_seconds: 60,
            open_files: 32,
        },
        open_timeout: Duration::from_secs(15),
        call_timeout: Duration::from_secs(15),
    }
}
pub fn color() -> InputColor {
    InputColor {
        encoding: Encoding {
            primaries: Primaries::Bt709,
            transfer: Transfer::Bt1886,
        },
        matrix: Matrix::Bt709,
        range: SignalRange::Limited,
        chroma: Some(ChromaLocation::Left),
        alpha: Alpha::Opaque,
    }
}
pub fn source(bundle: &WorkerBundle, path: &Path) -> (Source, MediaBinding, [u32; 2]) {
    let opt = options();
    let len = path.metadata().unwrap().len();
    let mut index = NativeIndex::open(
        bundle,
        reader(path),
        len,
        NativeIndexOptions {
            index: IndexLimits::default(),
            max_read_bytes: 64 << 20,
            process: opt.process,
            open_timeout: opt.open_timeout,
            call_timeout: opt.call_timeout,
        },
        &|| false,
    )
    .unwrap();
    let ordinal = (0..index.summary().tracks)
        .find(|&i| {
            let IndexPage::Track { track } = index
                .query(IndexQuery::Track { track: i }, &|| false)
                .unwrap()
            else {
                panic!()
            };
            track.handler == *b"vide"
        })
        .unwrap();
    let index_pid = index.process_id().unwrap();
    let timeline = NativeVideoTimeline::new(index, ordinal, &|| false).unwrap();
    let summary = timeline.summary().clone();
    let decoder = NativeVideoDecoder::open_raw_track(
        bundle,
        reader(path),
        len,
        opt,
        summary.track_id,
        &|| false,
    )
    .unwrap();
    let decoder_pid = decoder.process_id().unwrap();
    let dimensions = Canvas {
        width: decoder.stream().width.unwrap(),
        height: decoder.stream().height.unwrap(),
    };
    let binding = MediaBinding {
        resource: ResourceId::new("actual-video").unwrap(),
        digest: Digest::from_sha256(Sha256::digest(fs::read(path).unwrap()).into()),
        stream: StreamId::new(format!("track-{}", summary.track_id)).unwrap(),
        descriptor: MediaStream::Video {
            available: TimeRange {
                start: RationalTime::new(0, 1).unwrap(),
                end: summary.movie_end.to_time_exact().unwrap(),
            },
            dimensions,
            frame_rate: None,
        },
    };
    let raw_bytes = dimensions.width as usize * dimensions.height as usize * 3 / 2;
    let sampler = IndexedVideoSampler::new(
        decoder,
        timeline,
        binding.clone(),
        SourceLimits {
            max_frame_bytes: raw_bytes,
            max_retained_bytes: raw_bytes * 4,
            max_retained_frames: 4,
            ..Default::default()
        },
    )
    .unwrap();
    let source = IndexedVisualSource::new(
        sampler,
        ColorInterpretation::Override(color()),
        VisualSourceLimits {
            max_retained_bytes: dimensions.width as usize * dimensions.height as usize * 16 * 3,
            max_retained_frames: 3,
            ..Default::default()
        },
    )
    .unwrap();
    (source, binding, [index_pid, decoder_pid])
}
pub fn reaped(pid: u32) {
    assert!(matches!(
        rustix::process::waitpid(
            rustix::process::Pid::from_raw(pid as i32),
            rustix::process::WaitOptions::NOHANG
        ),
        Err(rustix::io::Errno::CHILD)
    ));
}
pub struct Output(pub File);
impl WriteAt for Output {
    fn write_at(&mut self, offset: u64, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.seek(SeekFrom::Start(offset))?;
        self.0.write(&bytes[..bytes.len().min(4093)])
    }
}
pub struct Encoder<'a> {
    pub inner: NativeEncoder<'a>,
    pub expected: Vec<Vec<u8>>,
    pub count: usize,
}
impl VideoEncoder for Encoder<'_> {
    type Error = WorkerError;
    fn configuration(&self) -> EncodeConfig {
        self.inner.configuration()
    }
    fn video(&mut self, frame: EncodeVideoFrame<'_>) -> std::result::Result<(), WorkerError> {
        let bytes: Vec<u8> = frame
            .planes
            .iter()
            .flat_map(|p| p.iter().copied())
            .collect();
        assert_eq!(
            bytes, self.expected[self.count],
            "preview/export frame {}",
            self.count
        );
        self.count += 1;
        self.inner.video(frame)
    }
    fn audio(&mut self, _: i64, _: &[f32]) -> std::result::Result<(), WorkerError> {
        panic!("silent")
    }
    fn finish(&mut self) -> std::result::Result<EncodeStats, WorkerError> {
        assert_eq!(self.count, self.expected.len());
        self.inner.finish()
    }
}
pub struct NoAudio;
impl mo_video_runtime::audio::AudioResources for NoAudio {
    fn describe(
        &mut self,
        _: &MediaBinding,
    ) -> mo_video_runtime::audio::Result<mo_video_runtime::audio::AudioAsset> {
        panic!("silent")
    }
    fn read(
        &mut self,
        _: &MediaBinding,
        _: &mo_video_runtime::audio::AudioAsset,
        _: mo_video_runtime::audio::PcmRange,
        _: mo_video_runtime::audio::PcmLimits,
        _: &dyn Fn() -> bool,
    ) -> mo_video_runtime::audio::Result<mo_video_runtime::audio::PcmBlock> {
        panic!("silent")
    }
}
