//! Host-side filesystem fixture only. The public PCM crate has no filesystem,
//! storage policy, task persistence or native codec dependency.
use mo_common::{ByteLength, Digest as ContentDigest, DocumentId, RationalTime, ResourceId};
use mo_media::{DecodeLimits, MediaKind};
use mo_native_limits::ProcessLimits;
use mo_native_media::{NativeContainer, NativeDecoder, WorkerBundle, WorkerOptions};
use mo_time::ExactRatio as R;
use mo_video_audio::{
    AudioAsset, AudioMixer, ByteOrder, ChannelLayout, MixLimits, PcmBlock, PcmLimits, PcmRange,
};
use mo_video_compile::{CompileLimits, CompiledDocument, MediaBinding};
use mo_video_model::*;
use mo_video_pcm::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
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
fn decoder(bundle: &WorkerBundle, path: &Path) -> NativeDecoder<'static> {
    let mut file = File::open(path).unwrap();
    let length = file.metadata().unwrap().len();
    let container = NativeContainer::open(
        bundle,
        move |offset, out: &mut [u8]| {
            file.seek(SeekFrom::Start(offset))?;
            file.read(out)
        },
        length,
        options(),
        &|| false,
    )
    .unwrap();
    let index = container
        .streams()
        .iter()
        .find(|s| s.kind == MediaKind::Audio)
        .unwrap()
        .index;
    container.select(index, &|| false).unwrap()
}
struct Disk {
    root: PathBuf,
    identity: ContentDigest,
}
impl Disk {
    fn publish(&self, tile: &PcmTile) {
        assert_eq!(tile.reference.source, self.identity);
        let blob = self.root.join(tile.reference.sha256.as_str());
        if !blob.exists() {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&blob)
                .unwrap();
            file.write_all(&tile.bytes).unwrap();
            file.sync_all().unwrap();
        }
        let temporary = self.root.join(format!("{}.pending", tile.reference.index));
        let final_path = self.root.join(format!("{}.json", tile.reference.index));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .unwrap();
        file.write_all(&serde_json::to_vec(&tile.reference).unwrap())
            .unwrap();
        file.sync_all().unwrap();
        drop(file);
        assert!(!final_path.exists());
        fs::rename(temporary, final_path).unwrap();
    }
}
impl PcmTileStore for Disk {
    fn lookup(
        &mut self,
        source: &ContentDigest,
        index: u32,
        _: &dyn Fn() -> bool,
    ) -> Result<Option<PcmTileRef>> {
        assert_eq!(source, &self.identity);
        let path = self.root.join(format!("{index}.json"));
        if !path.exists() {
            return Ok(None);
        }
        let data = fs::read(path).unwrap();
        assert!(data.len() < 4096);
        Ok(Some(serde_json::from_slice(&data).unwrap()))
    }
    fn read(
        &mut self,
        reference: &PcmTileRef,
        max_bytes: usize,
        _: &dyn Fn() -> bool,
    ) -> Result<Vec<u8>> {
        let file = File::open(self.root.join(reference.sha256.as_str())).unwrap();
        assert_eq!(file.metadata().unwrap().len(), max_bytes as u64);
        let mut data = Vec::new();
        file.take(max_bytes as u64 + 1)
            .read_to_end(&mut data)
            .unwrap();
        Ok(data)
    }
}
fn source(path: &Path, profile: &ContentDigest) -> Arc<PcmSource> {
    let binding = MediaBinding {
        resource: ResourceId::new("actual-audio").unwrap(),
        digest: ContentDigest::from_sha256(Sha256::digest(fs::read(path).unwrap()).into()),
        stream: StreamId::new("audio0").unwrap(),
        descriptor: MediaStream::Audio {
            available: TimeRange {
                start: t(0),
                end: t(3),
            },
            sample_rate: 48000,
            channels: 1,
        },
    };
    Arc::new(
        PcmSource::new(
            binding,
            AudioAsset {
                sample_rate: 48000,
                layout: ChannelLayout::mono(),
                sample_zero: R::integer(0),
                available: PcmRange::new(0, 144000).unwrap(),
            },
            profile,
            4096,
            256,
        )
        .unwrap(),
    )
}
fn t(n: i64) -> RationalTime {
    RationalTime::new(n, 1).unwrap()
}
fn reader(source: Arc<PcmSource>) -> PcmTileReader {
    PcmTileReader::new(
        source,
        PcmReadLimits {
            max_cached_bytes: 32768,
            max_cached_tiles: 2,
            max_working_bytes: 131072,
            ..PcmReadLimits::default()
        },
    )
    .unwrap()
}
fn bytes(pcm: &PcmBlock) -> Vec<u8> {
    pcm.samples().iter().flat_map(|v| v.to_le_bytes()).collect()
}
fn project(source: &PcmSource, length: u64) -> CompiledDocument {
    let root = SequenceId::new("main").unwrap();
    let track = TrackId::new("audio").unwrap();
    let clip = ClipId::new("reversed").unwrap();
    let binding = source.binding().unwrap();
    let mut d = VideoDocument::empty(
        DocumentId::new("canonical-audio-project").unwrap(),
        root.clone(),
        Canvas {
            width: 100,
            height: 100,
        },
        FrameRate {
            numerator: 25,
            denominator: 1,
        },
    );
    d.resources.insert(
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
    d.sequences
        .get_mut(&root)
        .unwrap()
        .tracks
        .push(track.clone());
    d.tracks.insert(
        track.clone(),
        Track {
            id: track,
            name: "Sound".into(),
            kind: TrackKind::Audio,
            locked: false,
            muted: false,
            solo: false,
            clips: vec![clip.clone()],
        },
    );
    d.clips.insert(
        clip.clone(),
        Clip {
            audio_context: None,
            id: clip,
            name: "Reverse".into(),
            timeline: TimeRange {
                start: t(0),
                end: t(3),
            },
            source: ClipSource::Media {
                resource: binding.resource.clone(),
                stream: binding.stream.clone(),
                time_map: TimeMap {
                    anchors: vec![
                        TimeAnchor {
                            local: t(0),
                            source: t(3),
                        },
                        TimeAnchor {
                            local: t(3),
                            source: t(0),
                        },
                    ],
                },
            },
            effects: vec![],
            audio: AudioTimePolicy {
                rate: AudioRatePolicy::Resample,
                reverse: ReverseAudioPolicy::Reverse,
                hold: HoldAudioPolicy::Silence,
            },
        },
    );
    CompiledDocument::new(d, CompileLimits::default(), &|| false).unwrap()
}
#[test]
#[ignore = "requires pinned worker, generated AAC fixtures and fresh output"]
fn actual_canonical_tiles_random_read_and_mixer() {
    let output = PathBuf::from(std::env::var_os("MO_PCM_OUTPUT").expect("fresh output"));
    fs::create_dir(&output).unwrap();
    let bundle_path = PathBuf::from(std::env::var_os("MO_MEDIA_TEST_BUNDLE").expect("bundle"));
    let pin = std::env::var("MO_MEDIA_TEST_BUNDLE_SHA256").expect("pin");
    let bundle = WorkerBundle::open(bundle_path, pin.clone()).unwrap();
    let profile = ContentDigest::try_from(pin).unwrap();
    let mut reports = vec![];
    for (name, env) in [("tone", "MO_PCM_TONE"), ("noise", "MO_PCM_NOISE")] {
        let path = PathBuf::from(std::env::var_os(env).expect("fixture"));
        let source = source(&path, &profile);
        let dir = output.join(name);
        fs::create_dir(&dir).unwrap();
        let root = dir.join("tiles");
        fs::create_dir(&root).unwrap();
        let disk = Disk {
            root: root.clone(),
            identity: source.identity().clone(),
        };
        let mut decode = decoder(&bundle, &path);
        let mut builder = PcmTileBuilder::new(Arc::clone(&source));
        let mut linear = Sha256::new();
        let mut normalized = File::create(dir.join("continuous.f32")).unwrap();
        let mut max_carry = 0;
        let mut frames = 0;
        while let Some(frame) = decode.next_frame(&|| false).unwrap() {
            let pcm = source
                .normalize(
                    &frame,
                    if cfg!(target_endian = "little") {
                        ByteOrder::Little
                    } else {
                        ByteOrder::Big
                    },
                    1,
                    PcmLimits::default(),
                    &|| false,
                )
                .unwrap();
            let data = bytes(&pcm);
            linear.update(&data);
            normalized.write_all(&data).unwrap();
            frames += pcm.frames();
            let candidate = builder
                .prepare(&pcm, PcmBuildLimits::default(), &|| false)
                .unwrap();
            for tile in &candidate.tiles {
                disk.publish(tile);
            }
            builder = candidate.next;
            max_carry = max_carry.max(builder.retained_bytes());
        }
        assert_eq!(frames, 144000);
        builder.verify_eof().unwrap();
        assert_eq!(builder.completed_tiles(), 36);
        decode.close(&|| false).unwrap();
        // Reopen the storage index, retaining neither the decoder nor prior PCM.
        let mut disk = Disk {
            root: root.clone(),
            identity: source.identity().clone(),
        };
        let mut random = reader(Arc::clone(&source));
        let mut queries = vec![];
        let mut result = File::create(dir.join("queries.f32")).unwrap();
        let mut peak = 0;
        let windows: Vec<_> = (0..64)
            .map(|i| ((i * 7919) % 139000, (i * 53) % 4096 + 1))
            .chain(
                (0..64)
                    .rev()
                    .map(|i| ((i * 7919) % 139000, (i * 53) % 4096 + 1)),
            )
            .chain([(0, 1), (143999, 1), (4095, 2), (143000, 1000)])
            .collect();
        for (first, n) in windows {
            let read = random
                .read(
                    source.binding().unwrap(),
                    PcmRange::new(first, first + n).unwrap(),
                    &mut disk,
                    &|| false,
                )
                .unwrap();
            let data = bytes(&read.pcm);
            result.write_all(&data).unwrap();
            peak = peak.max(read.stats.peak_working_bytes);
            queries.push(json!({"first":first,"frames":n,"sha256":format!("{:x}",Sha256::digest(&data)),"tileReads":read.stats.tile_reads,"cacheHits":read.stats.cache_hits}));
        }
        let mut warm = decoder(&bundle, &path);
        for _ in 0..50 {
            warm.next_frame(&|| false).unwrap().unwrap();
        }
        warm.seek(0, &|| false).unwrap();
        let mut seek_errors = Vec::new();
        for frame_index in 0..=20 {
            let after_seek = warm.next_frame(&|| false).unwrap().unwrap();
            if frame_index != 0 && frame_index != 20 {
                continue;
            }
            let changed = source
                .normalize(
                    &after_seek,
                    if cfg!(target_endian = "little") {
                        ByteOrder::Little
                    } else {
                        ByteOrder::Big
                    },
                    2,
                    PcmLimits::default(),
                    &|| false,
                )
                .unwrap();
            let start = changed.first_frame().max(0);
            let end = changed.end_frame().min(144000);
            assert!(end > start);
            let canonical = random
                .read(
                    source.binding().unwrap(),
                    PcmRange::new(start, end).unwrap(),
                    &mut disk,
                    &|| false,
                )
                .unwrap();
            let offset = (start - changed.first_frame()) as usize;
            let error = canonical
                .pcm
                .samples()
                .iter()
                .zip(&changed.samples()[offset..offset + canonical.pcm.frames()])
                .map(|(a, b)| (*a - *b).abs())
                .fold(0f32, f32::max);
            seek_errors.push(error);
        }
        warm.close(&|| false).unwrap();
        let compiled = project(&source, fs::metadata(&path).unwrap().len());
        let mut mixer =
            AudioMixer::new(&compiled, ChannelLayout::mono(), MixLimits::default()).unwrap();
        let mut resources = CanonicalAudioResource::new(
            Arc::clone(&source),
            disk,
            PcmReadLimits {
                max_cached_bytes: 32768,
                max_cached_tiles: 2,
                max_working_bytes: 131072,
                ..PcmReadLimits::default()
            },
        )
        .unwrap();
        let mut reversed = File::create(dir.join("mixed-reverse.f32")).unwrap();
        let mut first = 0;
        while first < 144000 {
            let n = (144000 - first).min(997) as u32;
            let block = mixer.render(&mut resources, first, n, &|| false).unwrap();
            reversed.write_all(&bytes(&block.pcm)).unwrap();
            first += i64::from(n);
        }
        reports.push(json!({"name":name,"fixtureSha256":source.binding().unwrap().digest,"sourceIdentity":source.identity(),"continuousSha256":format!("{:x}",linear.finalize()),"frames":frames,"tiles":builder.completed_tiles(),"maxBuilderCarryBytes":max_carry,"maxReaderWorkingBytes":peak,"seekZeroAfterWarmupMaximumDifference":seek_errors[0],"afterTwentyFramesMaximumDifference":seek_errors[1],"queries":queries}));
    }
    fs::write(
        output.join("report.json"),
        serde_json::to_vec_pretty(&json!({"profile":PROFILE,"cases":reports})).unwrap(),
    )
    .unwrap();
}
