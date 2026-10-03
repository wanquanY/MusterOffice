#[allow(dead_code)]
mod support;
use mo_common::*;
use mo_opc::{Package, PartName, RewritePlan};
use mo_pptx::{
    source::{
        document::{SourcePlan, import_document},
        table::SourceCellAddress,
        *,
    },
    *,
};
use mo_presentation_edit::{Operation, OperationEntry, Snapshot, Transaction, prepare};
use mo_presentation_model::*;
const SLIDE: &str = "/ppt/slides/slide1.xml";
fn escaped(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
fn fixture() -> Vec<u8> {
    let q: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/native-tables/request.json"
    ))
    .unwrap();
    export(
        &serde_json::from_value(q["document"].clone()).unwrap(),
        &serde_json::from_value(q["defaults"].clone()).unwrap(),
        &support::resources(),
        Default::default(),
        &|| false,
    )
    .unwrap()
}
fn package(b: &[u8]) -> Package<&[u8]> {
    Package::open(b, b.len() as u64, Default::default(), &|| false).unwrap()
}
fn xml(b: &[u8], part: &str) -> String {
    String::from_utf8(
        package(b)
            .read_part(&PartName::new(part).unwrap(), 1 << 20, &|| false)
            .unwrap(),
    )
    .unwrap()
}
fn change(b: &[u8], f: impl FnOnce(String) -> String) -> Vec<u8> {
    let mut plan = RewritePlan::new();
    plan.replace_part(PartName::new(SLIDE).unwrap(), f(xml(b, SLIDE)).into_bytes())
        .unwrap();
    plan.to_bytes(&package(b), &|| false).unwrap()
}
fn index(b: &[u8]) -> SourceIndex {
    inspect_source(&package(b), Default::default(), &|| false).unwrap()
}
fn object(i: &SourceIndex) -> &SourceObject {
    &i.surfaces[SLIDE].objects[0]
}
fn edit(i: &SourceIndex, p: u32, replacement: &str) -> SourceTextEdit {
    SourceTextEdit {
        target: SourceTextTarget {
            part: SLIDE.into(),
            object_id: object(i).native_id,
            paragraph: p,
            run: 0,
        },
        expected_text: object(i).paragraphs[p as usize][0].text.clone(),
        replacement: replacement.into(),
    }
}
#[test]
fn every_physical_cell_has_one_owned_text_range_including_covered_and_empty_cells() {
    let i = index(&fixture());
    let o = object(&i);
    let t = o.table.as_ref().unwrap();
    assert!(o.text_body_ordinal.is_none());
    assert_eq!(t.columns.len(), 3);
    assert_eq!(t.rows.len(), 3);
    assert_eq!(o.paragraphs.len(), 9);
    assert!(o.paragraphs[8].is_empty());
    assert!(i.surfaces[SLIDE].visual_issues.is_empty());
    assert!(o.visual_issues.is_empty());
    for (r, row) in t.rows.iter().enumerate() {
        for (c, cell) in row.cells.iter().enumerate() {
            assert_eq!(cell.paragraph_start, (r * 3 + c) as u32);
            assert_eq!(cell.paragraph_count, 1);
            let root = i.surfaces[SLIDE]
                .text
                .roots
                .iter()
                .find(|v| Some(v.source_ordinal) == cell.text_body_ordinal)
                .unwrap();
            assert_eq!(root.owner, Some(o.native_id));
            assert_eq!(
                root.cell,
                Some(SourceCellAddress {
                    row: r as u32,
                    column: c as u32
                })
            );
        }
    }
    assert_eq!(t.rows[0].cells[1].horizontal_merge, Some(true));
    assert_eq!(t.rows[1].cells[1].vertical_merge, Some(true));
    assert!(o.paragraphs.iter().flatten().all(|r| r.editable));
}
#[test]
fn native_absence_explicit_defaults_all_edges_and_unknowns_remain_distinct() {
    let b = change(&fixture(), |s| {
        let a = s.find("<a:tcPr").unwrap();
        let z = s[a..].find("</a:tcPr>").unwrap() + a + "</a:tcPr>".len();
        let edges=["lnL","lnR","lnT","lnB","lnTlToBr","lnBlToTr"].into_iter().map(|n|format!("<a:{n} w=\"12700\"><a:solidFill><a:srgbClr val=\"123456\"/></a:solidFill></a:{n}>")).collect::<String>();
        let props = format!(
            "<a:tcPr marL=\"1pt\" anchorCtr=\"0\" horzOverflow=\"clip\">{edges}<a:noFill/><a:extLst><a:ext uri=\"owned-test\"/></a:extLst></a:tcPr>"
        );
        let s = format!("{}{}{}", &s[..a], props, &s[z..]);
        s.replacen("<a:tblPr>","<a:tblPr rtl=\"0\" bandRow=\"1\" unknown=\"preserve\">",1)
            .replacen("</a:tblPr>","<a:solidFill><a:srgbClr val=\"abcdef\"/></a:solidFill><a:effectLst><a:glow rad=\"12700\"><a:srgbClr val=\"123456\"/></a:glow></a:effectLst><a:tableStyleId>{00000000-0000-0000-0000-000000000000}</a:tableStyleId></a:tblPr>",1)
            .replacen("rowSpan=\"2\"","rowSpan=\"2\" hMerge=\"0\"",1)
    });
    let i = index(&b);
    let t = object(&i).table.as_ref().unwrap();
    let props = t.properties.as_ref().unwrap();
    assert_eq!(props.right_to_left, Some(false));
    assert_eq!(props.band_rows, Some(true));
    assert_eq!(props.first_row, None);
    assert!(props.fill.is_some());
    assert!(props.effects.is_some());
    assert!(props.style_id.is_some());
    let c = &t.rows[0].cells[0];
    assert_eq!(c.horizontal_merge, Some(false));
    assert_eq!(t.rows[0].cells[2].horizontal_merge, None);
    let p = c.properties.as_ref().unwrap();
    assert_eq!(p.margins.left.as_ref().unwrap().lexical(), "1pt");
    assert!(p.margins.right.is_none());
    assert_eq!(p.center_anchor, Some(false));
    assert!(p.borders.iter().all(Option::is_some));
    assert_eq!(t.retained_ordinals.len(), 2);
    let changed = edit_source_text(
        &package(&b),
        &SourceTextEdits {
            expected_source_sha256: i.source_sha256.clone(),
            edits: vec![edit(&i, 4, "covered text edited")],
        },
        Default::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(object(&index(&changed)).table.as_ref(), Some(t));
    assert_eq!(
        xml(&changed, SLIDE),
        xml(&b, SLIDE).replace(
            &escaped(&object(&i).paragraphs[4][0].text),
            "covered text edited"
        )
    );
}
#[test]
fn source_edits_touch_only_selected_leaves_and_preserve_every_other_package_part() {
    let b = fixture();
    let i = index(&b);
    let source = package(&b);
    let q = SourceTextEdits {
        expected_source_sha256: i.source_sha256.clone(),
        edits: vec![
            edit(&i, 0, "起点 & <🚀>"),
            edit(&i, 4, "合并覆盖单元格"),
            edit(&i, 7, "last é"),
        ],
    };
    let output = edit_source_text(&source, &q, Default::default(), &|| false).unwrap();
    let after = package(&output);
    let j = index(&output);
    assert_eq!(
        source.parts().keys().collect::<Vec<_>>(),
        after.parts().keys().collect::<Vec<_>>()
    );
    assert_eq!(source.relationships(), after.relationships());
    for (part, info) in source.parts() {
        if part.as_str() != SLIDE {
            assert_eq!(info.sha256, after.parts()[part].sha256, "{part}");
        }
    }
    let mut expected = xml(&b, SLIDE);
    for e in &q.edits {
        expected = expected.replace(&escaped(&e.expected_text), &escaped(&e.replacement));
    }
    assert_eq!(xml(&output, SLIDE), expected);
    assert_eq!(i.surfaces[SLIDE].text, j.surfaces[SLIDE].text);
    assert_eq!(object(&i).table, object(&j).table);
    for e in &q.edits {
        assert_eq!(
            object(&j).paragraphs[e.target.paragraph as usize][0].text,
            e.replacement
        );
    }
    assert!(edit_source_text(&after, &q, Default::default(), &|| false).is_err());
}
#[test]
fn source_import_transaction_exports_editable_cell_text_through_existing_native_plan() {
    let b = fixture();
    let p = package(&b);
    let d = import_document(
        &p,
        DocumentId::new("table-import").unwrap(),
        ResourceId::new("source").unwrap(),
        Default::default(),
        &|| false,
    )
    .unwrap();
    let o=d.objects.values().find(|o|matches!(&o.content,ObjectContent::RetainedSource{paragraphs,..} if paragraphs.len()==9)).unwrap();
    let ObjectContent::RetainedSource { paragraphs, .. } = &o.content else {
        panic!()
    };
    let op = Operation::SpliceText {
        object: o.id.clone(),
        paragraph: paragraphs[4].id.clone(),
        run: paragraphs[4].runs[0].id.clone(),
        start: 0,
        delete: 0,
        insert: "原生表格 ".into(),
    };
    let snapshot = Snapshot::new(d, Default::default()).unwrap();
    let tx = Transaction {
        document_id: snapshot.document().id.clone(),
        base_revision: snapshot.revision().clone(),
        request_id: RequestId::new("cell-edit").unwrap(),
        operations: vec![OperationEntry {
            operation_id: OperationId::new("cell-edit-op").unwrap(),
            operation: op,
        }],
    };
    let result = prepare(&snapshot, &tx, Default::default()).unwrap();
    let output = SourcePlan::new(result.snapshot.document(), &p, Default::default(), &|| {
        false
    })
    .unwrap()
    .write(&p, &|| false)
    .unwrap();
    let original = index(&b);
    let after = index(&output);
    assert_eq!(object(&original).table, object(&after).table);
    assert_eq!(
        object(&after).paragraphs[4][0].text,
        format!("原生表格 {}", object(&original).paragraphs[4][0].text)
    );
}
#[test]
fn table_budgets_are_aggregate_and_cancellation_never_returns_partial_index() {
    let b = fixture();
    let p = package(&b);
    for limits in [
        SourceLimits {
            max_table_cells: 8,
            ..Default::default()
        },
        SourceLimits {
            max_table_elements: 1,
            ..Default::default()
        },
        SourceLimits {
            max_table_attribute_bytes: 1,
            ..Default::default()
        },
        SourceLimits {
            max_text_bytes: 1,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            inspect_source(&p, limits, &|| false),
            Err(PptxError::Xml(mo_xml::XmlError::Limit(_)))
        ));
    }
    let b = change(&b, |s| {
        let a = s.find("<p:graphicFrame>").unwrap();
        let z = s.find("</p:graphicFrame>").unwrap() + "</p:graphicFrame>".len();
        let original = &s[a..z];
        let id = object(&index(&fixture())).native_id;
        let copy = original.replacen(&format!("id=\"{id}\""), "id=\"987\"", 1);
        s.replace("</p:spTree>", &format!("{copy}</p:spTree>"))
    });
    assert!(
        inspect_source(
            &package(&b),
            SourceLimits {
                max_table_cells: 17,
                ..Default::default()
            },
            &|| false
        )
        .is_err()
    );
    assert_eq!(
        inspect_source(
            &package(&b),
            SourceLimits {
                max_table_cells: 18,
                ..Default::default()
            },
            &|| false
        )
        .unwrap()
        .surfaces[SLIDE]
            .objects
            .len(),
        2
    );
    let calls = std::cell::Cell::new(0);
    let _ = inspect_source(&p, Default::default(), &|| {
        calls.set(calls.get() + 1);
        false
    })
    .unwrap();
    for stop in [1, calls.get() / 2, calls.get() - 1] {
        let n = std::cell::Cell::new(0);
        assert!(matches!(
            inspect_source(&p, Default::default(), &|| {
                n.set(n.get() + 1);
                n.get() >= stop
            }),
            Err(PptxError::Cancelled
                | PptxError::Xml(mo_xml::XmlError::Cancelled)
                | PptxError::Opc(mo_opc::OpcError::Cancelled))
        ));
    }
}
#[test]
fn duplicate_out_of_order_or_ambiguous_text_declarations_are_rejected() {
    let b = fixture();
    for (from, to) in [
        ("<a:tblGrid>", "<a:tblPr/><a:tblGrid>"),
        ("</a:tblGrid>", "</a:tblGrid><a:tblGrid/>"),
        ("<a:tcPr", "<a:tcPr/><a:tcPr"),
        (
            "</a:txBody>",
            "</a:txBody><a:txBody><a:bodyPr/><a:p/></a:txBody>",
        ),
        ("</a:t>", "</a:t><a:t>duplicate</a:t>"),
    ] {
        let changed = change(&b, |s| s.replacen(from, to, 1));
        assert!(
            inspect_source(&package(&changed), Default::default(), &|| false).is_err(),
            "accepted {to}"
        );
    }
}
#[test]
fn non_table_graphic_data_never_becomes_a_table_projection() {
    let b = change(&fixture(), |s| {
        s.replace(
            "http://schemas.openxmlformats.org/drawingml/2006/table",
            "urn:owned-unimplemented-graphic",
        )
    });
    let i = index(&b);
    let o = object(&i);
    assert!(o.table.is_none());
    assert!(o.paragraphs.is_empty());
    assert!(!o.visual_issues.is_empty());
}

