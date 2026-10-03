mod support;
use mo_common::*;
use mo_opc::{Package, PartName, RewritePlan};
use mo_pptx::{
    source::{document::*, *},
    *,
};
use mo_presentation_edit::{Operation, OperationEntry, Snapshot, Transaction, prepare};
use mo_presentation_model::*;

fn fixture() -> Vec<u8> {
    let (d, defaults) = support::input();
    export(
        &d,
        &defaults,
        &support::resources(),
        Default::default(),
        &|| false,
    )
    .unwrap()
}
fn package(bytes: &[u8]) -> Package<&[u8]> {
    Package::open(bytes, bytes.len() as u64, Default::default(), &|| false).unwrap()
}
fn import(source: &dyn mo_opc::PackageRead) -> Document {
    import_document(
        source,
        DocumentId::new("imported").unwrap(),
        ResourceId::new("original").unwrap(),
        Default::default(),
        &|| false,
    )
    .unwrap()
}
fn target(d: &Document) -> (ObjectId, ParagraphId, RunId) {
    let id = d.objects.iter().find(|(_, o)| matches!(&o.parent, ContainerId::Slide(_)) && matches!(&o.content,
        ObjectContent::RetainedSource { paragraphs, .. } if !paragraphs.is_empty() && !paragraphs[0].runs.is_empty()) && o.transform.is_some()).unwrap().0.clone();
    let ObjectContent::RetainedSource { paragraphs, .. } = &d.objects[&id].content else {
        unreachable!()
    };
    (
        id,
        paragraphs[0].id.clone(),
        paragraphs[0].runs[0].id.clone(),
    )
}
fn transaction(s: &Snapshot, operations: Vec<Operation>) -> Transaction {
    Transaction {
        document_id: s.document().id.clone(),
        base_revision: s.revision().clone(),
        request_id: RequestId::new("edit").unwrap(),
        operations: operations
            .into_iter()
            .enumerate()
            .map(|(i, operation)| OperationEntry {
                operation_id: OperationId::new(format!("op{i}")).unwrap(),
                operation,
            })
            .collect(),
    }
}
fn change(bytes: &[u8], mutate: impl FnOnce(String) -> String) -> Vec<u8> {
    let p = package(bytes);
    let part = PartName::new("/ppt/slides/slide1.xml").unwrap();
    let xml = String::from_utf8(p.read_part(&part, 1 << 20, &|| false).unwrap()).unwrap();
    let mut plan = RewritePlan::new();
    plan.replace_part(part, mutate(xml).into_bytes()).unwrap();
    plan.to_bytes(&p, &|| false).unwrap()
}
#[test]
fn native_title_is_revisioned_and_writes_only_core_properties() {
    let bytes = fixture();
    let p = package(&bytes);
    let d = import(&p);
    assert_eq!(d.title, support::input().0.title);
    assert!(!d.title.is_empty());
    assert_eq!(
        d.source_bindings.as_ref().unwrap().profile,
        SourceBindingProfile::PresentationmlRetainedFieldsV4
    );
    let snapshot = Snapshot::new(d, Default::default()).unwrap();
    let title = "标题 <&>\r\n🚀";
    let result = prepare(
        &snapshot,
        &transaction(
            &snapshot,
            vec![Operation::SetTitle {
                title: title.into(),
            }],
        ),
        Default::default(),
    )
    .unwrap();
    assert_ne!(result.snapshot.revision(), snapshot.revision());
    let output = SourcePlan::new(result.snapshot.document(), &p, Default::default(), &|| {
        false
    })
    .unwrap()
    .write(&p, &|| false)
    .unwrap();
    let actual = package(&output);
    assert_eq!(import(&actual).title, title);
    assert_eq!(actual.relationships(), p.relationships());
    let core = mo_opc::read_core_properties(&p, Default::default(), &|| false)
        .unwrap()
        .unwrap()
        .part;
    for (part, info) in p.parts() {
        if *part == core {
            assert_ne!(info.sha256, actual.parts()[part].sha256);
        } else {
            assert_eq!(info.sha256, actual.parts()[part].sha256, "{part}");
        }
    }
    // Historical V1 never projected title. Loading one must not interpret its
    // empty domain field as a request to erase the source metadata.
    let mut legacy = snapshot.document().clone();
    legacy.source_bindings.as_mut().unwrap().profile =
        SourceBindingProfile::PresentationmlRetainedFieldsV1;
    legacy.title.clear();
    for object in legacy.objects.values_mut() {
        object.accessibility = Default::default();
    }
    assert_eq!(
        SourcePlan::new(&legacy, &p, Default::default(), &|| false)
            .unwrap()
            .write(&p, &|| false)
            .unwrap(),
        bytes
    );
    let legacy = Snapshot::new(legacy, Default::default()).unwrap();
    assert!(
        prepare(
            &legacy,
            &transaction(
                &legacy,
                vec![Operation::SetTitle {
                    title: "cannot reinterpret V1".into()
                }]
            ),
            Default::default()
        )
        .is_err()
    );
}
#[test]
fn text_and_transform_share_revision_and_one_preserving_candidate() {
    let bytes = change(&fixture(), |s| {
        s.replace("</p:sld>", "<p:extLst><p:ext uri=\"owned-test\"><x:opaque xmlns:x=\"urn:owned\" value=\"keep &amp; preserve\"/></p:ext></p:extLst></p:sld>")
    });
    let original = package(&bytes);
    let d = import(&original);
    assert_eq!(d, import(&original));
    let (id, paragraph, run) = target(&d);
    let binding = d.source_bindings.as_ref().unwrap().objects[&id].clone();
    let mut transform = d.objects[&id].transform.unwrap();
    transform.origin.x = Emu::new(101);
    let s = Snapshot::new(d, Default::default()).unwrap();
    let tx = transaction(
        &s,
        vec![
            Operation::SpliceText {
                object: id.clone(),
                paragraph: paragraph.clone(),
                run,
                start: 0,
                delete: 0,
                insert: "😀<&".into(),
            },
            Operation::SetTransform {
                object: id.clone(),
                transform,
            },
        ],
    );
    let result = prepare(&s, &tx, Default::default()).unwrap();
    assert_ne!(result.snapshot.revision(), s.revision());
    assert_eq!(
        result.receipt.changes.updated_objects,
        std::slice::from_ref(&id)
    );
    assert_eq!(result.receipt.changes.anchor_maps[0].inserted, 3);
    assert_eq!(result.receipt.changes.invalidated_slides.len(), 1);
    assert_eq!(
        result.receipt.changes.invalidated_slides[0],
        s.document().slide_order[0]
    );
    assert!(prepare(&result.snapshot, &tx, Default::default()).is_err());
    let plan = SourcePlan::new(
        result.snapshot.document(),
        &original,
        Default::default(),
        &|| false,
    )
    .unwrap();
    assert_ne!(plan.identity(), original.sha256());
    let output = plan.write(&original, &|| false).unwrap();
    let actual = package(&output);
    let observed = inspect_source(&actual, Default::default(), &|| false).unwrap();
    let native = observed.surfaces[&binding.part]
        .objects
        .iter()
        .find(|o| o.native_id == binding.native_id)
        .unwrap();
    assert!(native.paragraphs[0][0].text.starts_with("😀<&"));
    assert_eq!(
        native.transform.as_ref().unwrap().origin.unwrap().x,
        Emu::new(101)
    );
    assert_eq!(actual.relationships(), original.relationships());
    for part in original.parts().keys() {
        let before = original.read_part(part, 1 << 24, &|| false).unwrap();
        let after = actual.read_part(part, 1 << 24, &|| false).unwrap();
        if part.as_str() != binding.part {
            assert_eq!(before, after, "{part}");
        } else {
            assert!(
                String::from_utf8(after)
                    .unwrap()
                    .contains("value=\"keep &amp; preserve\"")
            );
        }
    }
    let unchanged =
        SourcePlan::new(s.document(), &original, Default::default(), &|| false).unwrap();
    assert_eq!(unchanged.write(&original, &|| false).unwrap(), bytes);
}
#[test]
fn structural_or_forged_source_edits_fail_and_do_not_change_the_snapshot() {
    let bytes = fixture();
    let p = package(&bytes);
    let d = import(&p);
    let (id, paragraph, run) = target(&d);
    let before = d.semantic_digest().unwrap();
    let s = Snapshot::new(d.clone(), Default::default()).unwrap();
    let tx = transaction(
        &s,
        vec![
            Operation::SpliceText {
                object: id.clone(),
                paragraph,
                run,
                start: 0,
                delete: 0,
                insert: "prefix".into(),
            },
            Operation::SetAppearance {
                object: id.clone(),
                appearance: Default::default(),
            },
        ],
    );
    assert!(prepare(&s, &tx, Default::default()).is_err());
    assert_eq!(s.semantic_digest(), &before);
    let mut forged = d.clone();
    forged
        .source_bindings
        .as_mut()
        .unwrap()
        .objects
        .get_mut(&id)
        .unwrap()
        .native_id += 999;
    assert!(SourcePlan::new(&forged, &p, Default::default(), &|| false).is_err());
    let mut changed = d;
    changed.source_bindings.as_mut().unwrap().profile =
        SourceBindingProfile::PresentationmlRetainedFieldsV1;
    changed.title = "unmapped".into();
    assert!(SourcePlan::new(&changed, &p, Default::default(), &|| false).is_err());
    assert!(SourcePlan::new(s.document(), &p, Default::default(), &|| true).is_err());
    let different = change(&bytes, |s| {
        s.replace("</p:sld>", "<!--different--></p:sld>")
    });
    assert!(
        SourcePlan::new(
            s.document(),
            &package(&different),
            Default::default(),
            &|| false
        )
        .is_err()
    );
}
#[test]
fn absent_native_transform_stays_absent_and_unsafe_text_is_protected() {
    let bytes = change(&fixture(), |mut s| {
        let start = s.find("<a:xfrm").unwrap();
        let end = start + s[start..].find("</a:xfrm>").unwrap() + "</a:xfrm>".len();
        s.replace_range(start..end, "");
        s
    });
    let p = package(&bytes);
    let d = import(&p);
    let object = d
        .objects
        .values()
        .find(|o| o.transform.is_none() && matches!(o.parent, ContainerId::Slide(_)))
        .unwrap();
    assert_eq!(
        d.source_bindings.as_ref().unwrap().objects[&object.id].transform_constraint,
        Some(NativeEditConstraint::MissingDirectTransform)
    );
    let plan = SourcePlan::new(&d, &p, Default::default(), &|| false).unwrap();
    assert_eq!(plan.write(&p, &|| false).unwrap(), bytes);
    let original = fixture();
    let bytes = change(&original, |s| {
        s.replacen("<a:t>", "<a:t xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\" xmlns:q=\"urn:ignored\" mc:Ignorable=\"q\"><q:metadata/>", 1)
    });
    let p = package(&bytes);
    let d = import(&p);
    let constrained = d
        .source_bindings
        .as_ref()
        .unwrap()
        .objects
        .iter()
        .find_map(|(o, b)| {
            b.runs
                .iter()
                .find(|(_, r)| r.constraint == Some(NativeEditConstraint::StructuredLeaf))
                .map(|(r, b)| (o.clone(), r.clone(), b.paragraph))
        })
        .unwrap();
    let ObjectContent::RetainedSource { paragraphs, .. } = &d.objects[&constrained.0].content
    else {
        unreachable!()
    };
    let paragraph = paragraphs[constrained.2 as usize].id.clone();
    let s = Snapshot::new(d, Default::default()).unwrap();
    let tx = transaction(
        &s,
        vec![Operation::SpliceText {
            object: constrained.0,
            paragraph,
            run: constrained.1,
            start: 0,
            delete: 0,
            insert: "blocked".into(),
        }],
    );
    assert!(prepare(&s, &tx, Default::default()).is_err());
}

