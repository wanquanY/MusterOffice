//! In-memory test host. Production persistence, permissions and job scheduling
//! remain product responsibilities; all PCM/DSP/derived reads use real core code.
#[path = "../../../mo-video-audio/tests/support/mod.rs"]
pub mod source;
use mo_common::Digest;
#[cfg(not(target_arch = "wasm32"))]
use mo_stretch_sys::NativeStretch;
use mo_video_audio::*;
use mo_video_compile::{CompiledDocument, MediaBinding};
use mo_video_model::*;
use mo_video_pcm::*;
use source::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

type TileMap = BTreeMap<(Digest, u32), (PcmTileRef, Vec<u8>)>;
#[derive(Clone, Default)]
pub struct Store(Rc<RefCell<TileMap>>);
impl Store {
    fn publish(&self, tiles: &[PcmTile]) {
        let mut values = self.0.borrow_mut();
        for tile in tiles {
            let key = (tile.reference.source.clone(), tile.reference.index);
            if let Some(old) = values.get(&key) {
                assert_eq!(old, &(tile.reference.clone(), tile.bytes.clone()));
            } else {
                values.insert(key, (tile.reference.clone(), tile.bytes.clone()));
            }
        }
    }
}
impl PcmTileStore for Store {
    fn lookup(
        &mut self,
        source: &Digest,
        index: u32,
        _: &dyn Fn() -> bool,
    ) -> mo_video_pcm::Result<Option<PcmTileRef>> {
        Ok(self
            .0
            .borrow()
            .get(&(source.clone(), index))
            .map(|v| v.0.clone()))
    }
    fn read(
        &mut self,
        reference: &PcmTileRef,
        max: usize,
        _: &dyn Fn() -> bool,
    ) -> mo_video_pcm::Result<Vec<u8>> {
        let values = self.0.borrow();
        let value = &values[&(reference.source.clone(), reference.index)];
        assert_eq!(&value.0, reference);
        assert!(value.1.len() <= max);
        Ok(value.1.clone())
    }
}
pub struct Host {
    pub raw: source::MemoryAudio,
    pub profile: Digest,
    pub requests: BTreeMap<Digest, RetimeRequest>,
    readers: BTreeMap<Digest, CanonicalRetimedAudio<Store>>,
    store: Store,
    pub jobs: usize,
    pub peak_materializer_bytes: usize,
    pub fail_publication: bool,
}
impl Host {
    pub fn new(raw: source::MemoryAudio) -> Self {
        Self {
            raw,
            profile: Digest::from_sha256([7; 32]),
            requests: BTreeMap::new(),
            readers: BTreeMap::new(),
            store: Store::default(),
            jobs: 0,
            peak_materializer_bytes: 0,
            fail_publication: false,
        }
    }
}
impl AudioResources for Host {
    fn describe(&mut self, binding: &MediaBinding) -> mo_video_audio::Result<AudioAsset> {
        self.raw.describe(binding)
    }
    fn read(
        &mut self,
        binding: &MediaBinding,
        asset: &AudioAsset,
        range: PcmRange,
        limits: PcmLimits,
        check: &dyn Fn() -> bool,
    ) -> mo_video_audio::Result<PcmBlock> {
        self.raw.read(binding, asset, range, limits, check)
    }
    fn retime_profile(&mut self) -> mo_video_audio::Result<Digest> {
        Ok(self.profile.clone())
    }
    fn read_retimed(
        &mut self,
        request: &RetimeRequest,
        range: PcmRange,
        limits: PcmLimits,
        check: &dyn Fn() -> bool,
    ) -> mo_video_audio::Result<PcmBlock> {
        self.requests
            .entry(request.identity().clone())
            .or_insert_with(|| request.clone());
        let reader = self
            .readers
            .entry(request.identity().clone())
            .or_insert_with(|| {
                CanonicalRetimedAudio::new(
                    request,
                    self.store.clone(),
                    256,
                    PcmReadLimits::default(),
                    256,
                )
                .unwrap()
            });
        Ok(reader.read(request, range, limits, check)?.pcm)
    }
}
struct Input<'a, 'd> {
    mixer: &'a mut AudioMixer<'d>,
    host: &'a mut Host,
    request: RetimeRequest,
}
impl StretchInput for Input<'_, '_> {
    fn read(
        &mut self,
        first: u64,
        output: &mut [f32],
        check: &dyn Fn() -> bool,
    ) -> mo_video_audio::Result<()> {
        let frames = (output.len() / self.request.layout().len()) as u32;
        let result =
            self.mixer
                .render_retime_input(self.host, &self.request, first, frames, check)?;
        output.copy_from_slice(result.pcm.samples());
        Ok(())
    }
}
fn materialize<E: StretchEngine, F: FnMut(u32, usize) -> mo_video_audio::Result<E>>(
    mixer: &mut AudioMixer<'_>,
    host: &mut Host,
    request: RetimeRequest,
    depth: usize,
    factory: &mut F,
) {
    assert!(depth < 16);
    let engine = factory(request.plan().sample_rate(), request.layout().len()).unwrap();
    let mut producer =
        RetimeMaterializer::new(request.clone(), engine, 256, StretchLimits::default(), 256)
            .unwrap();
    host.jobs += 1;
    let mut calls = 0;
    loop {
        calls += 1;
        let limits = if host.fail_publication && calls % 11 == 0 {
            PcmBuildLimits {
                max_candidate_bytes: 0,
                ..PcmBuildLimits::default()
            }
        } else {
            PcmBuildLimits::default()
        };
        let prepared = {
            let mut input = Input {
                mixer,
                host,
                request: request.clone(),
            };
            producer.prepare_next(&mut input, limits, &|| false)
        };
        match prepared {
            Ok(Some(tiles)) => {
                let references: Vec<_> = tiles.iter().map(|t| t.reference.clone()).collect();
                if host.fail_publication {
                    // Retry the same candidate after an unpublished store attempt.
                    let mut input = Input {
                        mixer,
                        host,
                        request: request.clone(),
                    };
                    let again = producer
                        .prepare_next(&mut input, PcmBuildLimits::default(), &|| false)
                        .unwrap()
                        .unwrap();
                    assert_eq!(
                        references,
                        again
                            .iter()
                            .map(|t| t.reference.clone())
                            .collect::<Vec<_>>()
                    );
                    host.store.publish(again);
                } else {
                    host.store.publish(tiles);
                }
                host.peak_materializer_bytes =
                    host.peak_materializer_bytes.max(producer.retained_bytes());
                producer.accept_published().unwrap();
            }
            Ok(None) => {
                assert!(producer.complete());
                break;
            }
            Err(PcmError::Budget) if limits.max_candidate_bytes == 0 => continue,
            Err(PcmError::Audio(AudioError::PendingDerived { identity, .. })) => {
                let child = host.requests[&identity].clone();
                assert_ne!(child.identity(), request.identity());
                materialize(mixer, host, child, depth + 1, factory);
            }
            Err(error) => panic!("materialization error: {error}"),
        }
    }
}
pub fn render_with<E: StretchEngine, F: FnMut(u32, usize) -> mo_video_audio::Result<E>>(
    mixer: &mut AudioMixer<'_>,
    host: &mut Host,
    first: i64,
    frames: u32,
    factory: &mut F,
) -> PcmBlock {
    for _ in 0..64 {
        match mixer.render(host, first, frames, &|| false) {
            Ok(result) => return result.pcm,
            Err(AudioError::PendingDerived { identity, .. }) => {
                materialize(mixer, host, host.requests[&identity].clone(), 0, factory);
            }
            Err(error) => panic!("render error: {error}"),
        }
    }
    panic!("derived dependency work did not converge");
}
pub fn mixer(document: &CompiledDocument) -> AudioMixer<'_> {
    AudioMixer::new(document, ChannelLayout::mono(), MixLimits::default()).unwrap()
}
pub fn pitch(samples: &[f32], rate: u32) -> f64 {
    samples
        .windows(2)
        .filter(|p| p[0] <= 0. && p[1] > 0.)
        .count() as f64
        * f64::from(rate)
        / samples.len() as f64
}