#[test]
fn legacy_projection_profiles_keep_opaque_tables_without_losing_bytes_or_accepting_forged_text() {
    let b = fixture();
    let p = package(&b);
    for profile in [
        SourceBindingProfile::PresentationmlRetainedFieldsV1,
        SourceBindingProfile::PresentationmlRetainedFieldsV2,
    ] {
        let mut d = import_document(
            &p,
            DocumentId::new("historical-table").unwrap(),
            ResourceId::new("native").unwrap(),
            Default::default(),
            &|| false,
        )
        .unwrap();
        assert_eq!(
            d.source_bindings.as_ref().unwrap().profile,
            SourceBindingProfile::PresentationmlRetainedFieldsV5
        );
        let current = d.clone();
        d.source_bindings.as_mut().unwrap().profile = profile;
        if profile == SourceBindingProfile::PresentationmlRetainedFieldsV1 {
            d.title.clear();
        }
        for object in d.objects.values_mut() {
            object.accessibility = Default::default();
            if let ObjectContent::RetainedSource { paragraphs, .. } = &mut object.content {
                paragraphs.clear();
            }
        }
        for native in d.source_bindings.as_mut().unwrap().objects.values_mut() {
            native.runs.clear();
        }
        let out = SourcePlan::new(&d, &p, Default::default(), &|| false)
            .unwrap()
            .write(&p, &|| false)
            .unwrap();
        assert_eq!(p.parts(), package(&out).parts());
        assert_eq!(xml(&b, SLIDE), xml(&out, SLIDE));
        let mut forged = current;
        forged.source_bindings.as_mut().unwrap().profile = profile;
        if profile == SourceBindingProfile::PresentationmlRetainedFieldsV1 {
            forged.title.clear();
        }
        assert!(matches!(
            SourcePlan::new(&forged, &p, Default::default(), &|| false),
            Err(PptxError::SourceConflict(_))
        ));
    }
}

