#[allow(dead_code)]
#[path = "table_styles/support.rs"]
mod support;
use mo_opc::{PartName, RewritePlan};
use mo_pptx::source::table::borders::query;
use mo_pptx::{
    PptxError,
    source::{
        SourceIndex,
        color::*,
        fill::resolve::*,
        line::resolve::*,
        table::{borders::*, *},
    },
};
use serde_json::{Value, json};
use support::{table, *};
const EDGE_TAGS: [&str; 6] = ["lnL", "lnR", "lnT", "lnB", "lnTlToBr", "lnBlToTr"];
const STYLE_EDGES: [&str; 8] = [
    "left", "right", "top", "bottom", "insideH", "insideV", "tl2br", "tr2bl",
];

fn cells(b: &[u8], props: impl Fn(usize) -> String) -> Vec<u8> {
    table::rewrite(b, |mut xml| {
        let mut start = 0;
        for i in 0..9 {
            let a = start + xml[start..].find("<a:tcPr").unwrap();
            let a = a + xml[a..].find('>').unwrap() + 1;
            let end = a + xml[a..].find("</a:tcPr>").unwrap();
            let value = props(i);
            xml.replace_range(a..end, &value);
            start = a + value.len();
        }
        xml
    })
}
fn base(body: &str, shared: bool) -> Vec<u8> {
    let b = cells(&table::bytes(), |_| String::new());
    if shared {
        inline(
            &catalog(&b, &list(&style("tblStyle", ID, body))),
            &format!("<a:tableStyleId>{ID}</a:tableStyleId>"),
        )
    } else {
        inline(&b, &style("tableStyle", ID, body))
    }
}
fn theme_lines(b: &[u8], lines: &str) -> Vec<u8> {
    let index = read(b);
    let package = package(b);
    let part = PartName::new(
        &index.surfaces[table::SLIDE]
            .theme_selection
            .format
            .as_ref()
            .unwrap()
            .part,
    )
    .unwrap();
    let mut xml = String::from_utf8(package.read_part(&part, 1 << 24, &|| false).unwrap()).unwrap();
    let a = xml.find("<a:lnStyleLst>").unwrap() + "<a:lnStyleLst>".len();
    let z = a + xml[a..].find("</a:lnStyleLst>").unwrap();
    xml.replace_range(a..z, lines);
    let mut plan = RewritePlan::new();
    plan.replace_part(part, xml.into_bytes()).unwrap();
    plan.to_bytes(&package, &|| false).unwrap()
}
fn target(index: &SourceIndex, row: u32, column: u32, edge: TableCellEdge) -> TableBorderTarget {
    TableBorderTarget {
        native_id: table::target(index).native_id,
        cell: SourceCellAddress { row, column },
        edge,
    }
}
fn request(index: &SourceIndex, targets: Vec<TableBorderTarget>) -> SourceTableBorderQuery {
    SourceTableBorderQuery {
        expected_source_sha256: index.source_sha256.clone(),
        surface: table::SLIDE.into(),
        targets,
        line_profile: LineProfile::Drawingml2024DraftV1,
        fill_profile: FillProfile::Drawingml2024DraftV1,
        color_profile: ColorProfile::Ecma3762016DraftV1,
        context: ColorContext::default(),
    }
}
fn all(index: &SourceIndex) -> Vec<TableBorderTarget> {
    (0..3)
        .flat_map(|r| (0..3).flat_map(move |c| TableCellEdge::ALL.map(|e| target(index, r, c, e))))
        .collect()
}
fn run(index: &SourceIndex, targets: Vec<TableBorderTarget>) -> Vec<Value> {
    query(index, &request(index, targets), Default::default(), &|| {
        false
    })
    .unwrap()
    .targets
    .into_iter()
    .map(|v| serde_json::to_value(v).unwrap())
    .collect()
}
fn width(v: &Value) -> &Value {
    &v["stroke"]["geometry"]["width"]["value"]
}
fn edge_origin(v: &Value) -> &Value {
    &v["stroke"]["geometry"]["width"]["declaredBy"]["edge"]
}
fn record(name: &str, b: &[u8], index: &SourceIndex) {
    if let Some(dir) = std::env::var_os("MO_TABLE_BORDER_EVIDENCE_DIR") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        let q = request(index, all(index));
        let out = query(index, &q, Default::default(), &|| false).unwrap();
        for (suffix, bytes) in [
            ("pptx", b.to_vec()),
            ("request.json", serde_json::to_vec_pretty(&q).unwrap()),
            ("response.json", serde_json::to_vec_pretty(&out).unwrap()),
        ] {
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(dir.join(format!("{name}.{suffix}")))
                .unwrap();
            std::io::Write::write_all(&mut f, &bytes).unwrap();
        }
    }
}
fn style_edges() -> String {
    STYLE_EDGES.iter().enumerate().map(|(i,e)| format!("<a:{e}><a:ln w=\"{}\"><a:solidFill><a:srgbClr val=\"{:02X}2030\"/></a:solidFill><a:round/></a:ln></a:{e}>", 10000+i*1000, 16+i)).collect()
}

