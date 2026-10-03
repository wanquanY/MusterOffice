use mo_kernel_api::*;
use mo_presentation_compile::PageRenderRequest;
use mo_presentation_edit::Snapshot;
use mo_presentation_model::ValidationLimits;
use mo_raster::{BackendReply, RasterBackend, RasterError};
use serde_json::{Value, json};
use std::cell::Cell;

fn prepare() -> Value {
    let q: PageRenderRequest = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/playback/page.json"
    ))
    .unwrap();
    let s = Snapshot::new(q.page.document, ValidationLimits::default())
        .unwrap()
        .into_record();
    json!({"operation":"prepare","request":{"binding":{"session":"retained","revision":s.revision,"generation":"4"},"snapshot":s,"slide":q.page.slide,"viewport":q.viewport,"defaults":q.defaults}})
}
fn send(session: &mut PlaybackSession, q: &Value) -> Value {
    let (s, pixels) = session.dispatch_json(&q.to_string(), None, &|| false);
    assert!(pixels.is_empty());
    serde_json::from_str(&s).unwrap()
}
fn sample(binding: &Value, ticks: &str, scale: u32) -> Value {
    json!({"operation":"compile","sample":{"binding":binding,"at":{"ticks":ticks,"timescale":scale},"history":null}})
}
fn code(r: &Value) -> &str {
    r["error"]["code"].as_str().unwrap()
}

#[test]
fn timing_diagnostics_follow_owner_fences_and_generation_invalidation() {
    let mut s = PlaybackSession::default();
    let q = prepare();
    let b = q["request"]["binding"].clone();
    let inspect = json!({"operation":"inspectTiming","binding":b});
    assert_eq!(code(&send(&mut s, &inspect)), "NOT_PREPARED");
    assert_eq!(send(&mut s, &q)["status"], "prepared");
    assert_eq!(
        send(&mut s, &inspect)["info"]["sampler"]["cachedBinding"],
        Value::Null
    );
    for n in ["1", "2", "0"] {
        assert_eq!(send(&mut s, &sample(&b, n, 3))["status"], "compiled");
    }
    let before = send(&mut s, &inspect);
    assert_eq!(before["status"], "timingInspected");
    assert_eq!(before["info"]["sampler"]["schedulesBuilt"], "1");
    assert_eq!(before["info"]["sampler"]["schedulesReused"], "2");
    assert_eq!(before["info"]["sampler"]["retainedEvents"], "0");
    assert_eq!(before["info"]["sampler"]["cachedBinding"], b);
    assert_eq!(
        code(&send(
            &mut s,
            &json!({"operation":"advance","binding":b,"generation":"4"})
        )),
        "GENERATION_NOT_INCREASING"
    );
    assert_eq!(send(&mut s, &inspect), before);
    let advance = json!({"operation":"advance","binding":b,"generation":"5"});
    let (r, _) = s.dispatch_json(&advance.to_string(), None, &|| true);
    assert_eq!(code(&serde_json::from_str(&r).unwrap()), "CANCELLED");
    assert_eq!(send(&mut s, &inspect), before);
    let next = send(&mut s, &advance)["info"]["binding"].clone();
    assert_eq!(code(&send(&mut s, &inspect)), "BINDING_CONFLICT");
    let inspect = json!({"operation":"inspectTiming","binding":next});
    let empty = send(&mut s, &inspect);
    assert_eq!(empty["info"]["sampler"]["cachedBinding"], Value::Null);
    assert_eq!(empty["info"]["sampler"]["retainedIntervals"], "0");
    assert_eq!(send(&mut s, &sample(&next, "1", 3))["status"], "compiled");
    assert_eq!(
        send(&mut s, &inspect)["info"]["sampler"]["schedulesBuilt"],
        "2"
    );
    send(&mut s, &json!({"operation":"dispose","binding":next}));
    assert_eq!(code(&send(&mut s, &inspect)), "DISPOSED");
}

