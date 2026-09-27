//! Explicit real-worker checks; no native graphics are linked into this host.
use mo_common::{Digest, RationalTime};
use mo_kernel_api::{PlaybackPageRequest, PlaybackSampleRequest, TimelineEvaluateRequest};
use mo_native_render::playback::*;
use mo_presentation_delivery::Content;
use mo_presentation_edit::Snapshot;
use sha2::{Digest as _, Sha256};
use std::{cell::Cell, io::Read, process::Command, rc::Rc, time::Duration};

fn hash(data: &[u8]) -> Digest {
    Digest::from_sha256(Sha256::digest(data).into())
}
fn config() -> NativePlayback {
    NativePlayback::new(
        std::env::var_os("MO_DELIVERY_WORKER")
            .expect("explicit worker")
            .into(),
        std::env::var("MO_DELIVERY_WORKER_SHA256")
            .expect("worker digest")
            .try_into()
            .unwrap(),
        Duration::from_secs(10),
    )
    .unwrap()
}
fn author() -> PlaybackPrepareRequest {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/playback/page.json"
    ))
    .unwrap();
    let snapshot = Snapshot::new(
        serde_json::from_value(v["page"]["document"].clone()).unwrap(),
        Default::default(),
    )
    .unwrap()
    .into_record();
    serde_json::from_value(serde_json::json!({"binding":{"session":"sdk:author","revision":snapshot.revision,"generation":"0"},
        "snapshot":snapshot,"slide":v["page"]["slide"],"viewport":v["viewport"],"defaults":v["defaults"]})).unwrap()
}
fn time(n: i64, d: u32) -> RationalTime {
    serde_json::from_value(serde_json::json!({"ticks":n.to_string(),"timescale":d})).unwrap()
}
fn raw(
    mode: &str,
    value: &impl serde::Serialize,
    source: &[u8],
    fonts: &[u8],
) -> (serde_json::Value, Vec<u8>) {
    let json = serde_json::to_vec(value).unwrap();
    let mut header = (json.len() as u32).to_le_bytes().to_vec();
    if mode == "--pptx-playback-page" {
        header.extend_from_slice(&(source.len() as u32).to_le_bytes());
        header.extend_from_slice(&(fonts.len() as u32).to_le_bytes());
    }
    let mut command = Command::new(std::env::var_os("MO_DELIVERY_WORKER").unwrap());
    command.arg(mode);
    mo_native_worker::exchange(
        command,
        vec![header, json, source.to_vec(), fonts.to_vec()],
        Duration::from_secs(10),
        |out| {
            let mut sizes = [0; 8];
            out.read_exact(&mut sizes).map_err(|e| e.to_string())?;
            let mut json = vec![0; u32::from_le_bytes(sizes[..4].try_into().unwrap()) as usize];
            let mut pixels = vec![0; u32::from_le_bytes(sizes[4..].try_into().unwrap()) as usize];
            out.read_exact(&mut json).map_err(|e| e.to_string())?;
            out.read_exact(&mut pixels).map_err(|e| e.to_string())?;
            Ok((serde_json::from_slice(&json).unwrap(), pixels))
        },
    )
    .unwrap()
}
#[test]
#[ignore = "requires explicitly pinned MO_DELIVERY_WORKER"]
fn author_animation_matches_one_shot_and_generation_failures_do_not_damage_owner() {
    let prepare = author();
    let mut owner = config().prepare_author(prepare.clone(), &|| false).unwrap();
    let id = owner.info().plan_id.clone();
    let mut digests = std::collections::BTreeSet::new();
    // Wire representations can differ while denoting the same exact instant.
    for at in [
        time(0, 1),
        time(1, 3),
        time(1, 1),
        time(2, 1),
        time(1, 3),
        time(0, 4),
        time(2, 6),
        time(4, 4),
        time(6, 3),
    ] {
        let frame = owner.sample(at, None, &|| false).unwrap();
        assert_eq!(frame.info.frame.state.time, at.normalized());
        let q = PlaybackPageRequest {
            playback: TimelineEvaluateRequest {
                snapshot: prepare.snapshot.clone(),
                slide: prepare.slide.clone(),
                binding: prepare.binding.clone(),
                at,
                history: None,
            },
            viewport: prepare.viewport.clone(),
            defaults: prepare.defaults.clone(),
        };
        let (old, pixels) = raw("--playback-page", &q, &[], &[]);
        assert_eq!(old["status"], "rendered");
        assert_eq!(old["info"], serde_json::to_value(&frame.info).unwrap());
        assert_eq!(pixels, frame.pixels);
        digests.insert(hash(&pixels));
    }
    assert!(digests.len() > 1, "animation must affect actual pixels");
    assert!(
        owner
            .advance(PlaybackGeneration::new(0), &|| false)
            .is_err()
    );
    assert_eq!(owner.info().binding.generation.get(), 0);
    assert!(!owner.is_stopped());
    owner
        .advance(PlaybackGeneration::new(9007199254740993), &|| false)
        .unwrap();
    assert_eq!(owner.info().plan_id, id);
    assert_eq!(
        owner
            .sample(time(0, 1), None, &|| false)
            .unwrap()
            .info
            .frame
            .state
            .binding,
        owner.info().binding
    );
    assert_eq!(
        owner.timing(&|| false).unwrap().binding,
        owner.info().binding
    );
    owner.dispose(&|| false).unwrap();
}

