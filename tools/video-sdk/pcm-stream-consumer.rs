// Independent registry consumer: no repository source paths or extra crates.
use mo_video_runtime::pcm::Result;
use mo_video_runtime::{
    audio::*, audio_movie::*, authoring::model::*, common::*, isobmff::timing::TimingLimits,
    media::*, native::*, pcm::*, process_limits::*, time::ExactRatio,
};
use std::{collections::BTreeMap, sync::Arc};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
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
fn reader(path: &Path) -> impl ReadAt + 'static {
    let mut file = File::open(path).unwrap();
    move |offset, bytes: &mut [u8]| {
        file.seek(SeekFrom::Start(offset))?;
        file.read(bytes)
    }
}
fn decoded(
    bundle: &WorkerBundle,
    path: &Path,
    mode: DecodeTimeline,
) -> (InspectedPcm, Vec<f32>, u32) {
    let container = NativeContainer::open_with_timeline(
        bundle,
        reader(path),
        path.metadata().unwrap().len(),
        options(),
        mode,
        &|| false,
    )
    .unwrap();
    let stream = container
        .streams()
        .iter()
        .find(|s| s.kind == MediaKind::Audio)
        .unwrap()
        .clone();
    let mut decoder = container.select(stream.index, &|| false).unwrap();
    let mut inspection = PcmInspection::new(1, PcmInspectLimits::default()).unwrap();
    let mut samples = vec![];
    while let Some(frame) = decoder.next_frame(&|| false).unwrap() {
        let order = if cfg!(target_endian = "little") {
            ByteOrder::Little
        } else {
            ByteOrder::Big
        };
        let pcm = inspection.observe_pcm(&frame, order, &|| false).unwrap();
        samples.extend_from_slice(pcm.samples());
    }
    decoder.close(&|| false).unwrap();
    (
        inspection.finish_at_eof(&|| false).unwrap(),
        samples,
        stream.container_track_id.unwrap(),
    )
}
struct Memory {
    tiles: BTreeMap<u32, PcmTile>,
}
impl PcmTileStore for Memory {
    fn lookup(
        &mut self,
        source: &Digest,
        index: u32,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Option<PcmTileRef>> {
        assert!(!cancelled());
        Ok(self
            .tiles
            .get(&index)
            .filter(|t| &t.reference.source == source)
            .map(|t| t.reference.clone()))
    }
    fn read(
        &mut self,
        r: &PcmTileRef,
        max: usize,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Vec<u8>> {
        assert!(!cancelled());
        let tile = &self.tiles[&r.index];
        assert_eq!(&tile.reference, r);
        assert!(tile.bytes.len() <= max);
        Ok(tile.bytes.clone())
    }
}
fn streamed(
    bundle: &WorkerBundle,
    path: &Path,
    content: Digest,
) -> (InspectedPcm, CanonicalAudioResource<Memory>, u32) {
    let container = NativeContainer::open_with_timeline(
        bundle,
        reader(path),
        path.metadata().unwrap().len(),
        options(),
        DecodeTimeline::RawIsoBmff,
        &|| false,
    )
    .unwrap();
    let stream = container
        .streams()
        .iter()
        .find(|s| s.kind == MediaKind::Audio)
        .unwrap()
        .clone();
    let id = stream.container_track_id.unwrap();
    let origin = PcmStreamOrigin {
        resource: ResourceId::new("source").unwrap(),
        content,
        stream: StreamId::new(format!("track-{id}")).unwrap(),
        decoder_profile: mo_video_runtime::common::digest(
            "musteroffice.stream-consumer/1",
            &(bundle.manifest_sha256(), "raw-isobmff"),
        )
        .unwrap(),
    };
    let limits = PcmStreamLimits::default();
    let mut builder = PcmStreamBuilder::new(origin.clone(), 1, limits).unwrap();
    let mut decoder = container.select(stream.index, &|| false).unwrap();
    let mut pending = vec![];
    while let Some(frame) = decoder.next_frame(&|| false).unwrap() {
        pending.extend(
            builder
                .push(
                    &frame,
                    if cfg!(target_endian = "little") {
                        ByteOrder::Little
                    } else {
                        ByteOrder::Big
                    },
                    &|| false,
                )
                .unwrap(),
        );
        assert!(builder.retained_bytes() <= 4096 * 4);
    }
    decoder.close(&|| false).unwrap();
    let mut complete = builder.finish_at_eof(&|| false).unwrap();
    if let Some(tail) = complete.tail.take() {
        pending.push(tail);
    }
    let tiles = pending
        .into_iter()
        .map(|tile| {
            let reference = complete.seal_ref(&tile.reference).unwrap();
            (
                reference.index,
                PcmTile {
                    reference,
                    bytes: tile.bytes,
                },
            )
        })
        .collect();
    let encoded =
        mo_video_runtime::authoring::encode(&complete.record().unwrap(), 8192, &|| false).unwrap();
    drop(complete);
    let record: PcmStreamRecord =
        mo_video_runtime::authoring::decode(&encoded, 8192, &|| false).unwrap();
    let (raw, source) = record.restore(&origin, limits).unwrap();
    let canonical = CanonicalAudioResource::new(
        Arc::clone(&source),
        Memory { tiles },
        PcmReadLimits {
            max_cached_tiles: 2,
            ..Default::default()
        },
    )
    .unwrap();
    (raw, canonical, id)
}
fn main() {
    let bundle = WorkerBundle::open(
        std::env::var_os("MO_CONSUMER_WORKER").unwrap().into(),
        std::env::var("MO_CONSUMER_WORKER_SHA256").unwrap(),
    )
    .unwrap();
    let path = std::path::PathBuf::from(std::env::var_os("MO_CONSUMER_FIXTURE").unwrap());
    let digest = Digest::try_from(std::env::var("MO_CONSUMER_FIXTURE_SHA256").unwrap()).unwrap();
    let (raw, canonical, id) = streamed(&bundle, &path, digest);
    let raw_binding = canonical.source().binding().unwrap().clone();
    let canonical_tiles = canonical.source().tile_count();
    let (reference, expected, reference_id) = decoded(&bundle, &path, DecodeTimeline::Demuxer);
    assert_eq!(id, reference_id);
    let o = options();
    let mut index = NativeIndex::open(
        &bundle,
        reader(&path),
        path.metadata().unwrap().len(),
        NativeIndexOptions {
            index: IndexLimits::default(),
            max_read_bytes: o.decode.max_read_bytes,
            process: o.process,
            open_timeout: o.open_timeout,
            call_timeout: o.call_timeout,
        },
        &|| false,
    )
    .unwrap();
    let mut ordinal = None;
    for i in 0..index.summary().tracks {
        if let IndexPage::Track { track } = index
            .query(IndexQuery::Track { track: i }, &|| false)
            .unwrap()
            && track.id == id
        {
            assert_eq!(track.handler, *b"soun");
            ordinal = Some(i);
        }
    }
    let timeline = index
        .movie_timeline(
            ordinal.unwrap(),
            &ExactRatio::from_time(raw.decoded_range(256).unwrap().end),
            TimingLimits::default(),
            &|| false,
        )
        .unwrap();
    assert_eq!(timeline.track_id(), id);
    index.close(&|| false).unwrap();
    let plan = MovieAudioPlan::new(
        &timeline,
        raw_binding.clone(),
        &raw,
        MovieAudioLimits::default(),
        &|| false,
    )
    .unwrap();
    let mut movie_binding = raw_binding.clone();
    movie_binding.descriptor = plan.descriptor().clone();
    let asset = plan.asset().clone();
    let mut source = MovieAudioResource::new(plan, movie_binding.clone(), canonical).unwrap();
    assert_eq!(asset.available.end, reference.asset().available.end);
    let channels = asset.layout.len();
    for first in (0..asset.available.end as usize).step_by(997) {
        let end = (first + 997).min(asset.available.end as usize);
        let out = source
            .read(
                &movie_binding,
                &asset,
                PcmRange::new(first as i64, end as i64).unwrap(),
                PcmLimits::default(),
                &|| false,
            )
            .unwrap();
        assert_eq!(out.samples().len(), (end - first) * channels);
        assert!(
            out.samples()
                .iter()
                .zip(&expected[first * channels..end * channels])
                .all(|(a, b)| a.to_bits() == b.to_bits())
        );
    }
    for i in (0..32).rev() {
        let first = (i * 7919) % asset.available.end;
        let end = (first + 719).min(asset.available.end);
        let out = source
            .read(
                &movie_binding,
                &asset,
                PcmRange::new(first, end).unwrap(),
                PcmLimits::default(),
                &|| false,
            )
            .unwrap();
        assert_eq!(out.samples().len(), (end - first) as usize * channels);
        assert!(
            out.samples()
                .iter()
                .zip(&expected[first as usize * channels..end as usize * channels])
                .all(|(a, b)| a.to_bits() == b.to_bits())
        );
    }
    println!(
        "{{\"rawSamples\":{},\"movieSamples\":{},\"randomQueries\":32,\"equalPcmBits\":true,\"streamedCanonicalTiles\":{},\"completionRecordRestored\":true,\"mappingProfile\":\"{}\"}}",
        raw.asset().available.end,
        asset.available.end,
        canonical_tiles,
        mo_video_runtime::audio_movie::PROFILE
    );
}