#[test]
fn prepare_once_samples_equal_legacy_boundary_and_source_is_owned() {
    let mut s = PlaybackSession::default();
    let mut q = prepare();
    let original = q.clone();
    let prepared = send(&mut s, &q);
    assert_eq!(prepared["status"], "prepared");
    assert_eq!(code(&send(&mut s, &q)), "ALREADY_PREPARED");
    q["request"]["snapshot"]["document"] = json!({});
    for (n, d) in [("0", 1), ("1", 3), ("2", 1), ("1", 3)] {
        let request = sample(&prepared["info"]["binding"], n, d);
        let r = send(&mut s, &request);
        assert_eq!(r["status"], "compiled");
        let p = &original["request"];
        let legacy = json!({"playback":{"snapshot":p["snapshot"],"slide":p["slide"],"binding":p["binding"],"at":request["sample"]["at"],"history":null},"viewport":p["viewport"],"defaults":p["defaults"]});
        assert_eq!(
            r,
            serde_json::from_str::<Value>(&compile_playback_page_json(
                &legacy.to_string(),
                &|| false
            ))
            .unwrap()
        );
    }
}
#[test]
fn generation_advance_preserves_plan_and_fences_every_prior_operation() {
    let mut s = PlaybackSession::default();
    let q = prepare();
    let first = send(&mut s, &q);
    let b = &first["info"]["binding"];
    let old = send(&mut s, &sample(b, "1", 3));
    for generation in ["0", "4"] {
        assert_eq!(
            code(&send(
                &mut s,
                &json!({"operation":"advance","binding":b,"generation":generation})
            )),
            "GENERATION_NOT_INCREASING"
        );
    }
    let next = send(
        &mut s,
        &json!({"operation":"advance","binding":b,"generation":"18446744073709551615"}),
    );
    assert_eq!(next["status"], "advanced");
    assert_eq!(next["info"]["planId"], first["info"]["planId"]);
    for request in [
        sample(b, "1", 3),
        json!({"operation":"inspect","binding":b}),
        json!({"operation":"dispose","binding":b}),
        json!({"operation":"advance","binding":b,"generation":"5"}),
    ] {
        assert_eq!(code(&send(&mut s, &request)), "BINDING_CONFLICT");
    }
    let current = &next["info"]["binding"];
    let frame = send(&mut s, &sample(current, "1", 3));
    assert_eq!(frame["frame"]["page"], old["frame"]["page"]);
    assert_eq!(frame["frame"]["placements"], old["frame"]["placements"]);
    assert_ne!(
        frame["frame"]["frame"]["sha256"],
        old["frame"]["frame"]["sha256"]
    );
    assert_eq!(
        code(&send(
            &mut s,
            &json!({"operation":"advance","binding":current,"generation":"0"})
        )),
        "GENERATION_NOT_INCREASING"
    );
}
#[test]
fn different_snapshot_session_or_generation_cannot_use_or_dispose_plan() {
    let mut s = PlaybackSession::default();
    let p = send(&mut s, &prepare());
    let b = &p["info"]["binding"];
    for (key, value) in [
        ("revision", "0".repeat(64)),
        ("session", "another".into()),
        ("generation", "5".into()),
    ] {
        let mut bad = b.clone();
        bad[key] = json!(value);
        assert_eq!(
            code(&send(&mut s, &sample(&bad, "0", 1))),
            "BINDING_CONFLICT"
        );
        assert_eq!(
            code(&send(&mut s, &json!({"operation":"dispose","binding":bad}))),
            "BINDING_CONFLICT"
        );
    }
    assert_eq!(send(&mut s, &sample(b, "0", 1))["status"], "compiled");
}
#[test]
fn preparation_failure_and_cancellation_are_atomic_and_dispose_is_terminal() {
    let q = prepare();
    let b = &q["request"]["binding"];
    let mut s = PlaybackSession::default();
    assert_eq!(code(&send(&mut s, &sample(b, "0", 1))), "NOT_PREPARED");
    let mut bad = q.clone();
    bad["request"]["snapshot"]["semanticDigest"] = json!("0".repeat(64));
    assert_eq!(send(&mut s, &bad)["status"], "error");
    let count = Cell::new(0);
    let mut probe = PlaybackSession::default();
    let (r, _) = probe.dispatch_json(&q.to_string(), None, &|| {
        count.set(count.get() + 1);
        false
    });
    assert_eq!(
        serde_json::from_str::<Value>(&r).unwrap()["status"],
        "prepared"
    );
    for stop in 0..count.get() {
        let mut candidate = PlaybackSession::default();
        let step = Cell::new(0);
        let (r, bytes) = candidate.dispatch_json(&q.to_string(), None, &|| {
            let cancel = step.get() == stop;
            step.set(step.get() + 1);
            cancel
        });
        assert_eq!(
            serde_json::from_str::<Value>(&r).unwrap()["status"],
            "error"
        );
        assert!(bytes.is_empty());
        assert_eq!(
            code(&send(&mut candidate, &sample(b, "0", 1))),
            "NOT_PREPARED"
        );
    }
    assert_eq!(send(&mut s, &q)["status"], "prepared");
    for op in [
        json!({"operation":"advance","binding":b,"generation":"5"}),
        json!({"operation":"dispose","binding":b}),
    ] {
        let (r, _) = s.dispatch_json(&op.to_string(), None, &|| true);
        assert_eq!(code(&serde_json::from_str(&r).unwrap()), "CANCELLED");
        assert_eq!(send(&mut s, &sample(b, "0", 1))["status"], "compiled");
    }
    let dispose = json!({"operation":"dispose","binding":b});
    assert_eq!(send(&mut s, &dispose), send(&mut s, &dispose));
    assert_eq!(code(&send(&mut s, &q)), "DISPOSED");
    assert_eq!(code(&send(&mut s, &sample(b, "0", 1))), "DISPOSED");
}
#[test]
fn malformed_and_oversize_wire_data_cannot_reconfigure_the_session() {
    let mut s = PlaybackSession::default();
    let p = send(&mut s, &prepare());
    let b = &p["info"]["binding"];
    for input in [
        "{\"operation\":\"inspect\",\"operation\":\"dispose\"}".into(),
        json!({"operation":"advance","binding":b,"generation":"18446744073709551616"}).to_string(),
        json!({"operation":"inspect","binding":b,"extra":true}).to_string(),
        " ".repeat(MAX_REQUEST_BYTES + 1),
    ] {
        let (r, bytes) = s.dispatch_json(&input, None, &|| false);
        assert!(bytes.is_empty());
        assert_eq!(
            serde_json::from_str::<Value>(&r).unwrap()["status"],
            "error"
        );
        assert_eq!(send(&mut s, &sample(b, "0", 1))["status"], "compiled");
    }
}
#[test]
fn history_must_be_rebound_explicitly_and_failed_sample_keeps_plan() {
    let mut q = prepare();
    let mut document = q["request"]["snapshot"]["document"].clone();
    document["timelines"]["slide:1"]["nodes"][0]["start"] =
        json!({"kind":"click","target":null,"delay":{"ticks":"0","timescale":1}});
    let init: Value = serde_json::from_str(&dispatch_json(
        &json!({"operation":"initialize","document":document}).to_string(),
    ))
    .unwrap();
    q["request"]["snapshot"] = init["snapshot"].clone();
    q["request"]["binding"]["revision"] = init["snapshot"]["revision"].clone();
    let mut s = PlaybackSession::default();
    let p = send(&mut s, &q);
    let b = &p["info"]["binding"];
    let mut request = sample(b, "1", 1);
    assert_eq!(
        send(&mut s, &request)["error"]["error"]["error"]["code"],
        "EVENT_HISTORY_REQUIRED"
    );
    request["sample"]["history"] =
        json!({"binding":b,"through":{"ticks":"2","timescale":1},"events":[]});
    assert_eq!(send(&mut s, &request)["status"], "compiled");
    let p = send(
        &mut s,
        &json!({"operation":"advance","binding":b,"generation":"5"}),
    );
    request["sample"]["binding"] = p["info"]["binding"].clone();
    assert_eq!(
        send(&mut s, &request)["error"]["error"]["error"]["code"],
        "EVENT_HISTORY_INVALID"
    );
    request["sample"]["history"]["binding"] = p["info"]["binding"].clone();
    assert_eq!(send(&mut s, &request)["status"], "compiled");
}
#[test]
fn post_raster_cancellation_discards_pixels_without_disposing_the_plan() {
    struct Backend<'a> {
        cancel: &'a Cell<bool>,
        invalid: bool,
        calls: usize,
    }
    impl RasterBackend for Backend<'_> {
        fn raster(&mut self, f: &[u32]) -> Result<BackendReply, RasterError> {
            self.calls += 1;
            self.cancel.set(true);
            Ok(BackendReply {
                status: 0,
                pixels: vec![1; (f[2] * f[3] * 4) as usize],
            })
        }
        fn invalidate(&mut self) {
            self.invalid = true;
        }
    }
    let mut s = PlaybackSession::default();
    let p = send(&mut s, &prepare());
    let mut q = sample(&p["info"]["binding"], "1", 3);
    q["operation"] = json!("render");
    assert_eq!(code(&send(&mut s, &q)), "RASTER_REQUIRED");
    let cancelled = Cell::new(false);
    let mut backend = Backend {
        cancel: &cancelled,
        invalid: false,
        calls: 0,
    };
    let (r, bytes) = s.dispatch_json(&q.to_string(), Some(&mut backend), &|| cancelled.get());
    assert!(bytes.is_empty());
    assert!(backend.invalid);
    assert_eq!(backend.calls, 1);
    assert_eq!(
        serde_json::from_str::<Value>(&r).unwrap()["error"]["error"]["error"]["code"],
        "CANCELLED"
    );
    assert_eq!(
        send(&mut s, &sample(&p["info"]["binding"], "1", 3))["status"],
        "compiled"
    );
}
#[test]
fn plan_identity_changes_with_presentation_settings_but_not_owner_generation() {
    let mut s = PlaybackSession::default();
    let q = prepare();
    let first = send(&mut s, &q);
    let mut other = q.clone();
    other["request"]["binding"]["session"] = json!("other");
    other["request"]["binding"]["generation"] = json!("20");
    let next = send(&mut PlaybackSession::default(), &other);
    assert_eq!(first["info"]["planId"], next["info"]["planId"]);
    other["request"]["defaults"]["pageBackground"] = json!([0, 0, 0, 255]);
    let changed = send(&mut PlaybackSession::default(), &other);
    assert_eq!(changed["status"], "prepared");
    assert_ne!(first["info"]["planId"], changed["info"]["planId"]);
}