#[test]
fn all_native_border_properties_and_full_paint_families_survive() {
    let paints = [
        "<a:solidFill><a:srgbClr val=\"A04020\"/></a:solidFill>",
        "<a:gradFill><a:gsLst><a:gs pos=\"0\"><a:srgbClr val=\"102030\"/></a:gs><a:gs pos=\"100000\"><a:srgbClr val=\"A0B0C0\"/></a:gs></a:gsLst><a:lin ang=\"5400000\" scaled=\"0\"/></a:gradFill>",
        "<a:pattFill prst=\"pct10\"><a:fgClr><a:srgbClr val=\"445566\"/></a:fgClr><a:bgClr><a:srgbClr val=\"AABBCC\"/></a:bgClr></a:pattFill>",
        "<a:noFill/>",
        "<a:solidFill><a:srgbClr val=\"123456\"/></a:solidFill>",
        "<a:solidFill><a:srgbClr val=\"654321\"/></a:solidFill>",
    ];
    let b = cells(&table::bytes(), |_| {
        EDGE_TAGS.iter().enumerate().map(|(i,e)| format!("<a:{e} w=\"{}\" cap=\"rnd\" cmpd=\"dbl\" algn=\"in\">{}<a:custDash><a:ds d=\"100000\" sp=\"200000\"/><a:ds d=\"50000\" sp=\"150000\"/></a:custDash><a:miter lim=\"900000\"/><a:headEnd type=\"triangle\" w=\"lg\" len=\"sm\"/><a:tailEnd type=\"oval\" w=\"sm\" len=\"lg\"/></a:{e}>",10000+i*1000,paints[i])).collect()
    });
    let index = read(&b);
    let before = serde_json::to_vec(&index).unwrap();
    let out = run(&index, all(&index));
    assert_eq!(out.len(), 54);
    for (n, v) in out.iter().enumerate() {
        let g = &v["stroke"]["geometry"];
        assert_eq!(width(v), &(10000 + (n % 6) * 1000).to_string());
        assert_eq!(g["cap"]["value"], "rnd");
        assert_eq!(g["compound"]["value"], "dbl");
        assert_eq!(g["alignment"]["value"], "in");
        assert_eq!(g["join"]["limit"]["value"], "900000");
        assert_eq!(g["dash"]["stops"].as_array().unwrap().len(), 2);
        assert_eq!(g["head"]["kind"]["value"], "triangle");
        assert_eq!(g["tail"]["length"]["value"], "lg");
        assert_eq!(g["width"]["declaredBy"]["kind"], "tableCell");
        assert_eq!(g["width"]["declaredBy"]["cell"], v["target"]["cell"]);
    }
    assert_eq!(
        out[0]["fill"]["colors"]["color"]["outcome"]["rgba8"],
        json!([160, 64, 32, 255])
    );
    assert_eq!(
        out[1]["fill"]["colors"]["stops"][1]["outcome"]["rgba8"],
        json!([160, 176, 192, 255])
    );
    assert_eq!(
        out[2]["fill"]["colors"]["foreground"]["outcome"]["rgba8"],
        json!([68, 85, 102, 255])
    );
    assert_eq!(out[3]["fill"]["colors"]["kind"], "none");
    assert_eq!(before, serde_json::to_vec(&index).unwrap());
    record("direct-properties", &b, &index);
}

#[test]
fn style_perimeters_interiors_and_rtl_keep_distinct_sources() {
    let body = format!(
        "<a:wholeTbl><a:tcStyle><a:tcBdr>{}</a:tcBdr></a:tcStyle></a:wholeTbl>",
        style_edges()
    );
    for (name, shared, rtl) in [
        ("inline-edges", false, false),
        ("shared-edges", true, false),
        ("rtl-edges", true, true),
    ] {
        let b = base(&body, shared);
        let b = if rtl {
            table::rewrite(&b, |x| x.replace("<a:tblPr>", "<a:tblPr rtl=\"1\">"))
        } else {
            b
        };
        let index = read(&b);
        let out = run(
            &index,
            TableCellEdge::ALL.map(|e| target(&index, 1, 1, e)).to_vec(),
        );
        for (v, i) in out.iter().zip([5, 5, 4, 4, 6, 7]) {
            assert_eq!(width(v), &(10000 + i * 1000).to_string());
            assert_eq!(
                v["stroke"]["geometry"]["width"]["declaredBy"]["part"],
                if shared { STYLES } else { table::SLIDE }
            );
        }
        let out = run(
            &index,
            vec![
                target(&index, 0, if rtl { 2 } else { 0 }, TableCellEdge::Left),
                target(&index, 2, if rtl { 0 } else { 2 }, TableCellEdge::Right),
                target(&index, 0, 1, TableCellEdge::Top),
                target(&index, 2, 1, TableCellEdge::Bottom),
            ],
        );
        for (v, i) in out.iter().zip(0..4) {
            assert_eq!(width(v), &(10000 + i * 1000).to_string());
        }
        assert_eq!(edge_origin(&out[0]), "left");
        assert_eq!(edge_origin(&out[3]), "bottom");
        record(name, &b, &index);
    }
}