struct Images;
impl mo_pptx::Resources for Images {
    fn open(
        &self,
        _: &mo_common::ResourceId,
    ) -> Result<mo_pptx::ResourceData<'_>, mo_pptx::PptxError> {
        static BYTES: &[u8] =
            include_bytes!("../../../fixtures/presentations/native-export/resources.bin");
        Ok(mo_pptx::ResourceData {
            reader: &BYTES,
            byte_length: BYTES.len() as u64,
        })
    }
}
struct Counted {
    bytes: Vec<u8>,
    reads: Rc<Cell<usize>>,
    dropped: Rc<Cell<bool>>,
}
impl mo_opc::ReaderAt for Counted {
    fn read_at(&self, buf: &mut [u8], offset: u64) -> std::io::Result<usize> {
        self.reads.set(self.reads.get() + 1);
        self.bytes.read_at(buf, offset)
    }
}
impl Drop for Counted {
    fn drop(&mut self) {
        self.dropped.set(true);
    }
}
#[test]
#[ignore = "requires explicitly pinned MO_DELIVERY_WORKER"]
fn source_resources_survive_input_release_and_match_actual_one_shot_pixels() {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/delivery/input.json"
    ))
    .unwrap();
    let source = mo_pptx::export(
        &serde_json::from_value(v["document"].clone()).unwrap(),
        &serde_json::from_value(v["settings"]["defaults"].clone()).unwrap(),
        &Images,
        Default::default(),
        &|| false,
    )
    .unwrap();
    let fonts = include_bytes!("../../../fixtures/fonts/owned.ttf").to_vec();
    let sha = hash(&source);
    let request:PptxPlaybackPrepareRequest=serde_json::from_value(serde_json::json!({
        "binding":{"session":"sdk:source","revision":sha,"generation":"0"},
        "page":{"profile":"drawingml-resource-page-q32-v1-draft","page":{"profile":"drawingml-static-solid-page-v1-draft","expectedSourceSha256":sha,"slide":"/ppt/slides/slide1.xml", "viewport": {
          "width":640,"height":360,"origin":{"x":"0","y":"0"},"scale":{"numerator":1,"denominator":19050},"coordinateTolerance":"16777216","background":[255,255,255,255]},"colorContext":v["settings"]["colorContext"]},
          "imageSource":v["settings"]["imageSource"],"sampling":v["settings"]["sampling"],"fonts":v["settings"]["fonts"]}
    })).unwrap();
    let reads = Rc::new(Cell::new(0));
    let dropped = Rc::new(Cell::new(false));
    let mut owner = {
        let source = Counted {
            bytes: source.clone(),
            reads: reads.clone(),
            dropped: dropped.clone(),
        };
        let fonts = fonts.clone();
        config()
            .prepare_source(
                request.clone(),
                Content {
                    byte_length: source.bytes.len() as u64,
                    reader: &source,
                },
                Content {
                    byte_length: fonts.len() as u64,
                    reader: &fonts,
                },
                &|| false,
            )
            .unwrap()
    };
    assert!(dropped.get());
    assert!(reads.get() > 0);
    let count = reads.get();
    for at in [
        time(0, 1),
        time(1, 2),
        time(3, 1),
        time(0, 4),
        time(2, 4),
        time(6, 2),
    ] {
        let frame = owner.sample(at, None, &|| false).unwrap();
        assert_eq!(frame.info.playback.evaluated.state.time, at.normalized());
        let old_request = mo_kernel_api::PptxPlaybackPageRequest {
            page: request.page.clone(),
            sample: PlaybackSampleRequest {
                binding: request.binding.clone(),
                at,
                history: None,
            },
        };
        let (old, pixels) = raw("--pptx-playback-page", &old_request, &source, &fonts);
        assert_eq!(old["status"], "rendered", "{old}");
        assert_eq!(frame.pixels, pixels);
        assert_eq!(
            serde_json::to_value(&frame.info.playback).unwrap(),
            old["info"]["playback"]
        );
        assert_eq!(frame.info.page.text_work.component_calls, 0);
        assert_eq!(frame.info.page.gather_copy_bytes, 0);
        assert_eq!(reads.get(), count);
    }
    let id = owner.info().plan_id.clone();
    owner
        .advance(PlaybackGeneration::new(2), &|| false)
        .unwrap();
    assert_eq!(owner.info().plan_id, id);
    owner.dispose(&|| false).unwrap();
}
#[test]
#[ignore = "requires explicitly pinned MO_DELIVERY_WORKER"]
fn cancellation_invalidates_the_owner_and_configuration_rejects_wrong_worker() {
    let runtime = config();
    let mut owner = runtime.prepare_author(author(), &|| false).unwrap();
    assert!(matches!(
        owner.sample(time(0, 1), None, &|| true),
        Err(Error::Transport(SessionError::Cancelled))
    ));
    assert!(owner.is_stopped());
    assert!(matches!(
        owner.timing(&|| false),
        Err(Error::Transport(SessionError::Stopped))
    ));
    let worker = std::env::var_os("MO_DELIVERY_WORKER").unwrap().into();
    assert!(NativePlayback::new(worker, hash(b"different"), Duration::from_secs(2)).is_err());
}