#[test]
fn empty_native_cells_and_exact_dimension_units_are_retained_without_invented_defaults() {
    let b = change(&fixture(), |s| {
        let a = s.find("<a:tc ").unwrap();
        let end = s[a..].find("</a:tc>").unwrap() + a + "</a:tc>".len();
        let replaced = format!(
            "{}<a:tc id=\"empty\" rowSpan=\"1\" gridSpan=\"1\" hMerge=\"false\" vMerge=\"false\"/>{}",
            &s[..a],
            &s[end..]
        );
        replaced
            .replacen(
                "<a:gridCol w=\"1828800\"",
                "<a:gridCol w=\"2.000000000000000001in\"",
                1,
            )
            .replacen("<a:tr h=\"914400\"", "<a:tr h=\"72pt\"", 1)
    });
    let i = index(&b);
    let o = object(&i);
    let t = o.table.as_ref().unwrap();
    assert_eq!(t.columns[0].width.lexical(), "2.000000000000000001in");
    assert_eq!(t.rows[0].height.lexical(), "72pt");
    let c = &t.rows[0].cells[0];
    assert!(c.properties.is_none());
    assert!(c.text_body_ordinal.is_none());
    assert_eq!(c.paragraph_count, 0);
    assert_eq!(c.native_id.as_deref(), Some("empty"));
    assert_eq!(o.paragraphs.len(), 8);
    assert_eq!(t.rows[0].cells[1].paragraph_start, 0);
    let mut q = SourceTextEdits {
        expected_source_sha256: i.source_sha256.clone(),
        edits: vec![edit(&i, 0, "精确保留")],
    };
    let out = edit_source_text(&package(&b), &q, Default::default(), &|| false).unwrap();
    assert_eq!(object(&index(&out)).table, object(&i).table);
    q.edits[0].expected_text.push_str("stale");
    assert!(edit_source_text(&package(&b), &q, Default::default(), &|| false).is_err());
}

