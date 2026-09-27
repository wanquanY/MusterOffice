use super::*;
use mo_common::{DocumentId, OperationId, RequestId, ResourceId};
use mo_embedded_sdk::{ExportOptions, Inputs, Presentation};
use mo_presentation_edit::{Operation, OperationEntry};
use mo_presentation_operations::{
    AssetDescriptor, AssetId, AssetInfo, AssetVerification, ExportAsset,
};

fn options(request: &mo_presentation_operations::OperationRequest) -> ExportOptions {
    let DocumentAction::Export { settings, .. } = &request.action else {
        unreachable!()
    };
    ExportOptions {
        delivery: settings.delivery.clone(),
        resources: settings.resources.clone(),
        font_asset_id: settings.font_asset_id.clone(),
    }
}

#[test]
fn sdk_creates_edits_exports_and_imports_real_bytes_without_an_owner_service() {
    let root = Root::new();
    let exporter = exporter(&root);
    let (snapshot, request, source) = input(exporter.renderer_identity());
    let mut inputs = Inputs::new();
    for (info, reader) in &source.0 {
        inputs
            .insert_bytes(
                info.id.clone(),
                info.descriptor.media_type.clone(),
                &reader.bytes,
                &|| false,
            )
            .unwrap();
    }
    let mut deck = Presentation::create(snapshot.document.clone(), &|| false).unwrap();
    assert_eq!(deck.snapshot(), &snapshot);
    let result = deck
        .export(
            &exporter,
            request.request_id.clone(),
            options(&request),
            &inputs,
            &|| false,
        )
        .unwrap();
    let direct = exporter
        .prepare(&request, snapshot, &source, &|| false)
        .unwrap();
    assert_eq!(
        serde_json::to_value(result.receipt()).unwrap(),
        serde_json::to_value(direct.receipt()).unwrap()
    );
    for asset in result.assets() {
        assert_eq!(bytes(&result, &asset.id), bytes(&direct, &asset.id));
    }
    let pptx = result
        .assets()
        .iter()
        .find(|a| a.role == AssetRole::Pptx)
        .unwrap();
    let source = result.open(&pptx.id).unwrap();
    let info = AssetInfo {
        id: AssetId::new("source").unwrap(),
        descriptor: AssetDescriptor {
            sha256: pptx.sha256.clone(),
            byte_length: pptx.byte_length,
            media_type: pptx.media_type.clone(),
        },
        verification: AssetVerification::BytesSha256,
    };
    let imported = Presentation::import(
        DocumentId::new("imported").unwrap(),
        ResourceId::new("source").unwrap(),
        ExportAsset {
            info: &info,
            reader: source.reader,
        },
        &|| false,
    )
    .unwrap();
    assert!(imported.document().source_bindings.is_some());
    assert_eq!(imported.document().slide_order.len(), 2);
    deck.edit(
        RequestId::new("edit").unwrap(),
        vec![OperationEntry {
            operation_id: OperationId::new("title").unwrap(),
            operation: Operation::SetTitle {
                title: "SDK revision".into(),
            },
        }],
        &|| false,
    )
    .unwrap();
    let edited = deck
        .export(
            &exporter,
            RequestId::new("export-edited").unwrap(),
            options(&request),
            &inputs,
            &|| false,
        )
        .unwrap();
    assert_eq!(edited.receipt().revision, deck.snapshot().revision);
    assert_ne!(edited.receipt().revision, result.receipt().revision);
    assert_eq!(edited.assets().len(), 12);
    drop((result, direct, edited, imported));
    root.clean();
}
