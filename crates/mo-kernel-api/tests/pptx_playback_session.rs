#[allow(dead_code)]
#[path = "../../../tools/test-support/source_playback.rs"]
mod support;
use mo_kernel_api::*;
use serde_json::{Value, json};
use std::cell::Cell;
use support::*;
struct Never;
impl mo_image::ImageDecoder for Never {
    fn decode(&mut self, _: &[u8]) -> Result<mo_image::DecoderReply, mo_image::ImageError> {
        panic!("no image source")
    }
    fn invalidate(&mut self) {
        panic!("no decoder call")
    }
}
fn fixture_input() -> (Vec<u8>, Value) {
    let bytes = animated(
        &image_fixture(&drawing_shape(42, &solid("112233"))),
        42,
        0,
        21600001,
        "freeze",
    );
    let index = read(&bytes);
    let q = json!({"operation":"prepare","request":{"page":{"profile":"drawingml-resource-page-q32-v1-draft","page":request(&index),"imageSource":"embeddedSnapshot","sampling":"nearest","fonts":null},"binding":{"session":"source","revision":index.source_sha256,"generation":"7"}}});
    (bytes, q)
}
fn send(s: &mut PptxPlaybackSession, q: &Value, bytes: &[u8], check: &dyn Fn() -> bool) -> Value {
    let (r, pixels) = s.dispatch_json(
        &q.to_string(),
        bytes,
        &[],
        Some(PptxPlaybackResources {
            decoder: &mut Never,
            text: None,
        }),
        None,
        check,
    );
    assert!(pixels.is_empty());
    serde_json::from_str(&r).unwrap()
}
fn sample(binding: &Value) -> Value {
    json!({"operation":"render","sample":{"binding":binding,"at":{"ticks":"1","timescale":3},"history":null}})
}
fn code(r: &Value) -> &str {
    r["error"]["code"].as_str().unwrap()
}
#[test]
fn source_session_generation_dispose_and_strict_input_share_author_lifecycle() {
    let (bytes, q) = fixture_input();
    let b = &q["request"]["binding"];
    let mut s = PptxPlaybackSession::default();
    assert_eq!(
        code(&send(&mut s, &sample(b), &[], &|| false)),
        "NOT_PREPARED"
    );
    let first = send(&mut s, &q, &bytes, &|| false);
    assert_eq!(first["status"], "prepared", "{first}");
    assert_eq!(
        code(&send(&mut s, &q, &bytes, &|| false)),
        "ALREADY_PREPARED"
    );
    assert_eq!(
        code(&send(&mut s, &sample(b), &bytes, &|| false)),
        "INPUT_INVALID"
    );
    for (key, value) in [
        ("session", "other".to_string()),
        ("generation", "8".into()),
        ("revision", "0".repeat(64)),
    ] {
        let mut wrong = b.clone();
        wrong[key] = json!(value);
        assert_eq!(
            code(&send(&mut s, &sample(&wrong), &[], &|| false)),
            "BINDING_CONFLICT"
        );
    }
    for op in [
        json!({"operation":"advance","binding":b,"generation":"8"}),
        json!({"operation":"dispose","binding":b}),
    ] {
        assert_eq!(code(&send(&mut s, &op, &[], &|| true)), "CANCELLED");
        assert_eq!(
            code(&send(&mut s, &sample(b), &[], &|| false)),
            "RASTER_REQUIRED"
        );
    }
    assert_eq!(
        code(&send(
            &mut s,
            &json!({"operation":"advance","binding":b,"generation":"7"}),
            &[],
            &|| false
        )),
        "GENERATION_NOT_INCREASING"
    );
    let next = send(
        &mut s,
        &json!({"operation":"advance","binding":b,"generation":"18446744073709551615"}),
        &[],
        &|| false,
    );
    assert_eq!(next["info"]["planId"], first["info"]["planId"]);
    assert_eq!(
        code(&send(&mut s, &sample(b), &[], &|| false)),
        "BINDING_CONFLICT"
    );
    let current = &next["info"]["binding"];
    let dispose = json!({"operation":"dispose","binding":current});
    assert_eq!(
        send(&mut s, &dispose, &[], &|| false),
        send(&mut s, &dispose, &[], &|| false)
    );
    assert_eq!(code(&send(&mut s, &q, &bytes, &|| false)), "DISPOSED");
    assert_eq!(
        code(&send(
            &mut s,
            &json!({"operation":"dispose","binding":b}),
            &[],
            &|| false
        )),
        "BINDING_CONFLICT"
    );
}
#[test]
fn source_prepare_failure_and_late_cancellation_do_not_publish_an_owner() {
    let (bytes, q) = fixture_input();
    let b = &q["request"]["binding"];
    let count = Cell::new(0);
    assert_eq!(
        send(&mut PptxPlaybackSession::default(), &q, &bytes, &|| {
            count.set(count.get() + 1);
            false
        })["status"],
        "prepared"
    );
    for stop in [0, 1, 3, count.get() / 2, count.get() - 2, count.get() - 1] {
        let mut s = PptxPlaybackSession::default();
        let n = Cell::new(0);
        let r = send(&mut s, &q, &bytes, &|| {
            let cancel = n.get() == stop;
            n.set(n.get() + 1);
            cancel
        });
        assert_eq!(r["status"], "error", "cancel step {stop}");
        assert_eq!(
            code(&send(&mut s, &sample(b), &[], &|| false)),
            "NOT_PREPARED"
        );
        assert_eq!(send(&mut s, &q, &bytes, &|| false)["status"], "prepared");
    }
    let mut s = PptxPlaybackSession::default();
    let mut bad = q.clone();
    bad["request"]["binding"]["revision"] = json!("0".repeat(64));
    assert_eq!(send(&mut s, &bad, &bytes, &|| false)["status"], "error");
    for raw in [
        "{\"operation\":\"inspect\",\"operation\":\"render\"}".to_string(),
        " ".repeat(MAX_REQUEST_BYTES + 1),
    ] {
        let (r, p) = s.dispatch_json(&raw, &[], &[], None, None, &|| false);
        assert_eq!(
            serde_json::from_str::<Value>(&r).unwrap()["status"],
            "error"
        );
        assert!(p.is_empty());
    }
    assert_eq!(send(&mut s, &q, &bytes, &|| false)["status"], "prepared");
}
#[test]
fn source_session_owns_inputs_and_recovers_after_post_raster_cancellation() {
    struct Raster<'a> {
        cancel: &'a Cell<bool>,
        trigger: bool,
        invalid: bool,
        frame: Vec<u32>,
    }
    impl mo_raster::RasterBackend for Raster<'_> {
        fn raster(&mut self, f: &[u32]) -> Result<mo_raster::BackendReply, mo_raster::RasterError> {
            self.frame = f.to_vec();
            if self.trigger {
                self.cancel.set(true);
            }
            Ok(mo_raster::BackendReply {
                status: 0,
                pixels: vec![1; (f[2] * f[3] * 4) as usize],
            })
        }
        fn raster_images(
            &mut self,
            f: &[u32],
            images: &[u8],
        ) -> Result<mo_raster::BackendReply, mo_raster::RasterError> {
            assert!(images.is_empty());
            self.raster(f)
        }
        fn invalidate(&mut self) {
            self.invalid = true;
        }
    }
    let (mut bytes, q) = fixture_input();
    let mut s = PptxPlaybackSession::default();
    assert_eq!(send(&mut s, &q, &bytes, &|| false)["status"], "prepared");
    bytes.fill(0);
    drop(bytes);
    let request = sample(&q["request"]["binding"]).to_string();
    let c = Cell::new(false);
    let mut bad = Raster {
        cancel: &c,
        trigger: true,
        invalid: false,
        frame: vec![],
    };
    let (r, p) = s.dispatch_json(&request, &[], &[], None, Some(&mut bad), &|| c.get());
    assert!(bad.invalid);
    assert!(!bad.frame.is_empty());
    assert!(p.is_empty());
    assert_eq!(
        serde_json::from_str::<Value>(&r).unwrap()["status"],
        "error"
    );
    c.set(false);
    let mut good = Raster {
        cancel: &c,
        trigger: false,
        invalid: false,
        frame: vec![],
    };
    let (r, p) = s.dispatch_json(&request, &[], &[], None, Some(&mut good), &|| c.get());
    assert_eq!(
        serde_json::from_str::<Value>(&r).unwrap()["status"],
        "rendered"
    );
    assert!(!p.is_empty());
    assert!(!good.invalid);
    assert_eq!(good.frame, bad.frame);
    let b = &q["request"]["binding"];
    let inspection = json!({"operation":"inspectTiming","binding":b});
    let work = send(&mut s, &inspection, &[], &|| false);
    // The first timeline evaluation succeeded; downstream raster cancellation
    // discarded pixels, but the second sample reused its valid intervals.
    assert_eq!(work["info"]["sampler"]["schedulesBuilt"], "1");
    assert_eq!(work["info"]["sampler"]["schedulesReused"], "1");
    assert_eq!(work["info"]["sampler"]["cachedBinding"], *b);
    let advanced = send(
        &mut s,
        &json!({"operation":"advance","binding":b,"generation":"8"}),
        &[],
        &|| false,
    );
    assert_eq!(
        code(&send(&mut s, &inspection, &[], &|| false)),
        "BINDING_CONFLICT"
    );
    let next = &advanced["info"]["binding"];
    let inspection = json!({"operation":"inspectTiming","binding":next});
    let cleared = send(&mut s, &inspection, &[], &|| false);
    assert_eq!(cleared["info"]["sampler"]["retainedIntervals"], "0");
    assert_eq!(cleared["info"]["sampler"]["cachedBinding"], Value::Null);
    send(
        &mut s,
        &json!({"operation":"dispose","binding":next}),
        &[],
        &|| false,
    );
    assert_eq!(code(&send(&mut s, &inspection, &[], &|| false)), "DISPOSED");
}
#[test]
fn source_plan_identity_binds_page_settings_but_excludes_owner() {
    let (bytes, q) = fixture_input();
    let first = send(&mut PptxPlaybackSession::default(), &q, &bytes, &|| false);
    let mut other = q.clone();
    other["request"]["binding"]["session"] = json!("other");
    other["request"]["binding"]["generation"] = json!("100");
    let next = send(&mut PptxPlaybackSession::default(), &other, &bytes, &|| {
        false
    });
    assert_eq!(first["info"]["planId"], next["info"]["planId"]);
    other["request"]["page"]["sampling"] = json!("linear");
    let next = send(&mut PptxPlaybackSession::default(), &other, &bytes, &|| {
        false
    });
    assert_eq!(next["status"], "prepared");
    assert_ne!(first["info"]["planId"], next["info"]["planId"]);
}