#[test]
fn source_transaction_rejects_unwritable_coordinates_and_text_before_commit() {
    let bytes = fixture();
    let p = package(&bytes);
    let d = import(&p);
    let (id, paragraph, run) = target(&d);
    let mut transform = d.objects[&id].transform.unwrap();
    transform.origin.x = Emu::new(PRESENTATIONML_COORDINATE_MAX + 1);
    let s = Snapshot::new(d, Default::default()).unwrap();
    for operation in [
        Operation::SetTransform {
            object: id.clone(),
            transform,
        },
        Operation::SpliceText {
            object: id,
            paragraph,
            run,
            start: 0,
            delete: 0,
            insert: "\0".into(),
        },
    ] {
        assert!(prepare(&s, &transaction(&s, vec![operation]), Default::default()).is_err());
    }
}

#[test]
fn high_level_native_range_roundtrips_with_opaque_parts_and_original_run_structure() {
    use mo_presentation_edit::{TextEditAction, TextEditCommand, TextSelection, prepare_text_edit};
    let (mut d, defaults) = support::input();
    let first_slide = d.slide_order[0].clone();
    let body = d
        .objects
        .values_mut()
        .filter(|o| o.parent == ContainerId::Slide(first_slide.clone()))
        .find_map(|o| match &mut o.content {
            ObjectContent::Shape {
                text: Some(body), ..
            } => Some(body),
            _ => None,
        })
        .unwrap();
    let style = body.paragraphs[0].runs[0].style.clone();
    body.paragraphs[0].runs = ["A😀", "e", "\u{301}中"]
        .into_iter()
        .enumerate()
        .map(|(i, text)| TextRun {
            id: RunId::new(format!("range:owned:{i}")).unwrap(),
            style: style.clone(),
            content: InlineContent::Text { text: text.into() },
        })
        .collect();
    let raw = export(
        &d,
        &defaults,
        &support::resources(),
        Default::default(),
        &|| false,
    )
    .unwrap();
    let bytes = change(&raw, |s| {
        s.replace("</p:sld>", "<p:extLst><p:ext uri=\"owned-range\"><x:opaque xmlns:x=\"urn:owned\" value=\"keep &amp; preserve\"/></p:ext></p:extLst></p:sld>")
    });
    let original = package(&bytes);
    let d = import(&original);
    let (id, paragraph) = d
        .objects
        .iter()
        .find_map(|(id, o)| match &o.content {
            ObjectContent::RetainedSource { paragraphs, .. } => paragraphs
                .iter()
                .find(|p| {
                    p.runs.iter().map(|r| r.text.as_str()).collect::<String>() == "A😀e\u{301}中"
                })
                .map(|p| (id.clone(), p.id.clone())),
            _ => None,
        })
        .unwrap();
    let binding = d.source_bindings.as_ref().unwrap().objects[&id].clone();
    let s = Snapshot::new(d, Default::default()).unwrap();
    let anchor = TextAnchor {
        paragraph: paragraph.clone(),
        scalar_offset: 1,
        affinity: Affinity::After,
    };
    let command = TextEditCommand {
        document_id: s.document().id.clone(),
        base_revision: s.revision().clone(),
        request_id: RequestId::new("range:roundtrip").unwrap(),
        operation_id: OperationId::new("range:operation").unwrap(),
        object: id.clone(),
        cell: None,
        action: TextEditAction::Replace {
            selection: TextSelection {
                anchor: anchor.clone(),
                focus: TextAnchor {
                    scalar_offset: 4,
                    ..anchor
                },
            },
            text: "α<&\t🚀".into(),
        },
    };
    let result = prepare_text_edit(&s, &command, Default::default(), &|| false).unwrap();
    let plan = SourcePlan::new(
        result.prepared.snapshot.document(),
        &original,
        Default::default(),
        &|| false,
    )
    .unwrap();
    let output = plan.write(&original, &|| false).unwrap();
    let actual = package(&output);
    assert_eq!(actual.relationships(), original.relationships());
    assert_eq!(
        actual.parts().keys().collect::<Vec<_>>(),
        original.parts().keys().collect::<Vec<_>>()
    );
    for part in original.parts().keys() {
        let before = original.read_part(part, 1 << 24, &|| false).unwrap();
        let after = actual.read_part(part, 1 << 24, &|| false).unwrap();
        if part.as_str() != binding.part {
            assert_eq!(before, after, "{part}");
        } else {
            assert!(
                String::from_utf8(after)
                    .unwrap()
                    .contains("value=\"keep &amp; preserve\"")
            );
        }
    }
    let restored = import(&actual);
    let restored_id = restored
        .source_bindings
        .as_ref()
        .unwrap()
        .objects
        .iter()
        .find(|(_, b)| b.part == binding.part && b.native_id == binding.native_id)
        .unwrap()
        .0;
    let ObjectContent::RetainedSource { paragraphs, .. } = &restored.objects[restored_id].content
    else {
        panic!()
    };
    assert!(
        paragraphs
            .iter()
            .any(|p| p.runs.iter().map(|r| r.text.as_str()).collect::<String>() == "Aα<&\t🚀中")
    );
    assert_eq!(
        result.prepared.snapshot.document().source_bindings,
        s.document().source_bindings
    );
    let undo = mo_presentation_edit::prepare_history(
        &result.prepared.snapshot,
        &mo_presentation_edit::HistoryTransaction {
            document_id: s.document().id.clone(),
            base_revision: result.prepared.snapshot.revision().clone(),
            request_id: RequestId::new("roundtrip:undo").unwrap(),
            original_snapshot: s.clone().into_record(),
            original_transaction: result.transaction,
            direction: mo_presentation_edit::HistoryDirection::Undo,
        },
        Default::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(
        SourcePlan::new(
            undo.snapshot.document(),
            &original,
            Default::default(),
            &|| false
        )
        .unwrap()
        .write(&original, &|| false)
        .unwrap(),
        bytes
    );
}
