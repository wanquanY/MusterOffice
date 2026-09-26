use mo_kernel_api::*;

#[test]
fn source_bridge_preserves_nested_limit_and_cancel_categories() {
    let bytes = export_pptx_json(
        include_str!("../../../fixtures/presentations/native-export/request.json"),
        include_bytes!("../../../fixtures/presentations/native-export/resources.bin"),
    )
    .unwrap();
    let response = inspect_pptx(
        bytes.as_slice(),
        bytes.len() as u64,
        SourceLimits {
            max_text_bytes: 1,
            ..Default::default()
        },
        &|| false,
    );
    assert!(
        matches!(response, PptxSourceResponse::Error { error } if matches!(error.code, PptxFailureCode::LimitExceeded))
    );
    let response = inspect_pptx(
        bytes.as_slice(),
        bytes.len() as u64,
        SourceLimits::default(),
        &|| true,
    );
    assert!(
        matches!(response, PptxSourceResponse::Error { error } if matches!(error.code, PptxFailureCode::Cancelled))
    );
}

#[test]
fn color_bridge_preserves_cancel_limit_and_source_conflict_categories() {
    let bytes = export_pptx_json(
        include_str!("../../../fixtures/presentations/native-export/request.json"),
        include_bytes!("../../../fixtures/presentations/native-export/resources.bin"),
    )
    .unwrap();
    let PptxSourceResponse::Inspected { index } = inspect_pptx(
        bytes.as_slice(),
        bytes.len() as u64,
        SourceLimits::default(),
        &|| false,
    ) else {
        panic!("source inspection failed")
    };
    let mut request: SourceColorQuery = serde_json::from_value(serde_json::json!({
        "expectedSourceSha256": index.source_sha256, "surface": "/ppt/slides/slide1.xml", "profile": "ecma376-2016-draft-v1",
        "colors": ["accent1"], "context": { "systemColors": {}, "placeholder": null }
    })).unwrap();
    for (limits, cancel, expected) in [
        (ColorLimits::default(), true, "CANCELLED"),
        (
            ColorLimits {
                max_steps: 1,
                ..Default::default()
            },
            false,
            "LIMIT_EXCEEDED",
        ),
    ] {
        let response = resolve_pptx_colors(
            &request,
            bytes.as_slice(),
            bytes.len() as u64,
            SourceLimits::default(),
            limits,
            &|| cancel,
        );
        assert_eq!(
            serde_json::to_value(response).unwrap()["error"]["code"],
            expected
        );
    }
    request.expected_source_sha256 = mo_common::Digest::from_sha256([0; 32]);
    let response = resolve_pptx_colors_json(&serde_json::to_string(&request).unwrap(), &bytes);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&response).unwrap()["error"]["code"],
        "SOURCE_CONFLICT"
    );
}

