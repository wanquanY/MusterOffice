//! Actual pinned worker checks with independently declared static control slides.
#[allow(dead_code)]
#[path = "../../../tools/test-support/source_visibility.rs"]
mod support;
use mo_common::{Digest, RationalTime};
use mo_native_render::playback::*;
use mo_presentation_delivery::Content;
use sha2::{Digest as _, Sha256};
use std::time::Duration;
use support::*;
fn time(ms: i64) -> RationalTime {
    RationalTime::new(ms, 1000).unwrap()
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
        Duration::from_secs(20),
    )
    .unwrap()
}
fn request(bytes: &[u8]) -> PptxPlaybackPrepareRequest {
    let sha = Digest::from_sha256(Sha256::digest(bytes).into());
    let fonts: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/fonts/manifest-paragraph.json"
    ))
    .unwrap();
    serde_json::from_value(serde_json::json!({
        "binding":{"session":"visibility-real-worker","revision":sha,"generation":"0"},
        "page":{"profile":"drawingml-resource-page-q32-v1-draft","page":support::request(&read(bytes)),
            "imageSource":"embeddedSnapshot","sampling":"nearest","fonts":fonts["manifest"]}
    })).unwrap()
}
fn owner(bytes: &[u8]) -> SourcePlayback {
    let font = include_bytes!("../../../fixtures/fonts/owned.ttf").to_vec();
    config()
        .prepare_source(
            request(bytes),
            Content {
                byte_length: bytes.len() as u64,
                reader: &bytes,
            },
            Content {
                byte_length: font.len() as u64,
                reader: &font,
            },
            &|| false,
        )
        .unwrap()
}
#[test]
#[ignore = "requires explicitly pinned MO_DELIVERY_WORKER"]
fn retained_visibility_matches_static_pixels_with_text_images_and_revealable_resources() {
    let image = blip("owned-image", "", STRETCH);
    let cyan = blip("owned-cyan", "", STRETCH);
    let text = shape(42, 0, 0, "", &colored("AA", "FF0000"));
    let body = receiver(42, [0, 0, 800000, 800000], "", &image);
    let mixed = shape(42, 0, 0, "", &colored("AA", "FF0000"))
        + &receiver(43, [800000, 0, 800000, 800000], "", &image);
    let mut cases = vec![
        ("text", text, 42, false),
        (
            "image-order",
            body + &receiver(43, [800000, 0, 800000, 800000], "", &cyan),
            42,
            false,
        ),
        (
            "reveal-text",
            shape(42, 0, 0, "", &colored("AA", "FF0000")),
            42,
            true,
        ),
        ("reveal-image", picture(42, "", &image, ""), 42, true),
    ];
    cases.push((
        "group",
        group(
            90,
            &transform("", [0, 0, 1600000, 800000], [0, 0, 1600000, 800000]),
            "<a:noFill/>",
            &mixed,
        ),
        90,
        false,
    ));
    let out = std::env::var_os("MO_VISIBILITY_OUTPUT").map(std::path::PathBuf::from);
    for (name, body, target, initially_hidden) in cases {
        let shown = image_fixture(&body);
        let off = hidden(&shown, target);
        let base = if initially_hidden { &off } else { &shown };
        let animated = visibility(
            base,
            target,
            if initially_hidden {
                "visible"
            } else {
                "hidden"
            },
            "remove",
        );
        let mut yes = owner(&shown);
        let mut no = owner(&off);
        let mut actual = owner(&animated);
        let prep = serde_json::to_value(&actual.info().preparation).unwrap();
        let mut frames = vec![];
        for (ms, active) in [
            (0, false),
            (499, false),
            (500, true),
            (1000, true),
            (1499, true),
            (1500, false),
            (3000, false),
            (750, true),
            (0, false),
        ] {
            let frame = actual.sample(time(ms), None, &|| false).unwrap();
            let visible = initially_hidden == active;
            let expected = if visible {
                yes.sample(time(0), None, &|| false).unwrap()
            } else {
                no.sample(time(0), None, &|| false).unwrap()
            };
            assert_eq!(frame.pixels, expected.pixels, "{name} at {ms}");
            assert_eq!(
                frame.info.page.text_frames, expected.info.page.text_frames,
                "{name} text coverage at {ms}"
            );
            assert_eq!(
                serde_json::to_value(&frame.info.page.text_capacity).unwrap(),
                serde_json::to_value(&expected.info.page.text_capacity).unwrap()
            );
            assert_eq!(frame.info.page.text_work.component_calls, 0);
            assert_eq!(frame.info.page.gather_copy_bytes, 0);
            assert_eq!(
                frame.info.playback.evaluated.state.profile,
                mo_timeline::PROPERTY_FRAME_PROFILE
            );
            assert_eq!(
                serde_json::to_value(&actual.info().preparation).unwrap(),
                prep
            );
            if let Some(out) = &out {
                let path = out.join(name);
                std::fs::create_dir_all(&path).unwrap();
                std::fs::write(
                    path.join(format!("frame-{}.rgba", frames.len())),
                    &frame.pixels,
                )
                .unwrap();
            }
            frames.push(serde_json::json!({"at":time(ms),"info":frame.info,"sha256":format!("{:x}",Sha256::digest(&frame.pixels))}));
        }
        if let Some(out) = &out {
            let path = out.join(name);
            std::fs::write(path.join("input.pptx"), &animated).unwrap();
            std::fs::write(
                path.join("prepare.json"),
                serde_json::to_vec_pretty(&request(&animated)).unwrap(),
            )
            .unwrap();
            std::fs::write(
                path.join("frames.json"),
                serde_json::to_vec_pretty(&frames).unwrap(),
            )
            .unwrap();
        }
        actual.dispose(&|| false).unwrap();
        yes.dispose(&|| false).unwrap();
        no.dispose(&|| false).unwrap();
    }
}
