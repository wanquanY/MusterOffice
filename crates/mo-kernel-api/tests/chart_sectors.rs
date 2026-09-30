use mo_kernel_api::{
    ChartSectorFailureCode as Code, ChartSectorsResponse, layout_chart_sectors_json,
};
use serde_json::json;

fn input() -> serde_json::Value {
    json!({"startTurn":"0","direction":"clockwise","negativeWeights":"reject",
        "weights":[{"pointIndex":7,"value":"1"},{"pointIndex":42,"value":"2"}]})
}
fn response(value: serde_json::Value) -> ChartSectorsResponse {
    serde_json::from_str(&layout_chart_sectors_json(&value.to_string(), &|| false)).unwrap()
}
#[test]
fn public_response_carries_exact_boundaries_and_structured_failures() {
    let ChartSectorsResponse::Computed { layout } = response(input()) else {
        panic!("expected layout")
    };
    assert_eq!(layout.sectors[0].point_index, 7);
    assert_eq!(layout.sectors[0].end_turn.raw(), 1431655765);
    assert_eq!(layout.sectors[1].start_turn, layout.sectors[0].end_turn);
    assert_eq!(layout.sectors[1].end_turn.raw(), 4294967296);
    let mut negative = input();
    negative["weights"][1]["value"] = json!("-2");
    let ChartSectorsResponse::Error { error } = response(negative) else {
        panic!("expected error")
    };
    assert_eq!(error.code, Code::InputInvalid);
    assert_eq!(error.point_index, Some(42));
    for (json, check, code) in [
        ("{}", true, Code::Cancelled),
        ("{}", false, Code::InputInvalid),
    ] {
        let ChartSectorsResponse::Error { error } =
            serde_json::from_str(&layout_chart_sectors_json(json, &|| check)).unwrap()
        else {
            panic!("expected error")
        };
        assert_eq!(error.code, code);
    }
}
#[test]
fn wire_rejects_float_weights_unknown_fields_duplicate_keys_and_oversized_input() {
    let mut floating = input();
    floating["weights"][0]["value"] = json!(1.0);
    let mut unknown = input();
    unknown["skipInvalid"] = json!(true);
    for value in [floating, unknown] {
        assert!(matches!(
            response(value),
            ChartSectorsResponse::Error { .. }
        ));
    }
    assert!(matches!(
        serde_json::from_str::<ChartSectorsResponse>(&layout_chart_sectors_json(
            "{\"startTurn\":\"0\",\"startTurn\":\"1\"}",
            &|| false
        ))
        .unwrap(),
        ChartSectorsResponse::Error { .. }
    ));
    let oversized = " ".repeat(mo_kernel_api::MAX_REQUEST_BYTES + 1);
    let ChartSectorsResponse::Error { error } =
        serde_json::from_str(&layout_chart_sectors_json(&oversized, &|| false)).unwrap()
    else {
        panic!("expected limit")
    };
    assert_eq!(error.code, Code::LimitExceeded);
}
