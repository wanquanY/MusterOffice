//! Actual graph -> source normalization -> DSP -> canonical tiles -> random
//! reads -> mix, using the same test-only host as the native integration suite.
#[allow(dead_code)]
#[path = "../../../crates/mo-video-pcm/tests/retime_host/mod.rs"]
mod host;
use host::{source::*, *};
use mo_video_audio::*;
use mo_video_compile::{CompileLimits, CompiledDocument};
use mo_video_edit::*;
use mo_video_model::*;
use serde::Deserialize;
use serde_json::{Value, json};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    stepped: Vec<f32>,
    tone: Vec<f32>,
}
fn compile(d: VideoDocument) -> CompiledDocument {
    CompiledDocument::new(d, CompileLimits::default(), &|| false).unwrap()
}
fn edited(d: VideoDocument, operations: Vec<Operation>) -> VideoDocument {
    let base = Snapshot::new(d, ValidationLimits::default(), &|| false).unwrap();
    let prepared = prepare(
        &base,
        &Transaction {
            contract: EditContract::V1,
            expected_digest: base.digest().clone(),
            operations,
        },
        EditLimits::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(
        prepare_inverse(
            &prepared.candidate,
            &prepared.inverse,
            EditLimits::default(),
            &|| false
        )
        .unwrap()
        .candidate
        .digest(),
        base.digest()
    );
    prepared.candidate.document().clone()
}
fn bits(pcm: &PcmBlock) -> Vec<u32> {
    pcm.samples().iter().map(|s| s.to_bits()).collect()
}
fn run<E: StretchEngine>(
    input: &str,
    mut factory: impl FnMut(u32, usize) -> mo_video_audio::Result<E>,
) -> std::result::Result<Value, String> {
    if input.len() > 2 << 20 {
        return Err("fixture bytes".into());
    }
    let f: Fixture = mo_common::from_json_str_with_path(input).map_err(|e| e.to_string())?;
    if f.stepped.len() != 32000 || f.tone.len() != 16000 {
        return Err("fixture samples".into());
    }
    let (d, mut h) = stepped();
    h.raw.samples = f.stepped;
    h.fail_publication = true;
    let c = compile(d.clone());
    let mut m = mixer(&c);
    let end = render_with(&mut m, &mut h, 31000, 1000, &mut factory);
    let full = render_with(&mut m, &mut h, 0, 32000, &mut factory);
    assert_eq!(end.samples(), &full.samples()[31000..]);
    let changed = edited(
        d,
        vec![Operation::Split {
            targets: vec![SplitTarget {
                clip: cid("a"),
                right: cid("b"),
            }],
            at: mo_common::RationalTime::new(40001, 16000).unwrap(),
            right_groups: vec![],
            linked_policy: LinkedPolicy::RequireAll,
        }],
    );
    let c = compile(changed);
    let mut m = mixer(&c);
    let split = render_with(&mut m, &mut h, 0, 32000, &mut factory);
    assert_eq!(split.samples(), full.samples());
    assert_eq!(h.jobs, 1);
    let mut windows = vec![];
    for first in [23999, 129, 16001, 0, 30001] {
        windows.push(bits(&render_with(&mut m, &mut h, first, 511, &mut factory)));
    }
    let (mut d, raw) = document(f.tone.clone(), 8000);
    d.sequences.insert(
        sid("child"),
        Sequence {
            id: sid("child"),
            tracks: vec![],
        },
    );
    track(&mut d, "child", "inside");
    clip(
        &mut d,
        "inside",
        "inner",
        range(0, 32000),
        media(&raw, mapping(0, 16000, 32000)),
    );
    d.clips.get_mut(&cid("inner")).unwrap().audio.rate = AudioRatePolicy::PreservePitch;
    track(&mut d, "main", "outside");
    clip(
        &mut d,
        "outside",
        "outer",
        range(0, 16000),
        ClipSource::Sequence {
            sequence: sid("child"),
            time_map: mapping(0, 32000, 16000),
        },
    );
    let mut nested = Host::new(raw);
    let fast = render_with(
        &mut mixer(&compile(d.clone())),
        &mut nested,
        0,
        16000,
        &mut factory,
    );
    assert_eq!(nested.jobs, 1);
    d.clips.get_mut(&cid("outer")).unwrap().audio.rate = AudioRatePolicy::PreservePitch;
    let held = render_with(&mut mixer(&compile(d)), &mut nested, 0, 16000, &mut factory);
    assert_eq!(nested.jobs, 2);
    let (mut d, raw) = document(f.tone, 8000);
    track(&mut d, "main", "audio");
    clip(
        &mut d,
        "audio",
        "a",
        range(0, 26000),
        media(
            &raw,
            TimeMap {
                anchors: [(0, 16000), (8000, 0), (10000, 0), (26000, 8000)]
                    .map(|(l, s)| TimeAnchor {
                        local: t(l),
                        source: t(s),
                    })
                    .to_vec(),
            },
        ),
    );
    d.clips.get_mut(&cid("a")).unwrap().audio.rate = AudioRatePolicy::PreservePitch;
    let mut reverse = Host::new(raw);
    let reversed = render_with(
        &mut mixer(&compile(d)),
        &mut reverse,
        0,
        26000,
        &mut factory,
    );
    assert_eq!(reverse.jobs, 2);
    assert!(reversed.samples()[8000..10000].iter().all(|v| *v == 0.));
    Ok(
        json!({"status":"complete","full":bits(&full),"split":bits(&split),"windows":windows,"nestedResample":bits(&fast),"nestedPreserve":bits(&held),"reverseHold":bits(&reversed),"jobs":[h.jobs,nested.jobs,reverse.jobs],"rustPayloadBytes":h.peak_materializer_bytes}),
    )
}
fn reply(result: std::result::Result<Value, String>) -> String {
    result
        .unwrap_or_else(|error| json!({"status":"invalidFixture","error":error}))
        .to_string()
}
#[cfg(not(target_arch = "wasm32"))]
pub fn video_retime_conformance(input: &str) -> String {
    reply(run(input, |rate, channels| {
        mo_stretch_sys::NativeStretch::new(rate, channels, 32 << 20)
    }))
}
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn video_retime_conformance(input: &str, components: Vec<JsValue>) -> String {
    let components: Vec<mo_stretch_wasm::StretchComponent> =
        components.into_iter().map(|v| v.unchecked_into()).collect();
    let mut pool = components.iter();
    reply(run(input, |rate, channels| {
        let component = pool
            .next()
            .ok_or(AudioError::Invalid("fixture DSP pool exhausted"))?;
        let engine = mo_stretch_wasm::WasmStretch::new(component)?;
        if engine.info().channels != channels || engine.info().sample_rate != rate {
            return Err(AudioError::Invalid("fixture DSP binding"));
        }
        Ok(engine)
    }))
}