#[test]
fn line_bridge_binds_real_source_and_preserves_failure_categories() {
    let bytes = export_pptx_json(
        include_str!("../../../fixtures/presentations/native-export/request.json"),
        include_bytes!("../../../fixtures/presentations/native-export/resources.bin"),
    )
    .unwrap();
    let PptxSourceResponse::Inspected { index } = inspect_pptx(
        bytes.as_slice(),
        bytes.len() as u64,
        SourceLimits::default(),
        &|| false,
    ) else {
        panic!("source inspection failed")
    };
    let mut request: SourceLineQuery = serde_json::from_value(serde_json::json!({
        "expectedSourceSha256":index.source_sha256,"surface":"/ppt/slides/slide1.xml",
        "objects":[index.surfaces["/ppt/slides/slide1.xml"].objects[0].native_id],"profile":"ms-oi29500-lines-2024-draft-v1"
    })).unwrap();
    for (limit, cancel, expected) in [
        (LineResolveLimits::default(), true, "CANCELLED"),
        (
            LineResolveLimits {
                max_steps: 0,
                ..Default::default()
            },
            false,
            "LIMIT_EXCEEDED",
        ),
    ] {
        let r = resolve_pptx_lines(
            &request,
            bytes.as_slice(),
            bytes.len() as u64,
            SourceLimits::default(),
            limit,
            &|| cancel,
        );
        assert_eq!(serde_json::to_value(r).unwrap()["error"]["code"], expected);
    }
    let r = resolve_pptx_lines_json(&serde_json::to_string(&request).unwrap(), &bytes);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&r).unwrap()["status"],
        "evaluated"
    );
    request.expected_source_sha256 = mo_common::Digest::from_sha256([0; 32]);
    let r = resolve_pptx_lines_json(&serde_json::to_string(&request).unwrap(), &bytes);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&r).unwrap()["error"]["code"],
        "SOURCE_CONFLICT"
    );
    assert!(serde_json::from_str::<SourceLineQuery>(r#"{"expectedSourceSha256":"0000000000000000000000000000000000000000000000000000000000000000","surface":"/ppt/slides/slide1.xml","objects":[],"profile":"unknown"}"#).is_err());
}

#[test]
fn line_color_bridge_binds_real_source_and_preserves_failure_categories() {
    let bytes = export_pptx_json(
        include_str!("../../../fixtures/presentations/native-export/request.json"),
        include_bytes!("../../../fixtures/presentations/native-export/resources.bin"),
    )
    .unwrap();
    let PptxSourceResponse::Inspected { index } = inspect_pptx(
        bytes.as_slice(),
        bytes.len() as u64,
        SourceLimits::default(),
        &|| false,
    ) else {
        panic!("source inspection failed")
    };
    let mut request: SourceLineColorQuery = serde_json::from_value(serde_json::json!({
        "expectedSourceSha256":index.source_sha256,"surface":"/ppt/slides/slide1.xml",
        "objects":[index.surfaces["/ppt/slides/slide1.xml"].objects[0].native_id],"lineProfile":"ms-oi29500-lines-2024-draft-v1","colorProfile":"ecma376-2016-draft-v1","context":{"systemColors":{},"placeholder":null}
    })).unwrap();
    for (limit, cancel, expected) in [
        (LineColorLimits::default(), true, "CANCELLED"),
        (
            LineColorLimits {
                colors: ColorLimits {
                    max_queries: 0,
                    ..Default::default()
                },
                ..Default::default()
            },
            false,
            "LIMIT_EXCEEDED",
        ),
    ] {
        let r = resolve_pptx_line_colors(
            &request,
            bytes.as_slice(),
            bytes.len() as u64,
            SourceLimits::default(),
            limit,
            &|| cancel,
        );
        assert_eq!(serde_json::to_value(r).unwrap()["error"]["code"], expected);
    }
    let r = resolve_pptx_line_colors_json(&serde_json::to_string(&request).unwrap(), &bytes);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&r).unwrap()["status"],
        "evaluated"
    );
    request.expected_source_sha256 = mo_common::Digest::from_sha256([0; 32]);
    let r = resolve_pptx_line_colors_json(&serde_json::to_string(&request).unwrap(), &bytes);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&r).unwrap()["error"]["code"],
        "SOURCE_CONFLICT"
    );
    assert!(serde_json::from_str::<SourceLineColorQuery>(r#"{"expectedSourceSha256":"0000000000000000000000000000000000000000000000000000000000000000","surface":"/ppt/slides/slide1.xml","objects":[],"profile":"unknown"}"#).is_err());
}

#[test]
fn geometry_bridge_preserves_source_failure_and_cancellation_contracts() {
    let bytes = export_pptx_json(
        include_str!("../../../fixtures/presentations/native-export/request.json"),
        include_bytes!("../../../fixtures/presentations/native-export/resources.bin"),
    )
    .unwrap();
    let PptxSourceResponse::Inspected { index } = inspect_pptx(
        bytes.as_slice(),
        bytes.len() as u64,
        SourceLimits::default(),
        &|| false,
    ) else {
        panic!("source inspection failed")
    };
    let mut request: SourceGeometryQuery = serde_json::from_value(serde_json::json!({
        "expectedSourceSha256":index.source_sha256,"surface":"/ppt/slides/slide1.xml",
        "objects":[index.surfaces["/ppt/slides/slide1.xml"].objects[0].native_id],"profile":"ecma376-2016-ms-presets-draft-v2"
    })).unwrap();
    for (limit, cancel, expected) in [
        (GeometryLimits::default(), true, "CANCELLED"),
        (
            GeometryLimits {
                max_queries: 0,
                ..Default::default()
            },
            false,
            "LIMIT_EXCEEDED",
        ),
    ] {
        let r = evaluate_pptx_geometry(
            &request,
            bytes.as_slice(),
            bytes.len() as u64,
            SourceLimits::default(),
            limit,
            &|| cancel,
        );
        assert_eq!(serde_json::to_value(r).unwrap()["error"]["code"], expected);
    }
    let r = evaluate_pptx_geometry_json(&serde_json::to_string(&request).unwrap(), &bytes);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&r).unwrap()["status"],
        "evaluated"
    );
    request.expected_source_sha256 = mo_common::Digest::from_sha256([0; 32]);
    let r = evaluate_pptx_geometry_json(&serde_json::to_string(&request).unwrap(), &bytes);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&r).unwrap()["error"]["code"],
        "SOURCE_CONFLICT"
    );
    assert!(serde_json::from_str::<SourceGeometryQuery>(r#"{"expectedSourceSha256":"0000000000000000000000000000000000000000000000000000000000000000","surface":"/ppt/slides/slide1.xml","objects":[],"profile":"unknown"}"#).is_err());
}

#[test]
fn native_paths_bridge_keeps_source_identity_and_atomic_budget_failures() {
    let bytes = export_pptx_json(
        include_str!("../../../fixtures/presentations/native-export/request.json"),
        include_bytes!("../../../fixtures/presentations/native-export/resources.bin"),
    )
    .unwrap();
    let PptxSourceResponse::Inspected { index } = inspect_pptx(
        bytes.as_slice(),
        bytes.len() as u64,
        SourceLimits::default(),
        &|| false,
    ) else {
        panic!()
    };
    let (surface, object) = index
        .surfaces
        .iter()
        .find_map(|(part, surface)| {
            surface
                .objects
                .iter()
                .find(|o| {
                    o.geometry.as_ref().is_some_and(|g| {
                        matches!(
                            g.definition,
                            mo_pptx::source::geometry::SourceGeometryDefinition::Custom(_)
                        )
                    })
                })
                .map(|object| (part, object))
        })
        .unwrap();
    let mut request:SourceNativePathsQuery=serde_json::from_value(serde_json::json!({"geometry":{"expectedSourceSha256":index.source_sha256,"surface":surface,"objects":[object.native_id,object.native_id],"profile":"ecma376-2016-ms-presets-draft-v2"},"options":{"profile":"drawingml-polar-arcs-q96-hermite-v1-draft","coordinateTolerance":"4294967296"}})).unwrap();
    let result = compile_pptx_paths_json(&serde_json::to_string(&request).unwrap(), &bytes);
    let r: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(r["status"], "compiled");
    assert_eq!(r["paths"]["objects"][0], r["paths"]["objects"][1]);
    assert_eq!(r["paths"]["objects"][0]["outcome"]["status"], "compiled");
    for (limits, cancel, code) in [
        (
            NativePathLimits {
                max_commands: 0,
                ..Default::default()
            },
            false,
            "LIMIT_EXCEEDED",
        ),
        (NativePathLimits::default(), true, "CANCELLED"),
    ] {
        let r = compile_pptx_paths(
            &request,
            bytes.as_slice(),
            bytes.len() as u64,
            SourceLimits::default(),
            GeometryLimits::default(),
            limits,
            &|| cancel,
        );
        let value = serde_json::to_value(r).unwrap();
        assert_eq!(value["status"], "error");
        assert_eq!(value["error"]["code"], code);
    }
    request.geometry.expected_source_sha256 = mo_common::Digest::from_sha256([0; 32]);
    let r: serde_json::Value = serde_json::from_str(&compile_pptx_paths_json(
        &serde_json::to_string(&request).unwrap(),
        &bytes,
    ))
    .unwrap();
    assert_eq!(r["error"]["code"], "SOURCE_CONFLICT");
}

#[test]
fn fill_color_bridge_binds_real_source_and_preserves_failure_categories() {
    let bytes = export_pptx_json(
        include_str!("../../../fixtures/presentations/native-export/request.json"),
        include_bytes!("../../../fixtures/presentations/native-export/resources.bin"),
    )
    .unwrap();
    let PptxSourceResponse::Inspected { index } = inspect_pptx(
        bytes.as_slice(),
        bytes.len() as u64,
        SourceLimits::default(),
        &|| false,
    ) else {
        panic!("source inspection failed")
    };
    let mut request: SourceFillColorQuery = serde_json::from_value(serde_json::json!({
        "expectedSourceSha256":index.source_sha256,"surface":"/ppt/slides/slide1.xml",
        "targets":[{"kind":"object","nativeId":index.surfaces["/ppt/slides/slide1.xml"].objects[0].native_id}],"fillProfile":"ms-oi29500-fills-2024-draft-v1","colorProfile":"ecma376-2016-draft-v1","context":{"systemColors":{},"placeholder":null}
    })).unwrap();
    for (limit, cancel, expected) in [
        (FillColorLimits::default(), true, "CANCELLED"),
        (
            FillColorLimits {
                fills: mo_pptx::source::fill::resolve::FillResolveLimits {
                    max_queries: 0,
                    ..Default::default()
                },
                ..Default::default()
            },
            false,
            "LIMIT_EXCEEDED",
        ),
    ] {
        let r = resolve_pptx_fill_colors(
            &request,
            bytes.as_slice(),
            bytes.len() as u64,
            SourceLimits::default(),
            limit,
            &|| cancel,
        );
        assert_eq!(serde_json::to_value(r).unwrap()["error"]["code"], expected);
    }
    let r = resolve_pptx_fill_colors_json(&serde_json::to_string(&request).unwrap(), &bytes);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&r).unwrap()["status"],
        "evaluated"
    );
    request.expected_source_sha256 = mo_common::Digest::from_sha256([0; 32]);
    let r = resolve_pptx_fill_colors_json(&serde_json::to_string(&request).unwrap(), &bytes);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&r).unwrap()["error"]["code"],
        "SOURCE_CONFLICT"
    );
    assert!(serde_json::from_str::<SourceFillColorQuery>(r#"{"expectedSourceSha256":"0000000000000000000000000000000000000000000000000000000000000000","surface":"/ppt/slides/slide1.xml","objects":[],"profile":"unknown"}"#).is_err());
}
