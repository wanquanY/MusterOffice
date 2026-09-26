use mo_kernel_api::*;
use mo_presentation_compile::PageRenderRequest;
use mo_presentation_edit::Snapshot;
use mo_presentation_model::ValidationLimits;
use mo_raster::{BackendReply, RasterBackend, RasterError};
use serde_json::{Value, json};
use std::cell::Cell;
fn request() -> Value {
    let q: PageRenderRequest = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/playback/page.json"
    ))
    .unwrap();
    let snapshot = Snapshot::new(q.page.document, ValidationLimits::default())
        .unwrap()
        .into_record();
    json!({"playback":{"binding":{"session":"test","revision":snapshot.revision,"generation":"4"},"snapshot":snapshot,"slide":q.page.slide,"at":{"ticks":"1","timescale":3},"history":null},"viewport":q.viewport,"defaults":q.defaults})
}
struct Never;
impl RasterBackend for Never {
    fn raster(&mut self, _: &[u32]) -> Result<BackendReply, RasterError> {
        panic!("rejected request reached renderer")
    }
    fn invalidate(&mut self) {
        panic!("unstarted renderer invalidated")
    }
}
#[test]
fn compiled_frame_is_bound_to_snapshot_time_and_the_existing_page_profile() {
    let q = request();
    let r: Value =
        serde_json::from_str(&compile_playback_page_json(&q.to_string(), &|| false)).unwrap();
    assert_eq!(r["status"], "compiled");
    assert_eq!(r["frame"]["profile"], PLAYBACK_PAGE_PROFILE);
    assert_eq!(
        r["frame"]["page"]["info"]["documentSha256"],
        q["playback"]["snapshot"]["semanticDigest"]
    );
    assert_eq!(
        r["frame"]["frame"]["state"]["binding"],
        q["playback"]["binding"]
    );
    assert_eq!(r["frame"]["frame"]["state"]["time"], q["playback"]["at"]);
}
#[test]
fn stale_snapshot_and_request_ambiguity_never_return_pixels() {
    let mut q = request();
    q["playback"]["binding"]["revision"] = json!("0".repeat(64));
    let (s, pixels) = render_playback_page_json(&q.to_string(), &mut Never, &|| false);
    assert!(pixels.is_empty());
    let r: Value = serde_json::from_str(&s).unwrap();
    assert_eq!(r["error"]["kind"], "timeline");
    assert_eq!(r["error"]["error"]["code"], "REVISION_CONFLICT");
    let q = request()
        .to_string()
        .replacen("\"viewport\":", "\"viewport\":{},\"viewport\":", 1);
    let (s, pixels) = render_playback_page_json(&q, &mut Never, &|| false);
    assert!(pixels.is_empty());
    assert_eq!(
        serde_json::from_str::<Value>(&s).unwrap()["error"]["error"]["code"],
        "INPUT_INVALID"
    );
}
#[test]
fn cancellation_after_component_discards_every_pixel() {
    struct Backend<'a> {
        cancel: &'a Cell<bool>,
        calls: usize,
        invalidated: bool,
    }
    impl RasterBackend for Backend<'_> {
        fn raster(&mut self, f: &[u32]) -> Result<BackendReply, RasterError> {
            self.calls += 1;
            self.cancel.set(true);
            Ok(BackendReply {
                status: 0,
                pixels: vec![7; (f[2] * f[3] * 4) as usize],
            })
        }
        fn invalidate(&mut self) {
            self.invalidated = true
        }
    }
    let cancelled = Cell::new(false);
    let mut backend = Backend {
        cancel: &cancelled,
        calls: 0,
        invalidated: false,
    };
    let (s, pixels) =
        render_playback_page_json(&request().to_string(), &mut backend, &|| cancelled.get());
    assert_eq!(backend.calls, 1);
    assert!(backend.invalidated);
    assert!(pixels.is_empty());
    let r: Value = serde_json::from_str(&s).unwrap();
    assert_eq!(r["error"]["error"]["code"], "CANCELLED");
    assert!(r.get("info").is_none());
}
