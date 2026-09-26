#[allow(dead_code)]
#[path = "../../../tools/test-support/source_playback.rs"]
mod support;
use mo_kernel_api::*;
use serde_json::{Value, json};
use support::*;
struct Never;
impl mo_image::ImageDecoder for Never {
    fn decode(&mut self, _: &[u8]) -> Result<mo_image::DecoderReply, mo_image::ImageError> {
        panic!("timing rejection reached decoder")
    }
    fn invalidate(&mut self) {
        panic!("timing rejection invalidated decoder")
    }
}
impl mo_raster::RasterBackend for Never {
    fn raster(&mut self, _: &[u32]) -> Result<mo_raster::BackendReply, mo_raster::RasterError> {
        panic!("timing rejection reached raster")
    }
    fn raster_images(
        &mut self,
        _: &[u32],
        _: &[u8],
    ) -> Result<mo_raster::BackendReply, mo_raster::RasterError> {
        panic!("timing rejection reached image raster")
    }
    fn invalidate(&mut self) {
        panic!("timing rejection invalidated raster")
    }
}
fn input(bytes: &[u8]) -> Value {
    let index = read(bytes);
    json!({"page":{"profile":"drawingml-resource-page-q32-v1-draft","page":request(&index),"imageSource":"embeddedSnapshot","sampling":"nearest","fonts":null},"sample":{"binding":{"session":"source","revision":index.source_sha256,"generation":"7"},"at":{"ticks":"1","timescale":3},"history":null}})
}
fn reject(input: &str, source: &[u8], check: &dyn Fn() -> bool) -> Value {
    let (r, b) = render_pptx_playback_page_json(
        input,
        source,
        &[],
        PptxResourcePageBackends {
            decoder: &mut Never,
            text: None,
            raster: &mut Never,
        },
        check,
    );
    assert!(b.is_empty());
    let v: Value = serde_json::from_str(&r).unwrap();
    assert_eq!(v["status"], "error");
    v["error"].clone()
}
#[test]
fn invalid_binding_missing_history_and_unknown_behavior_stop_before_resources() {
    let base = image_fixture(&picture(
        42,
        "",
        &blip("owned-image", "", STRETCH),
        &solid("FFFFFF"),
    ));
    let bytes = animated(&base, 42, 0, 21600000, "freeze");
    let mut q = input(&bytes);
    q["sample"]["binding"]["revision"] = json!("0".repeat(64));
    assert_eq!(
        reject(&q.to_string(), &bytes, &|| false)["error"]["code"],
        "REVISION_CONFLICT"
    );
    let interactive = rewrite(&bytes, SLIDE, |s| {
        s.replace(
            "<p:cond delay=\"0\"/>",
            "<p:cond evt=\"onClick\" delay=\"0\"><p:tgtEl><p:sldTgt/></p:tgtEl></p:cond>",
        )
    });
    assert_eq!(
        reject(&input(&interactive).to_string(), &interactive, &|| false)["error"]["code"],
        "EVENT_HISTORY_REQUIRED"
    );
    let unknown = rewrite(&bytes, SLIDE, |s| s.replace("p:animRot", "p:animScale"));
    assert_eq!(
        reject(&input(&unknown).to_string(), &unknown, &|| false)["error"]["error"]["code"],
        "MAPPING_NOT_IMPLEMENTED"
    );
}
#[test]
fn strict_wire_limits_source_mismatch_and_cancellation_have_no_pixels() {
    let base = image_fixture(&drawing_shape(42, &solid("112233")));
    let bytes = animated(&base, 42, 0, 21600000, "freeze");
    let q = input(&bytes);
    assert_eq!(
        reject(&q.to_string(), &base, &|| false)["error"]["error"]["code"],
        "SOURCE_CONFLICT"
    );
    assert_eq!(
        reject(&q.to_string(), &bytes, &|| true)["error"]["error"]["code"],
        "CANCELLED"
    );
    let s = q
        .to_string()
        .replacen("\"sample\":", "\"sample\":{},\"sample\":", 1);
    assert_eq!(
        reject(&s, &bytes, &|| false)["error"]["error"]["code"],
        "INPUT_INVALID"
    );
    assert_eq!(
        reject(&" ".repeat(MAX_REQUEST_BYTES + 1), &bytes, &|| false)["error"]["error"]["code"],
        "LIMIT_EXCEEDED"
    );
}