#[test]
fn regions_keep_both_conflicting_sides_and_disconnected_bands() {
    let body = format!(
        "<a:wholeTbl><a:tcStyle><a:tcBdr>{}</a:tcBdr></a:tcStyle></a:wholeTbl><a:band1H><a:tcStyle><a:tcBdr><a:top><a:ln w=\"33333\"><a:noFill/></a:ln></a:top><a:bottom><a:ln w=\"44444\"><a:noFill/></a:ln></a:bottom></a:tcBdr></a:tcStyle></a:band1H><a:firstRow><a:tcStyle><a:tcBdr><a:bottom><a:ln w=\"22222\"><a:noFill/></a:ln></a:bottom></a:tcBdr></a:tcStyle></a:firstRow>",
        style_edges()
    );
    let b = table::rewrite(&base(&body, true), |x| {
        x.replace("<a:tblPr>", "<a:tblPr firstRow=\"1\" bandRow=\"1\">")
    });
    let index = read(&b);
    let out = run(
        &index,
        vec![
            target(&index, 0, 2, TableCellEdge::Bottom),
            target(&index, 1, 2, TableCellEdge::Top),
            target(&index, 1, 2, TableCellEdge::Bottom),
            target(&index, 2, 2, TableCellEdge::Top),
        ],
    );
    assert_eq!(width(&out[0]), "22222");
    assert_eq!(width(&out[1]), "33333");
    assert_eq!(width(&out[2]), "44444");
    assert_eq!(width(&out[3]), "14000");
    assert_eq!(
        out[0]["topology"]["position"],
        out[1]["topology"]["position"]
    );
    assert_eq!(
        out[2]["topology"]["position"],
        out[3]["topology"]["position"]
    );
    record("regional-conflicts", &b, &index);
}

#[test]
fn merge_topology_pairs_physical_sides_without_discarding_covered_payloads() {
    for rtl in [false, true] {
        let b = table::rewrite(&table::bytes(), |x| {
            if rtl {
                x.replace("<a:tblPr>", "<a:tblPr rtl=\"1\">")
            } else {
                x
            }
        });
        let index = read(&b);
        let out = run(&index, all(&index));
        assert_eq!(
            out.iter()
                .filter(|v| v["topology"]["insideMerge"] == true)
                .count(),
            8
        );
        let mut positions = std::collections::BTreeMap::new();
        for v in &out {
            if v["topology"]["position"]["kind"] != "diagonal" {
                positions
                    .entry(v["topology"]["position"].to_string())
                    .or_insert_with(Vec::new)
                    .push(v);
            }
        }
        assert_eq!(positions.len(), 24);
        assert_eq!(positions.values().filter(|v| v.len() == 2).count(), 12);
        for sides in positions.values().filter(|v| v.len() == 2) {
            assert_eq!(
                sides[0]["topology"]["neighbour"],
                sides[1]["target"]["cell"]
            );
            assert_eq!(
                sides[1]["topology"]["neighbour"],
                sides[0]["target"]["cell"]
            );
        }
        record(if rtl { "merge-rtl" } else { "merge-ltr" }, &b, &index);
    }
}

