use super::*;
#[test]
fn resize_preserves_timing_and_fences_aba_completions() {
    let (bytes, q) = fixture_input();
    let mut owner = PptxPlaybackSession::default();
    let first = send(&mut owner, &q, &bytes, &|| false);
    assert_eq!(first["status"], "prepared", "{first}");
    let binding = first["info"]["binding"].clone();
    let query = sample(&binding);
    let sample: PlaybackSampleRequest = serde_json::from_value(query["sample"].clone()).unwrap();
    let inspect = json!({"operation":"inspectTiming","binding":binding});
    let initial = owner.prepare_render(sample.clone(), &|| false).unwrap();
    let words = initial.words().to_vec();
    let initial_pixels = vec![0; (words[2] * words[3] * 4) as usize];
    let result = owner
        .complete_render(
            initial,
            mo_raster::BackendReply {
                status: 0,
                pixels: initial_pixels,
            },
            &|| false,
        )
        .unwrap();
    let evaluated = serde_json::to_value(result.0.playback.evaluated).unwrap();
    let stale = owner.prepare_render(sample.clone(), &|| false).unwrap();
    let timing = send(&mut owner, &inspect, &[], &|| false);
    let original = first["info"]["viewport"].clone();
    let mut enlarged = original.clone();
    for field in ["width", "height"] {
        enlarged[field] = json!(enlarged[field].as_u64().unwrap() * 2);
    }
    enlarged["scale"]["numerator"] = json!(enlarged["scale"]["numerator"].as_u64().unwrap() * 2);
    for (revision, viewport) in [(0, enlarged), (1, original)] {
        let resize = json!({"operation":"resize","binding":binding,"expectedViewportRevision":revision,"viewport":viewport});
        let next = send(&mut owner, &resize, &bytes, &|| false);
        assert_eq!(next["status"], "resized", "{next}");
        assert_eq!(next["info"]["binding"], binding);
        assert_eq!(next["info"]["viewportRevision"], revision + 1);
        assert_eq!(send(&mut owner, &inspect, &[], &|| false), timing);
        assert_eq!(
            code(&send(&mut owner, &resize, &bytes, &|| false)),
            "VIEWPORT_CONFLICT"
        );
    }
    let after = send(
        &mut owner,
        &json!({"operation":"inspect","binding":binding}),
        &[],
        &|| false,
    );
    assert_eq!(after["info"]["planId"], first["info"]["planId"]);
    let rejected = owner
        .complete_render(
            stale,
            mo_raster::BackendReply {
                status: 2,
                pixels: vec![],
            },
            &|| false,
        )
        .err()
        .unwrap();
    assert!(rejected.invalidate_backend);
    assert!(matches!(
        rejected.error,
        PptxPlaybackSessionFailure::Session {
            code: PlaybackSessionFailureCode::BindingConflict,
            ..
        }
    ));
    let current = owner.prepare_render(sample, &|| false).unwrap();
    assert_eq!(current.words(), words);
    let result = owner
        .complete_render(
            current,
            mo_raster::BackendReply {
                status: 0,
                pixels: vec![0; (words[2] * words[3] * 4) as usize],
            },
            &|| false,
        )
        .unwrap();
    assert_eq!(
        serde_json::to_value(result.0.playback.evaluated).unwrap(),
        evaluated
    );
}

#[test]
fn cancelled_viewport_preparation_keeps_original_owner_at_every_checkpoint() {
    let (bytes, q) = fixture_input();
    let binding = q["request"]["binding"].clone();
    let mut viewport = q["request"]["page"]["page"]["viewport"].clone();
    for field in ["width", "height"] {
        viewport[field] = json!(viewport[field].as_u64().unwrap() * 2);
    }
    viewport["scale"]["numerator"] = json!(viewport["scale"]["numerator"].as_u64().unwrap() * 2);
    let resize = json!({"operation":"resize","binding":binding,"expectedViewportRevision":0,"viewport":viewport});
    let mut checkpoints = 0;
    for stop in 0..10000 {
        let mut owner = PptxPlaybackSession::default();
        let initial = send(&mut owner, &q, &bytes, &|| false);
        assert_eq!(initial["status"], "prepared");
        let ticks = Cell::new(0);
        let check = || {
            let n = ticks.get();
            ticks.set(n + 1);
            n >= stop
        };
        let response = send(&mut owner, &resize, &bytes, &check);
        if response["status"] == "resized" {
            checkpoints = stop;
            break;
        }
        assert_eq!(response["status"], "error", "{response}");
        assert!(response.to_string().contains("CANCELLED"), "{response}");
        let inspected = send(
            &mut owner,
            &json!({"operation":"inspect","binding":binding}),
            &[],
            &|| false,
        );
        assert_eq!(
            inspected["info"], initial["info"],
            "cancel checkpoint {stop}"
        );
    }
    assert!(checkpoints > 10, "did not reach a successful resize");
}

#[test]
fn rejected_viewports_leave_prepared_frames_and_live_metadata_usable() {
    let (bytes, q) = fixture_input();
    let mut owner = PptxPlaybackSession::default();
    let first = send(&mut owner, &q, &bytes, &|| false);
    let binding = first["info"]["binding"].clone();
    let query = sample(&binding);
    let sample: PlaybackSampleRequest = serde_json::from_value(query["sample"].clone()).unwrap();
    let pending = owner.prepare_render(sample, &|| false).unwrap();
    let pixels = vec![0; (pending.words()[2] * pending.words()[3] * 4) as usize];
    for (pointer, value) in [
        ("/width", json!(0)),
        ("/width", json!(8193)),
        ("/scale/numerator", json!(0)),
        ("/scale/denominator", json!(0)),
        ("/coordinateTolerance", json!("1")),
        ("/origin/x", json!("1")),
    ] {
        let mut viewport = first["info"]["viewport"].clone();
        *viewport.pointer_mut(pointer).unwrap() = value;
        let resize = json!({"operation":"resize","binding":binding,"expectedViewportRevision":0,"viewport":viewport});
        let rejected = send(&mut owner, &resize, &bytes, &|| false);
        assert_eq!(rejected["status"], "error", "{pointer}: {rejected}");
        let inspected = send(
            &mut owner,
            &json!({"operation":"inspect","binding":binding}),
            &[],
            &|| false,
        );
        assert_eq!(inspected["info"], first["info"]);
    }
    owner
        .complete_render(
            pending,
            mo_raster::BackendReply { status: 0, pixels },
            &|| false,
        )
        .unwrap();
}

#[test]
fn resize_to_fit_uses_kernel_page_geometry_and_the_same_revision_fence() {
    let (bytes, q) = fixture_input();
    let mut owner = PptxPlaybackSession::default();
    let prepared = send(&mut owner, &q, &bytes, &|| false);
    let binding = &prepared["info"]["binding"];
    let resize = json!({"operation":"resizeToFit","binding":binding,"expectedViewportRevision":0,"width":123,"height":83});
    let resized = send(&mut owner, &resize, &bytes, &|| false);
    assert_eq!(resized["status"], "resized", "{resized}");
    assert_eq!(resized["info"]["binding"], *binding);
    assert!(resized["info"]["viewport"]["width"].as_u64().unwrap() <= 123);
    assert!(resized["info"]["viewport"]["height"].as_u64().unwrap() <= 83);
    assert_eq!(
        code(&send(&mut owner, &resize, &bytes, &|| false)),
        "VIEWPORT_CONFLICT"
    );
}
