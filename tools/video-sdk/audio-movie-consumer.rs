// Independent registry consumer: no repository source paths or extra crates.
use mo_video_runtime::audio::Result;
use mo_video_runtime::{
    audio::*, audio_movie::*, authoring::model::*, common::*, compile::MediaBinding,
    isobmff::timing::TimingLimits, media::*, native::*, pcm::*, process_limits::*,
    time::ExactRatio,
};
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
        inspection.observe(&frame, order, &|| false).unwrap();
        let layout = ChannelLayout::from_codec_name(&frame.info.channel_layout).unwrap();
        let first = (samples.len() / layout.len()) as i64;
        let pcm = normalize_pcm(&frame, order, layout, first, PcmLimits::default(), &|| {
            false
        })
        .unwrap();
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
    binding: MediaBinding,
    asset: AudioAsset,
    samples: Vec<f32>,
}
impl AudioResources for Memory {
    fn describe(&mut self, binding: &MediaBinding) -> Result<AudioAsset> {
        assert_eq!(binding, &self.binding);
        Ok(self.asset.clone())
    }
    fn read(
        &mut self,
        binding: &MediaBinding,
        asset: &AudioAsset,
        range: PcmRange,
        limits: PcmLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<PcmBlock> {
        assert_eq!(binding, &self.binding);
        assert_eq!(asset, &self.asset);
        let channels = asset.layout.len();
        PcmBlock::new(
            asset.sample_rate,
            asset.layout.clone(),
            range.first,
            self.samples[range.first as usize * channels..range.end as usize * channels].to_vec(),
            limits,
            check,
        )
    }
}
fn main() {
    let bundle = WorkerBundle::open(
        std::env::var_os("MO_CONSUMER_WORKER").unwrap().into(),
        std::env::var("MO_CONSUMER_WORKER_SHA256").unwrap(),
    )
    .unwrap();
    let path = std::path::PathBuf::from(std::env::var_os("MO_CONSUMER_FIXTURE").unwrap());
    let digest = Digest::try_from(std::env::var("MO_CONSUMER_FIXTURE_SHA256").unwrap()).unwrap();
    let (raw, samples, id) = decoded(&bundle, &path, DecodeTimeline::RawIsoBmff);
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
    let raw_binding = MediaBinding {
        resource: ResourceId::new("source").unwrap(),
        digest,
        stream: StreamId::new(format!("track-{id}")).unwrap(),
        descriptor: MediaStream::Audio {
            available: raw.decoded_range(256).unwrap(),
            sample_rate: raw.asset().sample_rate,
            channels: raw.asset().layout.len() as u16,
        },
    };
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
    let mut source = MovieAudioResource::new(
        plan,
        movie_binding.clone(),
        Memory {
            binding: raw_binding,
            asset: raw.asset().clone(),
            samples,
        },
    )
    .unwrap();
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
        assert_eq!(out.samples(), &expected[first * channels..end * channels]);
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
        assert_eq!(
            out.samples(),
            &expected[first as usize * channels..end as usize * channels]
        );
    }
    println!(
        "{{\"rawSamples\":{},\"movieSamples\":{},\"randomQueries\":32,\"equalPcmBits\":true,\"mappingProfile\":\"{}\"}}",
        raw.asset().available.end,
        asset.available.end,
        mo_video_runtime::audio_movie::PROFILE
    );
}