#[test]
fn prepared_frames_are_bound_to_owner_generation_and_disposal() {
    let q = prepare();
    let binding = q["request"]["binding"].clone();
    let sample: PlaybackSampleRequest = serde_json::from_value(
        json!({"binding":binding,"at":{"ticks":"1","timescale":3},"history":null}),
    )
    .unwrap();
    let mut owner = PlaybackSession::default();
    let mut other = PlaybackSession::default();
    send(&mut owner, &q);
    send(&mut other, &q);
    let reply = || mo_raster::BackendReply {
        status: 0,
        pixels: vec![],
    };
    // Fault disposition is preserved even if an obsolete/wrong owner rejects the frame.
    let fault = owner.prepare_render(sample.clone(), &|| false).unwrap();
    let error = other
        .complete_render(
            fault,
            mo_raster::BackendReply {
                status: 2,
                pixels: vec![],
            },
            &|| false,
        )
        .err()
        .unwrap();
    assert!(error.invalidate_backend);
    for status in 0..=5 {
        let pending = owner.prepare_render(sample.clone(), &|| false).unwrap();
        let error = owner
            .complete_render(
                pending,
                mo_raster::BackendReply {
                    status,
                    pixels: vec![],
                },
                &|| false,
            )
            .err()
            .unwrap();
        assert_eq!(error.invalidate_backend, !matches!(status, 1 | 3));
    }
    let pending = owner.prepare_render(sample.clone(), &|| false).unwrap();
    let result = other.complete_render(pending, reply(), &|| false);
    let result = result.map_err(|e| {
        assert!(!e.invalidate_backend);
        e.error
    });
    assert!(matches!(
        result,
        Err(PlaybackSessionFailure::Session {
            code: PlaybackSessionFailureCode::BindingConflict,
            ..
        })
    ));
    let pending = owner.prepare_render(sample.clone(), &|| false).unwrap();
    let next = (binding["generation"]
        .as_str()
        .unwrap()
        .parse::<u64>()
        .unwrap()
        + 1)
    .to_string();
    send(
        &mut owner,
        &json!({"operation":"advance","binding":binding,"generation":next}),
    );
    let result = owner.complete_render(pending, reply(), &|| false);
    let result = result.map_err(|e| {
        assert!(!e.invalidate_backend);
        e.error
    });
    assert!(matches!(
        result,
        Err(PlaybackSessionFailure::Session {
            code: PlaybackSessionFailureCode::BindingConflict,
            ..
        })
    ));
    let mut sample = sample;
    sample.binding.generation = serde_json::from_value(json!(next)).unwrap();
    let abandoned = owner.prepare_render(sample.clone(), &|| false).unwrap();
    drop(abandoned);
    let pending = owner.prepare_render(sample.clone(), &|| false).unwrap();
    let result = owner.complete_render(pending, reply(), &|| true);
    let result = result.map_err(|e| {
        assert!(!e.invalidate_backend);
        e.error
    });
    assert!(matches!(
        result,
        Err(PlaybackSessionFailure::Session {
            code: PlaybackSessionFailureCode::Cancelled,
            ..
        })
    ));
    let pending = owner.prepare_render(sample.clone(), &|| false).unwrap();
    send(
        &mut owner,
        &json!({"operation":"dispose","binding":sample.binding}),
    );
    assert!(!pending.words().is_empty());
    let result = owner.complete_render(pending, reply(), &|| false);
    let result = result.map_err(|e| {
        assert!(!e.invalidate_backend);
        e.error
    });
    assert!(matches!(
        result,
        Err(PlaybackSessionFailure::Session {
            code: PlaybackSessionFailureCode::Disposed,
            ..
        })
    ));
}