#[cfg(not(target_arch = "wasm32"))]
pub fn render(mixer: &mut AudioMixer<'_>, host: &mut Host, first: i64, frames: u32) -> PcmBlock {
    render_with(mixer, host, first, frames, &mut |rate, channels| {
        NativeStretch::new(rate, channels, 32 << 20)
    })
}
pub fn stepped() -> (VideoDocument, Host) {
    let samples = (0..32000)
        .map(|i| {
            let hz = [220., 440., 880., 1760.][i / 8000];
            (0.3 * (i as f64 * std::f64::consts::TAU * hz / 8000.).sin()) as f32
        })
        .collect();
    let (mut d, raw) = document(samples, 8000);
    track(&mut d, "main", "audio");
    let source = media(
        &raw,
        TimeMap {
            anchors: vec![
                TimeAnchor {
                    local: t(0),
                    source: t(0),
                },
                TimeAnchor {
                    local: t(16000),
                    source: t(8000),
                },
                TimeAnchor {
                    local: t(24000),
                    source: t(16000),
                },
                TimeAnchor {
                    local: t(32000),
                    source: t(32000),
                },
            ],
        },
    );
    clip(&mut d, "audio", "a", range(0, 32000), source);
    d.clips.get_mut(&cid("a")).unwrap().audio.rate = AudioRatePolicy::PreservePitch;
    (d, Host::new(raw))
}
