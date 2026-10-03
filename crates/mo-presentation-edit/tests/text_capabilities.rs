#[allow(dead_code)]
mod support;
use mo_common::*;
use mo_presentation_edit::*;
use mo_presentation_model::*;
use support::*;

fn query(selection: Option<TextSelection>) -> TextCapabilitiesQuery {
    TextCapabilitiesQuery {
        object: id(),
        cell: None,
        selection,
    }
}
fn selection(end: u32) -> TextSelection {
    let a = TextAnchor {
        paragraph: paragraph_id(),
        scalar_offset: 0,
        affinity: Affinity::After,
    };
    TextSelection {
        anchor: a.clone(),
        focus: TextAnchor {
            scalar_offset: end,
            ..a
        },
    }
}
fn reason(value: TextEditAvailability) -> TextEditRestriction {
    let TextEditAvailability::Unavailable { reason } = value else {
        panic!()
    };
    reason
}
#[test]
fn authored_capabilities_follow_selection_and_missing_body_without_creating_revision() {
    let s = Snapshot::new(document(), Default::default()).unwrap();
    let before = s.clone().into_record();
    let caps = text_capabilities(&s, &query(None), &|| false).unwrap();
    assert_eq!(caps.revision, *s.revision());
    assert_eq!(caps.document_id, s.document().id);
    assert_eq!(
        caps.replacement_policy,
        Some(TextReplacementPolicy::AuthoredBody)
    );
    assert_eq!(reason(caps.initialize), TextEditRestriction::TextBodyExists);
    assert_eq!(reason(caps.replace), TextEditRestriction::SelectionRequired);
    let caps = text_capabilities(&s, &query(Some(selection(0))), &|| false).unwrap();
    assert_eq!(caps.replace, TextEditAvailability::Available);
    assert_eq!(
        reason(caps.character_style),
        TextEditRestriction::NonemptySelectionRequired
    );
    let caps = text_capabilities(&s, &query(Some(selection(2))), &|| false).unwrap();
    assert_eq!(caps.character_style, TextEditAvailability::Available);
    assert_eq!(caps.delete, TextEditAvailability::Available);
    assert!(text_capabilities(&s, &query(Some(selection(3))), &|| false).is_err());
    assert_eq!(s.clone().into_record(), before);
    let mut d = s.document().clone();
    let ObjectContent::Shape { text, .. } = &mut d.objects.get_mut(&id()).unwrap().content else {
        panic!()
    };
    *text = None;
    let s = Snapshot::new(d, Default::default()).unwrap();
    let caps = text_capabilities(&s, &query(None), &|| false).unwrap();
    assert_eq!(caps.initialize, TextEditAvailability::Available);
    assert_eq!(reason(caps.replace), TextEditRestriction::MissingTextBody);
}

#[test]
fn non_text_and_covered_cells_have_structured_capabilities_and_malformed_scope_is_an_error() {
    let mut d = document();
    let object = connector(&mut d);
    let s = Snapshot::new(d, Default::default()).unwrap();
    let caps = text_capabilities(
        &s,
        &TextCapabilitiesQuery {
            object,
            ..query(None)
        },
        &|| false,
    )
    .unwrap();
    assert_eq!(reason(caps.replace), TextEditRestriction::UnsupportedTarget);
    assert_eq!(caps.replacement_policy, None);
    let value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/native-tables/request.json"
    ))
    .unwrap();
    let s = Snapshot::new(
        serde_json::from_value(value["document"].clone()).unwrap(),
        Default::default(),
    )
    .unwrap();
    let ObjectContent::Table { table } = &s.document().objects[&id()].content else {
        panic!()
    };
    let covered = table
        .rows
        .iter()
        .flat_map(|r| &r.cells)
        .find(|c| matches!(c.merge, TableCellMerge::Covered { .. }))
        .unwrap();
    let q = TextCapabilitiesQuery {
        cell: Some(covered.id.clone()),
        ..query(None)
    };
    let caps = text_capabilities(&s, &q, &|| false).unwrap();
    assert_eq!(reason(caps.replace), TextEditRestriction::CoveredCell);
    assert_eq!(caps.cell, q.cell);
    assert!(text_capabilities(&s, &query(None), &|| false).is_err());
    assert!(
        text_capabilities(
            &s,
            &TextCapabilitiesQuery {
                cell: Some(CellId::new("missing").unwrap()),
                ..query(None)
            },
            &|| false
        )
        .is_err()
    );
    assert!(text_capabilities(&s, &q, &|| true).is_err());
}