#[test]
fn theme_line_reference_shares_full_fill_but_merges_stroke_fields_individually() {
    let edges=STYLE_EDGES.iter().map(|e|format!("<a:{e}><a:lnRef idx=\"1\"><a:srgbClr val=\"010000\"><a:redMod val=\"50000\"/></a:srgbClr></a:lnRef></a:{e}>")).collect::<String>();
    let body =
        format!("<a:wholeTbl><a:tcStyle><a:tcBdr>{edges}</a:tcBdr></a:tcStyle></a:wholeTbl>");
    let b = base(&body, true);
    let b = theme_lines(
        &b,
        "<a:ln w=\"22222\" cap=\"rnd\" cmpd=\"dbl\"><a:gradFill><a:gsLst><a:gs pos=\"0\"><a:schemeClr val=\"phClr\"><a:redMod val=\"50000\"/></a:schemeClr></a:gs><a:gs pos=\"100000\"><a:srgbClr val=\"FFFFFF\"/></a:gs></a:gsLst><a:lin ang=\"5400000\"/></a:gradFill><a:prstDash val=\"dash\"/><a:miter lim=\"600000\"/><a:headEnd type=\"triangle\" w=\"lg\"/></a:ln><a:ln/><a:ln/>",
    );
    let b = cells(&b, |_| {
        "<a:lnL w=\"0\"><a:gradFill rotWithShape=\"0\"/><a:custDash/><a:headEnd len=\"sm\"/></a:lnL>".into()
    });
    let index = read(&b);
    let out = run(&index, vec![target(&index, 0, 0, TableCellEdge::Left)]);
    let g = &out[0]["stroke"]["geometry"];
    assert_eq!(width(&out[0]), "0");
    assert_eq!(g["compound"]["value"], "dbl");
    assert_eq!(g["dash"]["stops"], json!([]));
    assert_eq!(g["join"]["limit"]["value"], "600000");
    assert_eq!(g["head"]["kind"]["value"], "triangle");
    assert_eq!(g["head"]["length"]["value"], "sm");
    assert_eq!(g["cap"]["declaredBy"]["kind"], "tableTheme");
    let fill = &out[0]["fill"];
    assert_eq!(
        fill["style"]["fill"]["gradient"]["rotateWithShape"]["value"],
        false
    );
    assert_eq!(
        fill["colors"]["stops"][0]["outcome"]["rgba16"],
        json!([64, 0, 0, 65535])
    );
    assert_eq!(
        fill["colors"]["stops"][0]["placeholder"]["sourcePart"],
        STYLES
    );
    assert_eq!(
        fill["colors"]["stops"][0]["placeholder"]["owner"]["target"]["kind"],
        "tableStyleBorder"
    );
    record("theme-gradient-stroke", &b, &index);
}

#[test]
fn unresolved_styles_and_atomic_limits_keep_precise_diagnostics() {
    let b = inline(
        &table::bytes(),
        &format!("<a:tableStyleId>{ID}</a:tableStyleId>"),
    );
    let index = read(&b);
    let out = run(&index, all(&index));
    assert!(
        out.iter()
            .all(|v| v["stroke"]["reason"]["reason"]["kind"] == "missingDefinition")
    );
    let b = table::bytes();
    let index = read(&b);
    let q = request(&index, all(&index));
    assert!(matches!(
        query(&index, &q, Default::default(), &|| true),
        Err(PptxError::Cancelled)
    ));
    let mut stale = q.clone();
    stale.expected_source_sha256 = mo_common::Digest::from_sha256([0; 32]);
    assert!(matches!(
        query(&index, &stale, Default::default(), &|| false),
        Err(PptxError::SourceConflict(_))
    ));
    let mut limited = TableBorderLimits::default();
    limited.lines.max_steps = 5;
    assert!(matches!(
        query(&index, &q, limited, &|| false),
        Err(PptxError::Limit(_))
    ));
    let count = std::cell::Cell::new(0);
    assert!(matches!(
        query(&index, &q, Default::default(), &|| {
            count.set(count.get() + 1);
            count.get() > 200
        }),
        Err(PptxError::Cancelled)
    ));
    let out = run(&index, vec![target(&index, 99, 0, TableCellEdge::Left)]);
    assert_eq!(
        out[0]["stroke"]["reason"]["reason"]["kind"],
        "cellOutsideGrid"
    );
    assert_eq!(out[0]["topology"], Value::Null);
    let b = table::rewrite(&b, |x| {
        x.replacen(
            "rowSpan=\"2\" gridSpan=\"2\"",
            "rowSpan=\"99\" gridSpan=\"2\"",
            1,
        )
    });
    let index = read(&b);
    let out = run(&index, vec![target(&index, 0, 0, TableCellEdge::Left)]);
    assert_eq!(out[0]["stroke"]["reason"]["kind"], "tableGrid");
}

#[test]
fn unknown_table_or_selected_line_declarations_do_not_become_resolved_strokes() {
    for (b, origin) in [
        (
            table::rewrite(&table::bytes(), |x| {
                x.replacen("<a:tbl>", "<a:tbl unknown=\"1\">", 1)
            }),
            "object",
        ),
        (
            cells(&table::bytes(), |_| "<a:lnL unknown=\"1\"/>".into()),
            "tableCell",
        ),
        (
            base(
                "<a:wholeTbl><a:tcStyle><a:tcBdr><a:left><a:ln unknown=\"1\"/></a:left></a:tcBdr></a:tcStyle></a:wholeTbl>",
                true,
            ),
            "tableStyle",
        ),
    ] {
        let index = read(&b);
        let v = run(&index, vec![target(&index, 0, 0, TableCellEdge::Left)]);
        assert_eq!(v[0]["stroke"]["reason"]["kind"], "retainedContent");
        assert_eq!(v[0]["stroke"]["reason"]["origin"]["kind"], origin);
        assert_eq!(v[0]["fill"]["style"]["status"], "unresolved");
    }
}
