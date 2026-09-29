//! Cell layout is resolved from real physical table/cell scopes.
#[allow(dead_code)]
#[path = "../../../tools/test-support/source_table.rs"]
mod support;
use mo_pptx::{
    PptxError,
    source::{
        SourceIndex,
        table::SourceCellAddress,
        text::{body::*, *},
    },
};
use support::*;

fn resolver(i: &SourceIndex) -> CellTextBodyResolver<'_> {
    CellTextBodyResolver::bind(i, &i.source_sha256, &target(i), Default::default(), &|| {
        false
    })
    .unwrap()
}
fn body(i: &SourceIndex, cell: SourceCellAddress) -> EffectiveTextBody {
    match resolver(i)
        .resolve(cell, Default::default(), &|| false)
        .unwrap()
    {
        TextBodyOutcome::Resolved { body } => *body,
        other => panic!("{other:?}"),
    }
}
fn first_properties(xml: &mut String, attributes: &str) {
    let start = xml.find("<a:tcPr").unwrap();
    let end = start + xml[start..].find('>').unwrap() + 1;
    xml.replace_range(start..end, &format!("<a:tcPr {attributes}>"));
}
const FIRST: SourceCellAddress = SourceCellAddress { row: 0, column: 0 };

#[test]
fn explicit_cell_layout_keeps_native_declaration_and_body_only_attributes() {
    let bytes = rewrite(&bytes(), |mut s| {
        let start = s.find("<a:bodyPr").unwrap();
        let end = start + s[start..].find('>').unwrap() + 1;
        s.replace_range(start..end, "<a:bodyPr lIns=\"999\" rIns=\"998\" tIns=\"997\" bIns=\"996\" anchor=\"b\" vert=\"vert\" anchorCtr=\"1\" horzOverflow=\"clip\" vertOverflow=\"clip\" numCol=\"3\" spcFirstLastPara=\"1\">");
        s
    });
    let i = read(&bytes);
    let native = i.surfaces[SLIDE].objects[0].table.as_ref().unwrap();
    let b = body(&i, FIRST);
    for (value, expected, property) in [
        (
            &b.attributes.left_inset,
            "10000",
            TextBodyProperty::LeftInset,
        ),
        (
            &b.attributes.right_inset,
            "30000",
            TextBodyProperty::RightInset,
        ),
        (&b.attributes.top_inset, "20000", TextBodyProperty::TopInset),
        (
            &b.attributes.bottom_inset,
            "40000",
            TextBodyProperty::BottomInset,
        ),
    ] {
        assert_eq!(value.as_ref().unwrap().lexical(), expected);
        assert_eq!(
            b.origins[&property],
            TextBodyOrigin::Cell {
                object: target(&i),
                cell: FIRST,
                source_ordinal: native.rows[0].cells[0]
                    .properties
                    .as_ref()
                    .unwrap()
                    .source_ordinal
            }
        );
    }
    assert_eq!(b.attributes.anchor, Some(NativeTextAnchor::Ctr));
    assert_eq!(b.attributes.vertical, Some(NativeTextVertical::Horz));
    assert_eq!(b.attributes.center_anchor, Some(false));
    assert_eq!(
        b.attributes.horizontal_overflow,
        Some(NativeTextHorizontalOverflow::Overflow)
    );
    assert_eq!(
        b.attributes.vertical_overflow,
        Some(NativeTextVerticalOverflow::Clip)
    );
    assert_eq!(b.attributes.columns, Some(3));
    assert_eq!(b.attributes.paragraph_spacing, Some(true));
    let root = native.rows[0].cells[0].text_body_ordinal.unwrap();
    let body_pr = i.surfaces[SLIDE].text.nodes[&root].children[0];
    assert_eq!(
        b.origins[&TextBodyProperty::Columns],
        TextBodyOrigin::Cell {
            object: target(&i),
            cell: FIRST,
            source_ordinal: body_pr
        }
    );
    // A covered cell preserves its independent payload. Painting ownership is
    // decided by the validated grid/frame layer, not by flattening this body.
    let covered = body(&i, SourceCellAddress { row: 0, column: 1 });
    assert_eq!(covered.attributes.columns, Some(1));
}

#[test]
fn native_cell_defaults_do_not_inherit_body_layout_attributes() {
    let bytes = rewrite(&bytes(), |mut s| {
        first_properties(&mut s, "");
        let start = s.find("<a:bodyPr").unwrap();
        let end = start + s[start..].find('>').unwrap() + 1;
        s.replace_range(
            start..end,
            "<a:bodyPr lIns=\"999\" anchor=\"b\" horzOverflow=\"overflow\">",
        );
        s
    });
    let i = read(&bytes);
    let b = body(&i, FIRST);
    assert_eq!(b.attributes.left_inset.as_ref().unwrap().lexical(), "91440");
    assert_eq!(
        b.attributes.right_inset.as_ref().unwrap().lexical(),
        "91440"
    );
    assert_eq!(b.attributes.top_inset.as_ref().unwrap().lexical(), "45720");
    assert_eq!(
        b.attributes.bottom_inset.as_ref().unwrap().lexical(),
        "45720"
    );
    assert_eq!(b.attributes.anchor, Some(NativeTextAnchor::T));
    assert_eq!(
        b.attributes.horizontal_overflow,
        Some(NativeTextHorizontalOverflow::Clip)
    );
    assert_eq!(
        b.origins[&TextBodyProperty::LeftInset],
        TextBodyOrigin::CellDefault {
            object: target(&i),
            cell: FIRST
        }
    );
    assert_eq!(b.origins.len(), 19);
    assert!(matches!(b.autofit, EffectiveTextAutofit::None { .. }));
}

#[test]
fn cell_body_binding_rejects_stale_input_and_obeys_limits_and_cancellation() {
    let i = read(&bytes());
    let before = i.clone();
    let wrong = mo_common::Digest::try_from("0".repeat(64)).unwrap();
    assert!(
        CellTextBodyResolver::bind(&i, &wrong, &target(&i), Default::default(), &|| false).is_err()
    );
    assert!(
        CellTextBodyResolver::bind(
            &i,
            &i.source_sha256,
            &target(&i),
            Default::default(),
            &|| true
        )
        .is_err()
    );
    let bound = resolver(&i);
    for limits in [
        TextBodyLimits {
            max_queries: 0,
            ..Default::default()
        },
        TextBodyLimits {
            max_steps: 0,
            ..Default::default()
        },
        TextBodyLimits {
            max_lexical_bytes: 0,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            bound.resolve(FIRST, limits, &|| false),
            Err(PptxError::Limit(_))
        ));
    }
    assert!(bound.resolve(FIRST, Default::default(), &|| true).is_err());
    assert!(
        bound
            .resolve(
                SourceCellAddress { row: 3, column: 0 },
                Default::default(),
                &|| false
            )
            .is_err()
    );
    assert_eq!(i, before);
}