#[test]
fn validated_pixels_cannot_bypass_owner_plan_or_generation_fences() {
    for mode in ["valid", "other", "advance", "dispose", "cancel"] {
        let q = prepare();
        let binding = q["request"]["binding"].clone();
        let sample: PlaybackSampleRequest = serde_json::from_value(
            json!({"binding":binding,"at":{"ticks":"1","timescale":3},"history":null}),
        )
        .unwrap();
        let mut owner = PlaybackSession::default();
        let mut other = PlaybackSession::default();
        send(&mut owner, &q);
        send(&mut other, &q);
        let pending = owner.prepare_render(sample.clone(), &|| false).unwrap();
        let bytes = pending.words()[2] as usize * pending.words()[3] as usize * 4;
        let mut validation = mo_raster::RasterReplyValidation::new(
            mo_raster::BackendReply {
                status: 0,
                pixels: vec![0; bytes],
            },
            &|| false,
        )
        .unwrap();
        while !validation.step(1, &|| false).unwrap() {}
        let reply = validation.take().unwrap();
        if mode == "advance" {
            let generation = (binding["generation"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
                + 1)
            .to_string();
            send(
                &mut owner,
                &json!({"operation":"advance","binding":binding,"generation":generation}),
            );
        } else if mode == "dispose" {
            send(
                &mut owner,
                &json!({"operation":"dispose","binding":binding}),
            );
        }
        let receiver = if mode == "other" {
            &mut other
        } else {
            &mut owner
        };
        let result =
            receiver.complete_render_reply(pending, Ok(reply.into()), &|| mode == "cancel");
        if mode == "valid" {
            assert_eq!(result.unwrap().pixels.len(), bytes);
        } else {
            let error = result.err().unwrap();
            assert!(!error.invalidate_backend);
            let code = match mode {
                "dispose" => PlaybackSessionFailureCode::Disposed,
                "cancel" => PlaybackSessionFailureCode::Cancelled,
                _ => PlaybackSessionFailureCode::BindingConflict,
            };
            assert!(
                matches!(error.error, PlaybackSessionFailure::Session {code: found, ..} if found == code)
            );
        }
    }
}

#[path = "playback_session/viewport.rs"]
mod viewport;
