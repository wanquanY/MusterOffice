use super::*;
use mo_common::RequestId;
use mo_presentation_edit::Snapshot;
use mo_presentation_operations::{ContractVersion, OperationProfile};
use serde_json::{Value, json};

fn request() -> (Request, RendererIdentity) {
    let input: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/presentations/delivery/input.json"
    ))
    .unwrap();
    let snapshot = Snapshot::new(
        serde_json::from_value(input["document"].clone()).unwrap(),
        Default::default(),
    )
    .unwrap()
    .into_record();
    let renderer = RendererIdentity {
        implementation_sha256: Digest::from_sha256([7; 32]),
        profile: "drawingml-resource-page-q32-v1-draft".into(),
    };
    let request = OperationRequest {
        contract_version: ContractVersion::V1,
        request_id: RequestId::new("export:wire-test").unwrap(),
        profile_id: OperationProfile::ResourceDelivery,
        action: DocumentAction::Export {
            document_id: snapshot.document.id.clone(),
            base_revision: snapshot.revision.clone(),
            settings: Box::new(mo_presentation_operations::ExportSettings {
                delivery: serde_json::from_value(input["settings"].clone()).unwrap(),
                resources: vec![],
                font_asset_id: None,
                renderer: renderer.clone(),
            }),
        },
    };
    (
        Request {
            version: VERSION.into(),
            request,
            snapshot,
            assets: vec![],
        },
        renderer,
    )
}

#[test]
fn serialization_stops_at_budget_including_escaped_bytes() {
    let value = json!({"text": "\n\t\"".repeat(2000)});
    let expected = serde_json::to_vec(&value).unwrap();
    assert!(encode_limited(&value, expected.len() - 1).is_err());
    let frame = encode_limited(&value, expected.len()).unwrap();
    assert_eq!(&frame[4..], expected);
    assert_eq!(
        u32::from_le_bytes(frame[..4].try_into().unwrap()) as usize,
        expected.len()
    );
    assert_eq!(read::<Value>(&mut frame.as_slice()).unwrap(), value);
}

#[test]
fn duplicate_keys_invalid_utf8_truncation_and_length_limits_fail_closed() {
    for bytes in [b"{\"outer\":{\"x\":1,\"x\":2}}".as_slice(), &[0xff], b"{"] {
        let mut framed = (bytes.len() as u32).to_le_bytes().to_vec();
        framed.extend_from_slice(bytes);
        assert!(read::<Value>(&mut framed.as_slice()).is_err());
    }
    let too_big = (MAX_METADATA as u32 + 1).to_le_bytes();
    assert!(read::<Value>(&mut too_big.as_slice()).is_err());
    for bytes in [b"\x01\x00\x00".as_slice(), b"\x02\x00\x00\x00{"] {
        assert!(read::<Value>(&mut &bytes[..]).is_err());
    }
}

#[test]
fn typed_frames_reject_paths_wrong_version_action_and_revision() {
    let (mut value, renderer) = request();
    value.validate(&renderer).unwrap();
    let mut json = serde_json::to_value(&value).unwrap();
    json["spoolDirectory"] = "some/path".into();
    assert!(read::<Request>(&mut encode(&json).unwrap().as_slice()).is_err());
    for version in ["unknown", "musteroffice.native-export/1-draft"] {
        value.version = version.into();
        assert_eq!(
            value.validate(&renderer).unwrap_err().code,
            FailureCode::ExecutorMismatch
        );
    }
    value.version = VERSION.into();
    value.snapshot.revision = Digest::from_sha256([0; 32]);
    assert_eq!(
        value.validate(&renderer).unwrap_err().code,
        FailureCode::RevisionConflict
    );
    value.request.action = DocumentAction::Create {
        document: Box::new(value.snapshot.document.clone()),
    };
    assert!(value.validate(&renderer).is_err());
}

#[test]
fn exact_asset_coverage_duplicate_ids_and_budgets_are_required() {
    use mo_common::ByteLength;
    use mo_presentation_operations::{AssetDescriptor, AssetVerification};
    let (mut value, renderer) = request();
    let asset = AssetInfo {
        id: AssetId::new("font:1").unwrap(),
        descriptor: AssetDescriptor {
            sha256: Digest::from_sha256([1; 32]),
            byte_length: ByteLength::new(0),
            media_type: "font/ttf".into(),
        },
        verification: AssetVerification::BytesSha256,
    };
    value.assets.push(asset.clone());
    assert!(value.validate(&renderer).is_err(), "unbound asset");
    let DocumentAction::Export { settings, .. } = &mut value.request.action else {
        unreachable!()
    };
    settings.font_asset_id = Some(asset.id.clone());
    value.validate(&renderer).unwrap();
    value.assets.push(asset);
    assert!(value.validate(&renderer).is_err(), "duplicate asset");
    value.assets.pop();
    value.assets[0].descriptor.byte_length = ByteLength::new(32 * 1024 * 1024 + 1);
    assert!(value.validate(&renderer).is_err(), "font budget");
    value.assets.clear();
    assert!(value.validate(&renderer).is_err(), "missing asset");
}

#[test]
fn prepared_response_uses_camel_case_request_digest() {
    let input: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/presentations/delivery-receive/request.json"
    ))
    .unwrap();
    let bundle = serde_json::from_value(input["bundle"].clone()).unwrap();
    let (q, _) = request();
    let response = Response::Prepared {
        request_digest: Digest::from_sha256([1; 32]),
        receipt: Box::new(ExportReceipt {
            document_id: q.snapshot.document.id,
            revision: q.snapshot.revision,
            semantic_digest: q.snapshot.semantic_digest,
            bundle,
        }),
    };
    let json = serde_json::to_value(&response).unwrap();
    assert!(json.get("requestDigest").is_some());
    assert!(json.get("request_digest").is_none());
    read::<Response>(&mut encode(&response).unwrap().as_slice()).unwrap();
}
