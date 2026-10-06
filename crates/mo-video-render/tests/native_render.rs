//! Executed explicitly by video-render-check.py with this run's actual worker bundle.
mod support;
use mo_common::{ByteLength, Digest as ContentDigest, DocumentId, RationalTime, ResourceId};
use mo_media::{DecodeLimits, ReadAt, StreamInfo};
use mo_native_limits::ProcessLimits;
use mo_native_media::{NativeContainer, WorkerBundle, WorkerOptions};
use mo_video_compile::{
    CompileLimits, CompiledDocument, MediaBinding, QueryLimits, VisualEffect, VisualSource,
};
use mo_video_model::*;
use mo_video_render::*;
use mo_video_source::{
    SampleMode, SourceLimits, SourceSelection, VideoDecoder, VideoSampler,
    native::NativeVideoDecoder,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use support::{node, output, plan, t};
fn input(path: &Path) -> (impl ReadAt + 'static, u64) {
    let mut file = File::open(path).unwrap();
    let n = file.metadata().unwrap().len();
    (
        move |off, buf: &mut [u8]| {
            file.seek(SeekFrom::Start(off))?;
            file.read(buf)
        },
        n,
    )
}
fn options() -> WorkerOptions {
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
fn span() -> TimeRange {
    TimeRange {
        start: t(0),
        end: RationalTime::new(90090, 30000).unwrap(),
    }
}
fn document(binding: &MediaBinding, length: u64) -> CompiledDocument {
    let root = SequenceId::new("main").unwrap();
    let track = TrackId::new("v1").unwrap();
    let id = ClipId::new("movie").unwrap();
    let mut doc = VideoDocument::empty(
        DocumentId::new("composed-film").unwrap(),
        root.clone(),
        Canvas {
            width: 128,
            height: 96,
        },
        FrameRate {
            numerator: 30000,
            denominator: 1001,
        },
    );
    doc.resources.insert(
        binding.resource.clone(),
        Resource {
            id: binding.resource.clone(),
            digest: binding.digest.clone(),
            byte_length: ByteLength::new(length),
            descriptor: ResourceDescriptor::Media {
                streams: BTreeMap::from([(binding.stream.clone(), binding.descriptor.clone())]),
            },
        },
    );
    doc.sequences
        .get_mut(&root)
        .unwrap()
        .tracks
        .push(track.clone());
    doc.tracks.insert(
        track.clone(),
        Track {
            id: track,
            name: "Video".into(),
            kind: TrackKind::Video,
            locked: false,
            muted: false,
            solo: false,
            clips: vec![id.clone()],
        },
    );
    doc.clips.insert(
        id.clone(),
        Clip {
            id,
            name: "Fixture".into(),
            timeline: span(),
            source: ClipSource::Media {
                resource: binding.resource.clone(),
                stream: binding.stream.clone(),
                time_map: TimeMap {
                    anchors: vec![
                        TimeAnchor {
                            local: t(0),
                            source: t(0),
                        },
                        TimeAnchor {
                            local: span().end,
                            source: span().end,
                        },
                    ],
                },
            },
            effects: vec![],
            audio: AudioTimePolicy::default(),
        },
    );
    CompiledDocument::new(doc, CompileLimits::default(), &|| false).unwrap()
}
struct Resources {
    sampler: VideoSampler<NativeVideoDecoder<'static>>,
    stream: StreamInfo,
}
impl VisualResources for Resources {
    fn media(
        &mut self,
        binding: &MediaBinding,
        sample: &TimeSample,
        check: &dyn Fn() -> bool,
    ) -> Result<ResolvedMedia> {
        let s = self
            .sampler
            .sample(binding, sample, SampleMode::Continue, check)
            .map_err(|e| RenderError::Resource(e.to_string()))?;
        let SourceSelection::Frame(frame) = s.selection else {
            return Err(RenderError::Resource("missing source frame".into()));
        };
        let c = InputColor::from_codec(&frame.frame.info, false, None)?;
        let geometry = SourceGeometry::from_media(&frame.frame.info, &self.stream)?;
        let image =
            Arc::new(normalize_frame(&frame.frame, c, PixelLimits::default(), check)?.image);
        Ok(ResolvedMedia {
            binding: binding.clone(),
            interval: Some(frame.interval),
            image,
            geometry,
        })
    }
}
#[test]
#[ignore = "needs generated actual media and pinned worker; runner verifies independent colors"]
fn actual_frames_are_normalized_and_compiled_plan_is_composited() {
    let fixture = PathBuf::from(std::env::var_os("MO_MEDIA_TEST_FIXTURE").expect("fixture"));
    let output_dir =
        PathBuf::from(std::env::var_os("MO_VIDEO_RENDER_TEST_OUTPUT").expect("output"));
    fs::create_dir(&output_dir).unwrap();
    let bundle = WorkerBundle::open(
        PathBuf::from(std::env::var_os("MO_MEDIA_TEST_BUNDLE").expect("bundle")),
        std::env::var("MO_MEDIA_TEST_BUNDLE_SHA256").expect("pin"),
    )
    .unwrap();
    let (source, length) = input(&fixture);
    let mut raw = NativeContainer::open(&bundle, source, length, options(), &|| false)
        .unwrap()
        .select(0, &|| false)
        .unwrap();
    let mut pixels = File::create(output_dir.join("linear-rgb.f32")).unwrap();
    let mut source_yuv = File::create(output_dir.join("source.yuv")).unwrap();
    let mut frames = 0;
    while let Some(frame) = raw.next_frame(&|| false).unwrap() {
        assert_eq!(frame.info.format, "yuv420p");
        assert_eq!(frame.info.colors.chroma_location, 1);
        for p in &frame.planes {
            source_yuv.write_all(&p.bytes).unwrap();
        }
        let color = InputColor::from_codec(&frame.info, false, None).unwrap();
        let linear = normalize_frame(&frame, color, PixelLimits::default(), &|| false).unwrap();
        for pixel in linear.image.pixels() {
            for v in &pixel[..3] {
                pixels.write_all(&v.to_le_bytes()).unwrap();
            }
        }
        frames += 1;
    }
    raw.close(&|| false).unwrap();
    assert_eq!(frames, 77);
    let binding = MediaBinding {
        resource: ResourceId::new("fixture").unwrap(),
        digest: ContentDigest::from_sha256(Sha256::digest(fs::read(&fixture).unwrap()).into()),
        stream: StreamId::new("video0").unwrap(),
        descriptor: MediaStream::Video {
            available: span(),
            dimensions: Canvas {
                width: 128,
                height: 96,
            },
            frame_rate: None,
        },
    };
    let (source, _) = input(&fixture);
    let decoder =
        NativeVideoDecoder::open(&bundle, source, length, options(), 0, &|| false).unwrap();
    let stream = decoder.stream().clone();
    let mut resources = Resources {
        sampler: VideoSampler::new(
            decoder,
            binding.clone(),
            span(),
            SourceLimits {
                max_frame_bytes: 18432,
                max_retained_bytes: 18432 * 12,
                max_retained_frames: 12,
                ..Default::default()
            },
        )
        .unwrap(),
        stream,
    };
    let doc = document(&binding, length);
    let mut rendered = Vec::new();
    for index in [0, 17, 40, 89] {
        let computed = doc
            .frame(index, QueryLimits::default(), &|| false)
            .unwrap()
            .value;
        let image = render_frame(
            &computed,
            &mut resources,
            Sampling::Bilinear,
            RenderLimits::default(),
            &|| false,
        )
        .unwrap();
        let mut options = output();
        options.alpha = OutputAlpha::PremultipliedEncoded;
        let bytes = image.image.to_srgb8(options, &|| false).unwrap();
        let name = format!("frame-{index}.png");
        let mut file = File::create(output_dir.join(&name)).unwrap();
        mo_image::png::encode_to(
            &bytes.rgba,
            bytes.width,
            bytes.height,
            &mut file,
            1 << 20,
            &|| false,
        )
        .unwrap();
        rendered.push(json!({"file":name,"index":index,"sha256":format!("{:x}",Sha256::digest(&bytes.rgba)),"pixelVisits":image.stats.pixel_visits,"temporaryBytes":image.stats.peak_temporary_bytes}));
        if index == 17 {
            let mut picture = computed.layers[0].clone();
            picture.effects.push(VisualEffect::Transform {
                x_milli_pixels: 0,
                y_milli_pixels: 0,
                scale_x_ppm: 750000,
                scale_y_ppm: 750000,
                rotation_milli_degrees: 15000,
                opacity_ppm: 850000,
            });
            let graph = plan(
                vec![
                    node(
                        "background",
                        VisualSource::Solid {
                            rgba: [30, 30, 38, 255],
                        },
                    ),
                    picture,
                ],
                128,
                96,
            );
            let composed = render_frame(
                &graph,
                &mut resources,
                Sampling::Bilinear,
                RenderLimits::default(),
                &|| false,
            )
            .unwrap()
            .image
            .to_srgb8(options, &|| false)
            .unwrap();
            mo_image::png::encode_to(
                &composed.rgba,
                128,
                96,
                &mut File::create(output_dir.join("composed.png")).unwrap(),
                1 << 20,
                &|| false,
            )
            .unwrap();
        }
    }
    fs::write(
        output_dir.join("frames.json"),
        serde_json::to_vec_pretty(&json!({"normalizedFrames":frames,"renders":rendered})).unwrap(),
    )
    .unwrap();
}
