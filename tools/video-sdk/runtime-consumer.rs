//! Standalone registry consumer. File I/O belongs to this explicit test host.
use mo_video_runtime::authoring::{
    edit::{EditContract, Operation},
    model::*,
};
use mo_video_runtime::{
    audio::*, authoring::*, common::*, compile::*, export::*, media::*, native::*,
    process_limits::*, render::*, source::*,
};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

fn options() -> WorkerOptions {
    WorkerOptions {
        decode: DecodeLimits {
            max_input_bytes: 1 << 30,
            max_read_bytes: 1 << 32,
            max_frame_bytes: 32 << 20,
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
fn input(path: &Path) -> impl ReadAt + 'static {
    let mut file = File::open(path).unwrap();
    move |offset, target: &mut [u8]| {
        file.seek(SeekFrom::Start(offset))?;
        let limit = target.len().min(4093);
        file.read(&mut target[..limit])
    }
}
struct Output(File);
impl WriteAt for Output {
    fn write_at(&mut self, offset: u64, bytes: &[u8]) -> io::Result<usize> {
        self.0.seek(SeekFrom::Start(offset))?;
        self.0.write(&bytes[..bytes.len().min(4093)])
    }
}
struct RecordedEncoder<'a> {
    inner: NativeEncoder<'a>,
    expected: File,
}
impl VideoEncoder for RecordedEncoder<'_> {
    type Error = WorkerError;
    fn configuration(&self) -> EncodeConfig {
        self.inner.configuration()
    }
    fn video(&mut self, frame: EncodeVideoFrame<'_>) -> std::result::Result<(), WorkerError> {
        let planes = frame.planes;
        self.inner.video(frame)?;
        for plane in planes {
            self.expected.write_all(plane)?;
        }
        Ok(())
    }
    fn audio(&mut self, first: i64, samples: &[f32]) -> std::result::Result<(), WorkerError> {
        self.inner.audio(first, samples)
    }
    fn finish(&mut self) -> std::result::Result<EncodeStats, WorkerError> {
        self.inner.finish()
    }
}
struct Visual {
    sampler: VideoSampler<source::native::NativeVideoDecoder<'static>>,
    stream: StreamInfo,
}
use mo_video_runtime::source;
impl VisualResources for Visual {
    fn media(
        &mut self,
        binding: &MediaBinding,
        sample: &TimeSample,
        check: &dyn Fn() -> bool,
    ) -> mo_video_runtime::render::Result<ResolvedMedia> {
        let selected = self
            .sampler
            .sample(binding, sample, SampleMode::Seek, check)
            .map_err(|e| RenderError::Resource(e.to_string()))?;
        let SourceSelection::Frame(value) = selected.selection else {
            return Err(RenderError::Resource("fixture frame unavailable".into()));
        };
        let color = InputColor::from_codec(&value.frame.info, false, None)?;
        let normalized = normalize_frame(&value.frame, color, PixelLimits::default(), check)?;
        Ok(ResolvedMedia {
            binding: binding.clone(),
            interval: Some(value.interval),
            image: Arc::new(normalized.image),
            geometry: SourceGeometry::from_media(&value.frame.info, &self.stream)?,
        })
    }
}
struct NoAudio;
impl AudioResources for NoAudio {
    fn describe(&mut self, _: &MediaBinding) -> mo_video_runtime::audio::Result<AudioAsset> {
        Err(AudioError::Unsupported("silent export fixture"))
    }
    fn read(
        &mut self,
        _: &MediaBinding,
        _: &AudioAsset,
        _: PcmRange,
        _: PcmLimits,
        _: &dyn Fn() -> bool,
    ) -> mo_video_runtime::audio::Result<PcmBlock> {
        Err(AudioError::Unsupported("silent export fixture"))
    }
}
fn track(id: &str, kind: TrackKind) -> Track {
    Track {
        id: TrackId::new(id).unwrap(),
        name: id.into(),
        kind,
        locked: false,
        muted: false,
        solo: false,
        clips: vec![],
    }
}
fn clip(id: &str, source: ClipSource, timeline: TimeRange) -> Clip {
    Clip {
        id: ClipId::new(id).unwrap(),
        name: id.into(),
        timeline,
        source,
        effects: vec![],
        audio: AudioTimePolicy::default(),
        audio_context: None,
    }
}
fn rgba(project: &VideoProject, visual: &mut Visual, frame: i64) -> Vec<u8> {
    let plan = project
        .frame(
            FrameRequest {
                expected_digest: project.digest().clone(),
                frame: Ticks::new(frame),
            },
            &|| false,
        )
        .unwrap();
    render_frame(
        &plan,
        visual,
        Sampling::Bilinear,
        RenderLimits::default(),
        &|| false,
    )
    .unwrap()
    .image
    .to_srgb8(
        OutputOptions {
            tone_map: None,
            alpha: OutputAlpha::Straight,
            max_bytes: 1 << 20,
        },
        &|| false,
    )
    .unwrap()
    .rgba
}
fn main() {
    let output = PathBuf::from(std::env::var_os("MO_CONSUMER_OUTPUT").unwrap());
    fs::create_dir(&output).unwrap();
    let bundle = WorkerBundle::open(
        PathBuf::from(std::env::var_os("MO_CONSUMER_WORKER").unwrap()),
        std::env::var("MO_CONSUMER_WORKER_SHA256").unwrap(),
    )
    .unwrap();
    let fixture = PathBuf::from(std::env::var_os("MO_CONSUMER_FIXTURE").unwrap());
    let length = fs::metadata(&fixture).unwrap().len();
    let container =
        NativeContainer::open(&bundle, input(&fixture), length, options(), &|| false).unwrap();
    let streams = container.streams().to_vec();
    drop(container);
    let stream = streams
        .iter()
        .find(|s| s.kind == MediaKind::Video)
        .unwrap()
        .clone();
    let audio = streams.iter().find(|s| s.kind == MediaKind::Audio).unwrap();
    assert_eq!((stream.width, stream.height), (Some(128), Some(96)));
    let mut decoded = NativeContainer::open(&bundle, input(&fixture), length, options(), &|| false)
        .unwrap()
        .select(audio.index, &|| false)
        .unwrap();
    let mut samples = 0u64;
    while let Some(frame) = decoded.next_frame(&|| false).unwrap() {
        samples += u64::from(frame.info.samples.unwrap());
    }
    decoded.close(&|| false).unwrap();
    assert_eq!(samples, 144000);
    // This particular generated fixture has a verified continuous [0,3) span.
    // Production import must establish spans/edit lists, not infer them from duration.
    let span = TimeRange {
        start: RationalTime::new(0, 1).unwrap(),
        end: RationalTime::new(3, 1).unwrap(),
    };
    let binding = MediaBinding {
        resource: ResourceId::new("source").unwrap(),
        stream: StreamId::new("video").unwrap(),
        digest: Digest::try_from(std::env::var("MO_CONSUMER_FIXTURE_SHA256").unwrap()).unwrap(),
        descriptor: MediaStream::Video {
            available: span,
            dimensions: Canvas {
                width: 128,
                height: 96,
            },
            frame_rate: None,
        },
    };
    let project = VideoProject::create(
        CreateProject {
            document_id: DocumentId::new("registry-media-consumer").unwrap(),
            title: "原始素材".into(),
            root_sequence: SequenceId::new("main").unwrap(),
            canvas: Canvas {
                width: 128,
                height: 96,
            },
            frame_rate: FrameRate {
                numerator: 25,
                denominator: 1,
            },
            audio_sample_rate: 48000,
        },
        Limits::default(),
        &|| false,
    )
    .unwrap();
    let prepared = project
        .prepare(
            &Transaction {
                contract: EditContract::V1,
                expected_digest: project.digest().clone(),
                operations: vec![
                    Operation::AddResource {
                        resource: Resource {
                            id: binding.resource.clone(),
                            digest: binding.digest.clone(),
                            byte_length: ByteLength::new(length),
                            descriptor: ResourceDescriptor::Media {
                                streams: BTreeMap::from([(
                                    binding.stream.clone(),
                                    binding.descriptor.clone(),
                                )]),
                            },
                        },
                    },
                    Operation::AddTrack {
                        sequence: SequenceId::new("main").unwrap(),
                        track: track("picture", TrackKind::Video),
                        before: None,
                    },
                    Operation::Insert {
                        track: TrackId::new("picture").unwrap(),
                        clip: clip(
                            "source",
                            ClipSource::Media {
                                resource: binding.resource.clone(),
                                stream: binding.stream.clone(),
                                time_map: TimeMap {
                                    anchors: vec![
                                        TimeAnchor {
                                            local: span.start,
                                            source: span.start,
                                        },
                                        TimeAnchor {
                                            local: span.end,
                                            source: span.end,
                                        },
                                    ],
                                },
                            },
                            span,
                        ),
                        ripple_tracks: vec![],
                    },
                ],
            },
            &|| false,
        )
        .unwrap();
    let original = VideoProject::open(prepared.snapshot, Limits::default(), &|| false).unwrap();
    let decoder = source::native::NativeVideoDecoder::open(
        &bundle,
        input(&fixture),
        length,
        options(),
        stream.index,
        &|| false,
    )
    .unwrap();
    let mut visual = Visual {
        sampler: VideoSampler::new(decoder, binding.clone(), span, SourceLimits::default())
            .unwrap(),
        stream,
    };
    let before = rgba(&original, &mut visual, 0);
    let edited = original
        .prepare(
            &Transaction {
                contract: EditContract::V1,
                expected_digest: original.digest().clone(),
                operations: vec![
                    Operation::AddTrack {
                        sequence: SequenceId::new("main").unwrap(),
                        track: track("overlay", TrackKind::Graphics),
                        before: None,
                    },
                    Operation::Insert {
                        track: TrackId::new("overlay").unwrap(),
                        clip: clip(
                            "tint",
                            ClipSource::Solid {
                                rgba: [255, 0, 0, 64],
                            },
                            span,
                        ),
                        ripple_tracks: vec![],
                    },
                ],
            },
            &|| false,
        )
        .unwrap();
    let project = VideoProject::open(edited.snapshot, Limits::default(), &|| false).unwrap();
    let after = rgba(&project, &mut visual, 0);
    assert_ne!(before, after);
    // Random forward/backward navigation uses the same exact source sampler.
    let future = rgba(&project, &mut visual, 52);
    assert_ne!(future, after);
    assert_eq!(rgba(&project, &mut visual, 0), after);
    fs::write(output.join("preview-before.rgba"), before).unwrap();
    fs::write(output.join("preview-after.rgba"), after).unwrap();
    let plan = ExportPlan::new(
        project.compiled(),
        ExportOptions {
            profile: EncodeProfile::Ffv1PcmMp4,
            video_bit_rate: 2_000_000,
            gop_frames: 25,
            audio: ExportAudio::Omit,
            background_srgb: [0; 3],
            tone_map: None,
            sampling: Sampling::Bilinear,
        },
        ExportLimits::default(),
        &|| false,
    )
    .unwrap();
    let config = plan.configuration();
    let encoder = NativeEncoder::open(
        &bundle,
        Output(File::create(output.join("project.mp4")).unwrap()),
        config,
        EncoderOptions {
            encode: EncodeLimits::default(),
            process: options().process,
            open_timeout: Duration::from_secs(15),
            call_timeout: Duration::from_secs(15),
        },
        || false,
    )
    .unwrap();
    let mut export = ExportSession::new(
        plan,
        RecordedEncoder {
            inner: encoder,
            expected: File::create(output.join("expected.yuv")).unwrap(),
        },
    )
    .unwrap();
    let candidate = loop {
        match export.step(&mut visual, &mut NoAudio, &|| false).unwrap() {
            ExportStep::Advanced(_) => (),
            ExportStep::WaitingForAudio(_) => panic!("silent graph must not read audio"),
            ExportStep::ReadyToVerify(candidate) => break candidate,
        }
    };
    assert_eq!(candidate.encoded.video_frames, 75);
    assert_eq!(&candidate.document_digest, project.digest());
    assert_eq!(
        candidate.encoded.byte_length,
        fs::metadata(output.join("project.mp4")).unwrap().len()
    );
    drop(export);
    assert!(visual.sampler.retained_bytes() <= SourceLimits::default().max_retained_bytes);
    // Immutable project and pixels are independent of the export lifecycle.
    assert_eq!(project.summary(&|| false).unwrap().frame_count.get(), 75);
    println!(
        "{{\"profile\":\"{}\",\"documentDigest\":\"{}\",\"frames\":75,\"sourceAudioSamples\":{},\"encodedBytes\":{},\"previewBytes\":49152,\"width\":128,\"height\":96,\"retainedSourceBytes\":{},\"outputAudio\":false}}",
        mo_video_runtime::PROFILE,
        project.digest().as_str(),
        samples,
        candidate.encoded.byte_length,
        visual.sampler.retained_bytes()
    );
}
