#[allow(dead_code)]
#[path = "table_styles/support.rs"]
mod support;
use mo_opc::{PartName, RewritePlan};
use mo_pptx::{
    PptxError,
    source::{
        color::*,
        fill::{colors::*, resolve::*},
        table::SourceCellAddress,
        *,
    },
};
use serde_json::{Value, json};
use support::{table, *};

fn rewrite_part(b: &[u8], name: &str, f: impl FnOnce(String) -> String) -> Vec<u8> {
    let p = package(b);
    let part = PartName::new(name).unwrap();
    let xml = String::from_utf8(p.read_part(&part, 1 << 24, &|| false).unwrap()).unwrap();
    let mut plan = RewritePlan::new();
    plan.replace_part(part, f(xml).into_bytes()).unwrap();
    plan.to_bytes(&p, &|| false).unwrap()
}
fn cells(b: &[u8], fill: impl Fn(usize) -> String) -> Vec<u8> {
    table::rewrite(b, |mut xml| {
        let mut start = 0;
        for i in 0..9 {
            let a = start + xml[start..].find("<a:tcPr").unwrap();
            let a = a + xml[a..].find('>').unwrap() + 1;
            let b = a + xml[a..].find("</a:tcPr>").unwrap();
            let value = fill(i);
            xml.replace_range(a..b, &value);
            start = a + value.len();
        }
        xml
    })
}
fn solid(hex: &str) -> String {
    format!("<a:solidFill><a:srgbClr val=\"{hex}\"/></a:solidFill>")
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
fn cell(index: &SourceIndex, row: u32, column: u32) -> FillTarget {
    FillTarget::TableCell {
        native_id: table::target(index).native_id,
        cell: SourceCellAddress { row, column },
    }
}
fn request(index: &SourceIndex, targets: Vec<FillTarget>) -> SourceFillColorQuery {
    SourceFillColorQuery {
        expected_source_sha256: index.source_sha256.clone(),
        surface: table::SLIDE.into(),
        targets,
        fill_profile: FillProfile::Drawingml2024DraftV1,
        color_profile: ColorProfile::Ecma3762016DraftV1,
        context: ColorContext::default(),
    }
}
fn run(index: &SourceIndex, targets: Vec<FillTarget>) -> Vec<Value> {
    mo_pptx::source::fill::colors::query(
        index,
        &request(index, targets),
        Default::default(),
        &|| false,
    )
    .unwrap()
    .targets
    .into_iter()
    .map(|v| serde_json::to_value(v).unwrap())
    .collect()
}
fn rgba(v: &Value) -> &Value {
    &v["colors"]["color"]["outcome"]["rgba8"]
}
fn all_cells(index: &SourceIndex) -> Vec<FillTarget> {
    (0..3)
        .flat_map(|r| (0..3).map(move |c| cell(index, r, c)))
        .collect()
}
fn theme(b: &[u8], fill: &str) -> Vec<u8> {
    let index = read(b);
    let part = &index.surfaces[table::SLIDE]
        .theme_selection
        .format
        .as_ref()
        .unwrap()
        .part;
    rewrite_part(b, part, |mut xml| {
        let a = xml.find("<a:fillStyleLst>").unwrap() + "<a:fillStyleLst>".len();
        let z = a + xml[a..].find("</a:fillStyleLst>").unwrap();
        xml.replace_range(a..z, &format!("{fill}<a:noFill/><a:noFill/>"));
        xml
    })
}
fn record(name: &str, b: &[u8], index: &SourceIndex, targets: Vec<FillTarget>) {
    if let Some(dir) = std::env::var_os("MO_TABLE_FILL_EVIDENCE_DIR") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        let q = request(index, targets);
        let out =
            mo_pptx::source::fill::colors::query(index, &q, Default::default(), &|| false).unwrap();
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

#[test]
fn native_cell_fills_keep_covered_cells_and_do_not_change_source() {
    let b = table::bytes();
    let index = read(&b);
    let before = serde_json::to_vec(&index).unwrap();
    let targets = all_cells(&index);
    let out = run(&index, targets.clone());
    for (n, value) in out.iter().enumerate() {
        assert_eq!(
            rgba(value),
            &json!([30 * (n / 3 + 1), 40 * (n % 3 + 1), 200, 255])
        );
        assert_eq!(
            value["style"]["fill"]["declaredBy"]["owner"]["target"],
            serde_json::to_value(&targets[n]).unwrap()
        );
        assert_eq!(value["colors"]["color"]["placeholder"], Value::Null);
    }
    assert_eq!(before, serde_json::to_vec(&index).unwrap());
    record("direct-cells", &b, &index, targets);
}

#[test]
fn native_regions_and_direct_no_fill_keep_table_background_separate() {
    let body = format!(
        "<a:tblBg><a:fill>{}</a:fill></a:tblBg><a:wholeTbl><a:tcStyle><a:fill>{}</a:fill></a:tcStyle></a:wholeTbl><a:firstRow><a:tcStyle><a:fill>{}</a:fill></a:tcStyle></a:firstRow><a:nwCell><a:tcStyle><a:fill>{}</a:fill></a:tcStyle></a:nwCell>",
        solid("FFFFFF"),
        solid("010203"),
        solid("112233"),
        solid("445566")
    );
    for shared in [false, true] {
        let b = table::rewrite(&base(&body, shared), |x| {
            x.replace("<a:tblPr>", "<a:tblPr firstRow=\"1\" firstCol=\"1\">")
        });
        let b = cells(&b, |n| {
            if n == 8 {
                "<a:noFill/>".into()
            } else {
                String::new()
            }
        });
        let index = read(&b);
        let id = table::target(&index).native_id;
        let mut targets = all_cells(&index);
        targets.push(FillTarget::TableBackground { native_id: id });
        let out = run(&index, targets.clone());
        assert_eq!(rgba(&out[0]), &json!([68, 85, 102, 255]));
        assert_eq!(rgba(&out[1]), &json!([17, 34, 51, 255]));
        assert_eq!(rgba(&out[4]), &json!([1, 2, 3, 255]));
        assert_eq!(out[8]["colors"]["kind"], "none");
        assert_eq!(rgba(&out[9]), &json!([255, 255, 255, 255]));
        let origin = &out[0]["style"]["fill"]["declaredBy"];
        assert_eq!(origin["kind"], "tableStyle");
        assert_eq!(origin["part"], if shared { STYLES } else { table::SLIDE });
        assert_eq!(origin["via"]["target"]["region"], "nwCell");
        record(
            if shared {
                "shared-regions"
            } else {
                "inline-regions"
            },
            &b,
            &index,
            targets,
        );
    }
}

#[test]
fn shared_reference_color_is_lazy_precise_and_names_its_actual_part() {
    let style = "<a:wholeTbl><a:tcStyle><a:fillRef idx=\"1\"><a:srgbClr val=\"010000\"><a:redMod val=\"50000\"/></a:srgbClr></a:fillRef></a:tcStyle></a:wholeTbl>";
    let b = theme(
        &base(style, true),
        "<a:solidFill><a:schemeClr val=\"phClr\"><a:redMod val=\"50000\"/></a:schemeClr></a:solidFill>",
    );
    let index = read(&b);
    let out = run(&index, vec![cell(&index, 0, 0)]);
    assert_eq!(rgba(&out[0]), &json!([0, 0, 0, 255]));
    assert_eq!(
        out[0]["colors"]["color"]["outcome"]["rgba16"],
        json!([64, 0, 0, 65535])
    );
    assert_eq!(
        out[0]["colors"]["color"]["placeholder"]["sourcePart"],
        STYLES
    );
    assert_eq!(
        out[0]["colors"]["color"]["placeholder"]["owner"]["target"]["region"],
        "wholeTbl"
    );
    assert_eq!(out[0]["style"]["fill"]["declaredBy"]["kind"], "theme");
    record("shared-reference", &b, &index, all_cells(&index));

    // A complete direct sRGB fill does not sample the table fillRef or fetch
    // its missing format entry. Direct phClr can still use the reference color.
    let b = base(&style.replace("idx=\"1\"", "idx=\"999\""), true);
    let b = cells(&b, |n| {
        if n == 0 {
            solid("123456")
        } else if n == 1 {
            "<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>".into()
        } else {
            String::new()
        }
    });
    let index = read(&b);
    let out = run(
        &index,
        vec![cell(&index, 0, 0), cell(&index, 0, 1), cell(&index, 0, 2)],
    );
    assert_eq!(rgba(&out[0]), &json!([18, 52, 86, 255]));
    assert_eq!(out[0]["colors"]["color"]["placeholder"], Value::Null);
    assert_eq!(
        out[1]["colors"]["color"]["outcome"]["rgba16"],
        json!([129, 0, 0, 65535])
    );
    assert_eq!(out[2]["style"]["reason"]["kind"], "styleIndexOutOfRange");
    record("lazy-reference", &b, &index, all_cells(&index));
}

#[test]
fn gradient_fields_inherit_without_losing_their_native_owner_or_color_context() {
    let body = "<a:wholeTbl><a:tcStyle><a:fillRef idx=\"1\"><a:srgbClr val=\"804020\"/></a:fillRef></a:tcStyle></a:wholeTbl><a:firstRow><a:tcStyle><a:fill><a:gradFill><a:lin ang=\"5400000\"/></a:gradFill></a:fill></a:tcStyle></a:firstRow>";
    let b = theme(
        &base(body, true),
        "<a:gradFill flip=\"x\" rotWithShape=\"1\"><a:gsLst><a:gs pos=\"0\"><a:schemeClr val=\"phClr\"/></a:gs><a:gs pos=\"100000\"><a:schemeClr val=\"phClr\"><a:tint val=\"50000\"/></a:schemeClr></a:gs></a:gsLst><a:lin ang=\"0\" scaled=\"1\"/><a:tileRect/></a:gradFill>",
    );
    let b = table::rewrite(&b, |x| x.replace("<a:tblPr>", "<a:tblPr firstRow=\"1\">"));
    let b = cells(&b, |n| {
        if n == 0 {
            "<a:gradFill rotWithShape=\"0\"/>".into()
        } else {
            String::new()
        }
    });
    let index = read(&b);
    let out = run(&index, vec![cell(&index, 0, 0)]);
    let g = &out[0]["style"]["fill"]["gradient"];
    assert_eq!(g["rotateWithShape"]["value"], false);
    assert_eq!(g["rotateWithShape"]["declaredBy"]["kind"], "declaration");
    assert_eq!(g["shade"]["angle"]["value"], 5400000);
    assert_eq!(
        g["shade"]["angle"]["declaredBy"]["via"]["target"]["region"],
        "firstRow"
    );
    assert_eq!(g["stops"]["declaredBy"]["kind"], "theme");
    assert_eq!(
        out[0]["colors"]["stops"][0]["outcome"]["rgba8"],
        json!([128, 64, 32, 255])
    );
    assert_eq!(
        out[0]["colors"]["stops"][0]["placeholder"]["owner"]["target"]["region"],
        "wholeTbl"
    );
    record("gradient-inheritance", &b, &index, all_cells(&index));
}

#[test]
fn background_no_fill_image_and_required_effects_are_explicit() {
    for idx in [0, 1000] {
        let b = base(
            &format!(
                "<a:tblBg><a:fillRef idx=\"{idx}\"><a:schemeClr val=\"accent1\"/></a:fillRef></a:tblBg>"
            ),
            true,
        );
        let index = read(&b);
        let out = run(
            &index,
            vec![FillTarget::TableBackground {
                native_id: table::target(&index).native_id,
            }],
        );
        assert_eq!(out[0]["colors"]["kind"], "none");
        assert_eq!(out[0]["style"]["fill"]["declaredBy"]["part"], STYLES);
    }
    let body = "<a:tblBg><a:fill><a:blipFill><a:blip r:embed=\"owned-image\"/><a:stretch><a:fillRect/></a:stretch></a:blipFill></a:fill></a:tblBg>";
    let b = base(body, false);
    let index = read(&b);
    let out = run(
        &index,
        vec![FillTarget::TableBackground {
            native_id: table::target(&index).native_id,
        }],
    );
    assert_eq!(out[0]["colors"]["kind"], "imageResourcesRequired");
    assert_eq!(
        out[0]["style"]["fill"]["image"]["embed"]["value"],
        "owned-image"
    );
    assert_eq!(
        out[0]["style"]["fill"]["image"]["embed"]["declaredBy"]["part"],
        table::SLIDE
    );
    let b = base(
        "<a:tblBg><a:effect><a:effectLst><a:glow rad=\"1000\"><a:srgbClr val=\"112233\"/></a:glow></a:effectLst></a:effect></a:tblBg>",
        false,
    );
    let index = read(&b);
    let out = run(
        &index,
        vec![FillTarget::TableBackground {
            native_id: table::target(&index).native_id,
        }],
    );
    assert_eq!(
        out[0]["style"]["reason"]["kind"],
        "effectEvaluationRequired"
    );
}

#[test]
fn a_retained_style_region_is_reported_only_when_its_reference_color_is_used() {
    let b = base(
        "<a:wholeTbl future=\"unknown\"><a:tcStyle><a:fillRef idx=\"999\"><a:srgbClr val=\"112233\"/></a:fillRef></a:tcStyle></a:wholeTbl>",
        true,
    );
    let b = cells(&b, |n| {
        if n == 0 {
            solid("123456")
        } else {
            "<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>".into()
        }
    });
    let index = read(&b);
    let out = run(&index, vec![cell(&index, 0, 0), cell(&index, 0, 1)]);
    assert_eq!(rgba(&out[0]), &json!([18, 52, 86, 255]));
    assert_eq!(
        out[1]["colors"]["color"]["outcome"]["reason"]["kind"],
        "retainedPlaceholderContext"
    );
    assert_eq!(
        out[1]["colors"]["color"]["outcome"]["reason"]["part"],
        STYLES
    );
}

#[test]
fn source_binding_topology_selection_and_shared_budgets_fail_precisely() {
    let b = table::bytes();
    let index = read(&b);
    let q = request(&index, vec![cell(&index, 0, 0)]);
    assert!(matches!(
        mo_pptx::source::fill::colors::query(&index, &q, Default::default(), &|| true),
        Err(PptxError::Cancelled)
    ));
    let out = run(&index, vec![cell(&index, 99, 0)]);
    assert_eq!(out[0]["style"]["reason"]["kind"], "tableStyle");
    assert_eq!(
        out[0]["style"]["reason"]["reason"]["kind"],
        "cellOutsideGrid"
    );
    let q2 = request(&index, all_cells(&index));
    let mut limits = FillColorLimits::default();
    limits.fills.max_values = 140;
    mo_pptx::source::fill::colors::query(&index, &q, limits, &|| false).unwrap();
    assert!(matches!(
        mo_pptx::source::fill::colors::query(&index, &q2, limits, &|| false),
        Err(PptxError::Limit(_))
    ));
    let mut stale = q;
    stale.expected_source_sha256 = mo_common::Digest::from_sha256([0; 32]);
    assert!(matches!(
        mo_pptx::source::fill::colors::query(&index, &stale, Default::default(), &|| false),
        Err(PptxError::SourceConflict(_))
    ));
    let b = table::rewrite(&b, |x| {
        x.replacen(
            "rowSpan=\"2\" gridSpan=\"2\"",
            "rowSpan=\"99\" gridSpan=\"2\"",
            1,
        )
    });
    let index = read(&b);
    let out = run(&index, vec![cell(&index, 0, 0)]);
    assert_eq!(out[0]["style"]["reason"]["kind"], "tableGrid");
    let b = inline(
        &table::bytes(),
        &format!("<a:tableStyleId>{ID}</a:tableStyleId>"),
    );
    let index = read(&b);
    let out = run(&index, vec![cell(&index, 0, 0)]);
    assert_eq!(
        out[0]["style"]["reason"]["reason"]["kind"],
        "missingDefinition"
    );
}
