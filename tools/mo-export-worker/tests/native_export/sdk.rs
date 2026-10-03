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

#[test]
fn imported_pptx_append_renders_and_exports_the_same_preserved_pages() {
    use mo_common::{ObjectId, SlideId};
    use mo_embedded_sdk::model::{ContainerId, ObjectContent};
    use mo_presentation_operations::ExportAssets;
    let root = Root::new();
    let exporter = exporter(&root);
    let (snapshot, request, source) = input(exporter.renderer_identity());
    let candidate = exporter
        .prepare(&request, snapshot.clone(), &source, &|| false)
        .unwrap();
    let pptx = candidate
        .assets()
        .iter()
        .find(|a| a.role == AssetRole::Pptx)
        .unwrap();
    let original = bytes(&candidate, &pptx.id);
    let source_asset = AssetId::new("source-pptx").unwrap();
    let source_resource = ResourceId::new("source-pptx").unwrap();
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
    inputs
        .insert_bytes(
            source_asset.clone(),
            pptx.media_type.clone(),
            &original,
            &|| false,
        )
        .unwrap();
    let mut imported = Presentation::import(
        DocumentId::new("source-append").unwrap(),
        source_resource.clone(),
        inputs.get(&source_asset).unwrap(),
        &|| false,
    )
    .unwrap();
    let mut export_options = options(&request);
    export_options.resources = vec![mo_presentation_operations::AssetBinding {
        resource_id: source_resource,
        asset_id: source_asset,
    }];
    let before_options = ExportOptions {
        delivery: export_options.delivery.clone(),
        resources: export_options.resources.clone(),
        font_asset_id: export_options.font_asset_id.clone(),
    };
    let before = imported
        .export(
            &exporter,
            RequestId::new("before").unwrap(),
            before_options,
            &inputs,
            &|| false,
        )
        .unwrap();
    let page_id = SlideId::new("added-page").unwrap();
    let mut slide = snapshot.document.slides[&snapshot.document.slide_order[0]].clone();
    slide.id = page_id.clone();
    slide.layout = None;
    slide.objects.clear();
    slide.name = "New page".into();
    let mut object = snapshot
        .document
        .objects
        .values()
        .find(|o| matches!(o.content, ObjectContent::Shape { text: Some(_), .. }))
        .unwrap()
        .clone();
    object.id = ObjectId::new("added-text").unwrap();
    object.parent = ContainerId::Slide(page_id);
    imported
        .edit(
            RequestId::new("append").unwrap(),
            vec![
                OperationEntry {
                    operation_id: OperationId::new("page").unwrap(),
                    operation: Operation::InsertSlide { slide, index: 2 },
                },
                OperationEntry {
                    operation_id: OperationId::new("text").unwrap(),
                    operation: Operation::InsertObject { object, index: 0 },
                },
            ],
            &|| false,
        )
        .unwrap();
    let after = imported
        .export(
            &exporter,
            RequestId::new("after").unwrap(),
            export_options,
            &inputs,
            &|| false,
        )
        .unwrap();
    assert_eq!(after.receipt().bundle.previews.len(), 3);
    for index in 0..2 {
        let old = &before.receipt().bundle.previews[index].image_asset_id;
        let new = &after.receipt().bundle.previews[index].image_asset_id;
        assert_eq!(
            bytes(&before, old),
            bytes(&after, new),
            "retained visual page {index}"
        );
    }
    let exported = after
        .assets()
        .iter()
        .find(|a| a.role == AssetRole::Pptx)
        .unwrap();
    assert!(exported.byte_length.get() > original.len() as u64);
    assert_eq!(after.receipt().revision, imported.snapshot().revision);
    drop((before, after, candidate));
    root.clean();
}