#[test]
fn multiple_paragraphs_breaks_and_fields_keep_flat_addresses_and_edit_constraints() {
    let b = change(&fixture(), |s| {
        let a = s.find("<a:txBody>").unwrap();
        let end = s[a..].find("</a:txBody>").unwrap() + a + "</a:txBody>".len();
        let body = "<a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>first</a:t></a:r><a:br/><a:fld id=\"{00000000-0000-0000-0000-000000000001}\" type=\"slidenum\"><a:t>1</a:t></a:fld></a:p><a:p><a:r><a:t>second</a:t></a:r><a:r><a:t>第三</a:t></a:r></a:p></a:txBody>";
        format!("{}{}{}", &s[..a], body, &s[end..])
    });
    let i = index(&b);
    let o = object(&i);
    let t = o.table.as_ref().unwrap();
    assert_eq!(t.rows[0].cells[0].paragraph_count, 2);
    assert_eq!(t.rows[0].cells[1].paragraph_start, 2);
    assert_eq!(o.paragraphs.len(), 10);
    assert_eq!(
        o.paragraphs[0].iter().map(|r| r.kind).collect::<Vec<_>>(),
        [
            SourceRunKind::Text,
            SourceRunKind::Break,
            SourceRunKind::Field
        ]
    );
    assert_eq!(
        o.paragraphs[0][2].edit_constraint,
        Some(SourceTextConstraint::DynamicField)
    );
    assert!(!o.paragraphs[0][2].editable);
    let q = SourceTextEdits {
        expected_source_sha256: i.source_sha256.clone(),
        edits: vec![edit(&i, 1, "第二段改写"), edit(&i, 2, "下一单元格")],
    };
    let output = edit_source_text(&package(&b), &q, Default::default(), &|| false).unwrap();
    let after = index(&output);
    assert_eq!(object(&after).table, o.table);
    assert_eq!(object(&after).paragraphs[1][1].text, "第三");
    assert_eq!(object(&after).paragraphs[0], o.paragraphs[0]);
}
#[test]
fn table_text_keeps_compatibility_structured_leaf_and_timing_protection() {
    for (kind, constraint) in [
        (0, SourceTextConstraint::CompatibilityBranch),
        (1, SourceTextConstraint::StructuredLeaf),
        (2, SourceTextConstraint::TimingReferences),
    ] {
        let b = change(&fixture(), |s| {
            match kind {
            0=> {let a=s.find("<a:txBody>").unwrap();let end=s[a..].find("</a:txBody>").unwrap()+a+"</a:txBody>".len();let body=&s[a..end];format!("{}<mc:AlternateContent xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\" xmlns:u=\"urn:future\"><mc:Choice Requires=\"u\">{body}</mc:Choice><mc:Fallback>{body}</mc:Fallback></mc:AlternateContent>{}",&s[..a],&s[end..])},
            1=>s.replacen("<a:t>","<a:t xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\" xmlns:q=\"urn:ignored\" mc:Ignorable=\"q\"><q:metadata/>",1),
            _=>s.replace("</p:sld>","<p:timing/></p:sld>")
        }
        });
        let i = index(&b);
        assert_eq!(
            object(&i).paragraphs[0][0].edit_constraint,
            Some(constraint)
        );
        assert!(!object(&i).paragraphs[0][0].editable);
        let q = SourceTextEdits {
            expected_source_sha256: i.source_sha256.clone(),
            edits: vec![edit(&i, 0, "protected")],
        };
        assert!(matches!(
            edit_source_text(&package(&b), &q, Default::default(), &|| false),
            Err(PptxError::Unsupported(_))
        ));
        if kind != 2 {
            let q = SourceTextEdits {
                expected_source_sha256: i.source_sha256.clone(),
                edits: vec![edit(&i, 1, "next cell")],
            };
            assert!(edit_source_text(&package(&b), &q, Default::default(), &|| false).is_ok());
        }
    }
}
