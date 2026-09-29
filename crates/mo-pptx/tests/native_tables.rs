use mo_common::*;
use mo_opc::{Package, PackageLimits, PartName};
use mo_pptx::*;
use mo_presentation_model::*;
use mo_xml::{XmlEvent, XmlLimits};
mod support;

fn fixture() -> (Document, ExportDefaults) {
    let mut d: Document = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/basic-shape.json"
    ))
    .unwrap();
    let object = d.objects.values_mut().next().unwrap();
    let ObjectContent::Shape {
        text: Some(body), ..
    } = &object.content
    else {
        panic!()
    };
    let body = body.clone();
    let mut table = Table {
        columns: (0..3)
            .map(|c| TableColumn {
                id: ColumnId::new(format!("column:{c}")).unwrap(),
                width: Emu::new(1_828_800),
            })
            .collect(),
        rows: (0..3)
            .map(|r| TableRow {
                id: RowId::new(format!("row:{r}")).unwrap(),
                height: Emu::new(914_400),
                cells: (0..3)
                    .map(|c| {
                        let mut text = body.clone();
                        text.insets = Insets {
                            left: Emu::new(10000),
                            top: Emu::new(20000),
                            right: Emu::new(30000),
                            bottom: Emu::new(40000),
                        };
                        text.paragraphs[0].id =
                            ParagraphId::new(format!("paragraph:{r}:{c}")).unwrap();
                        text.paragraphs[0].runs[0].id = RunId::new(format!("run:{r}:{c}")).unwrap();
                        text.paragraphs[0].runs[0].content = InlineContent::Text {
                            text: format!("cell {r}:{c} 中文 & <🚀> é"),
                        };
                        let mut style = TableCellStyle {
                            fill: Inherited::Value(Fill::Solid {
                                color: Color::Srgb {
                                    rgba: Rgba {
                                        red: 30 * (r + 1) as u8,
                                        green: 40 * (c + 1) as u8,
                                        blue: 200,
                                        alpha: 255,
                                    },
                                },
                            }),
                            vertical_alignment: Inherited::Value(TableVerticalAlignment::Center),
                            ..Default::default()
                        };
                        style.borders.bottom = Inherited::Value(Stroke::Solid {
                            width: Emu::new(12700),
                            color: Color::Theme {
                                slot: ThemeColor::Accent1,
                            },
                            cap: Some(LineCap::Flat),
                            join: Some(LineJoin::Round {}),
                        });
                        TableCell {
                            id: CellId::new(format!("cell:{r}:{c}")).unwrap(),
                            merge: Default::default(),
                            text: Some(text),
                            style,
                        }
                    })
                    .collect(),
            })
            .collect(),
    };
    table.rows[0].cells[0].merge = TableCellMerge::Span {
        rows: 2,
        columns: 2,
    };
    for (r, c) in [(0, 1), (1, 0), (1, 1)] {
        table.rows[r].cells[c].merge = TableCellMerge::Covered {
            origin: CellId::new("cell:0:0").unwrap(),
        };
    }
    table.rows[2].cells[2].text = None;
    object.transform.as_mut().unwrap().size = Size {
        width: Emu::new(5_486_400),
        height: Emu::new(2_743_200),
    };
    object.content = ObjectContent::Table { table };
    (d, support::input().1)
}
#[test]
fn merged_table_exports_native_physical_cells_rich_text_and_cell_properties() {
    let (d, defaults) = fixture();
    let original = d.clone();
    let bytes = export(
        &d,
        &defaults,
        &support::resources(),
        Default::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(d, original);
    let package = Package::open(
        bytes.as_slice(),
        bytes.len() as u64,
        PackageLimits::default(),
        &|| false,
    )
    .unwrap();
    let xml = package
        .read_part(
            &PartName::new("/ppt/slides/slide1.xml").unwrap(),
            1 << 20,
            &|| false,
        )
        .unwrap();
    let a = "http://schemas.openxmlformats.org/drawingml/2006/main";
    let p = "http://schemas.openxmlformats.org/presentationml/2006/main";
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    let mut cells = vec![];
    let mut all_text = String::new();
    mo_xml::scan(&xml, XmlLimits::default(), |event| {
        match event {
            XmlEvent::Start { element, .. } => {
                *counts.entry(element.qualified_name.clone()).or_default() += 1;
                if element.name.is(a, "tc") {
                    cells.push(
                        ["rowSpan", "gridSpan", "hMerge", "vMerge"]
                            .map(|k| element.attribute(k).map(str::to_owned)),
                    );
                }
                if element.name.is(a, "tcPr") {
                    assert_eq!(
                        (element.attribute("marL"), element.attribute("marT")),
                        if cells.len() == 9 {
                            (None, None)
                        } else {
                            (Some("10000"), Some("20000"))
                        }
                    );
                    assert_eq!(element.attribute("anchor"), Some("ctr"));
                }
                if element.name.is(a, "bodyPr") {
                    assert!(element.attribute("lIns").is_none());
                }
                assert!(!element.name.is(p, "txBody"));
            }
            XmlEvent::Text { text, .. } => all_text.push_str(text),
            _ => (),
        }
        Ok(())
    })
    .unwrap();
    for (name, count) in [
        ("p:graphicFrame", 1),
        ("p:xfrm", 1),
        ("a:tbl", 1),
        ("a:tr", 3),
        ("a:gridCol", 3),
        ("a:tc", 9),
        ("a:txBody", 9),
        ("a:lnB", 9),
    ] {
        assert_eq!(counts.get(name), Some(&count), "{name}");
    }
    assert!(!counts.contains_key("p:sp"));
    assert!(!counts.contains_key("p:pic"));
    let str_cells: Vec<_> = cells
        .iter()
        .map(|c| c.each_ref().map(|v| v.as_deref()))
        .collect();
    assert_eq!(
        &str_cells[..5],
        &[
            [Some("2"), Some("2"), None, None],
            [Some("2"), None, Some("1"), None],
            [None, None, None, None],
            [None, Some("2"), None, Some("1")],
            [None, None, Some("1"), Some("1")],
        ]
    );
    for r in 0..3 {
        for c in 0..3 {
            if (r, c) != (2, 2) {
                assert!(all_text.contains(&format!("cell {r}:{c} 中文 & <🚀> é")));
            }
        }
    }
    assert_eq!(
        bytes,
        export(
            &d,
            &defaults,
            &support::resources(),
            Default::default(),
            &|| false
        )
        .unwrap()
    );
}
#[test]
fn export_rejects_frame_grid_disagreement_and_never_discards_frame_paint() {
    let (mut d, defaults) = fixture();
    let object = d.objects.values_mut().next().unwrap();
    object.transform.as_mut().unwrap().size.width = Emu::new(1);
    assert!(matches!(
        export(
            &d,
            &defaults,
            &support::resources(),
            Default::default(),
            &|| false
        ),
        Err(PptxError::InvalidDocument(_))
    ));
    let (mut d, defaults) = fixture();
    d.objects.values_mut().next().unwrap().appearance.fill = Inherited::Value(Fill::Solid {
        color: Color::Srgb {
            rgba: Rgba {
                red: 1,
                green: 2,
                blue: 3,
                alpha: 255,
            },
        },
    });
    assert!(matches!(
        export(
            &d,
            &defaults,
            &support::resources(),
            Default::default(),
            &|| false
        ),
        Err(PptxError::Unsupported(_))
    ));
}
