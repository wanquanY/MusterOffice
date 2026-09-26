mod support;
use mo_opc::{Package, PackageLimits, PartName, RewritePlan};
use mo_pptx::{
    source::{geometry::evaluate::*, *},
    *,
};
use serde_json::{Value, json};
const SLIDE: &str = "/ppt/slides/slide1.xml";
fn index(inner: &str) -> SourceIndex {
    let (d, defaults) = support::input();
    let bytes = export(
        &d,
        &defaults,
        &support::resources(),
        PptxLimits::default(),
        &|| false,
    )
    .unwrap();
    let p = Package::open(
        bytes.as_slice(),
        bytes.len() as u64,
        PackageLimits::default(),
        &|| false,
    )
    .unwrap();
    let part = PartName::new(SLIDE).unwrap();
    let mut xml = String::from_utf8(p.read_part(&part, 1 << 20, &|| false).unwrap()).unwrap();
    let first = xml.find("<p:sp>").unwrap();
    let a = first + xml[first..].find("<p:spPr>").unwrap();
    let b = a + xml[a..].find("</p:spPr>").unwrap() + 9;
    xml.replace_range(a..b, &format!("<p:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"21600\" cy=\"10800\"/></a:xfrm><a:custGeom>{inner}</a:custGeom></p:spPr>"));
    let mut plan = RewritePlan::new();
    plan.replace_part(part, xml.into_bytes()).unwrap();
    let candidate = plan.to_bytes(&p, &|| false).unwrap();
    let p = Package::open(
        candidate.as_slice(),
        candidate.len() as u64,
        PackageLimits::default(),
        &|| false,
    )
    .unwrap();
    inspect_source(&p, SourceLimits::default(), &|| false).unwrap()
}
fn request(index: &SourceIndex) -> SourceGeometryQuery {
    SourceGeometryQuery {
        expected_source_sha256: index.source_sha256.clone(),
        surface: SLIDE.into(),
        objects: vec![index.surfaces[SLIDE].objects[0].native_id],
        profile: GeometryProfile::Drawingml2016PresetsDraftV2,
    }
}
fn outcome(index: &SourceIndex) -> Value {
    serde_json::to_value(
        &query(index, &request(index), GeometryLimits::default(), &|| false)
            .unwrap()
            .objects[0]
            .outcome,
    )
    .unwrap()
}
#[test]
fn all_seventeen_native_operations_are_computed_in_source_order() {
    let cases = [
        ("val 2", 2.0),
        ("*/ 10 3 4", 7.5),
        ("+- 2 4 7", -1.0),
        ("+/ 2 3 2", 2.5),
        ("?: 0 2 3", 3.0),
        ("abs -2", 2.0),
        ("at2 -1 1", 8_100_000.0),
        ("cat2 10 -3 4", -6.0),
        ("cos 10 cd2", -10.0),
        ("max 1 2", 2.0),
        ("min 1 2", 1.0),
        ("mod 2 3 6", 7.0),
        ("pin 4 2 8", 4.0),
        ("sat2 10 -3 4", 8.0),
        ("sin 10 cd4", 10.0),
        ("sqrt -16", 4.0),
        ("tan 10 cd8", 10.0),
    ];
    let guides = cases
        .iter()
        .enumerate()
        .map(|(i, (f, _))| format!("<a:gd name=\"g{i}\" fmla=\"{f}\"/>"))
        .collect::<String>();
    let r = outcome(&index(&format!("<a:gdLst>{guides}</a:gdLst><a:pathLst/>")));
    assert_eq!(r["status"], "resolved");
    for (i, (_, expected)) in cases.iter().enumerate() {
        let value = r["geometry"]["guides"][i]["value"].as_f64().unwrap();
        assert!(
            (value - expected).abs() <= 1e-12 * expected.abs().max(1.0),
            "{i}: {value}"
        );
    }
}
#[test]
fn sequential_redefinitions_bind_prior_values_and_keep_dependencies() {
    let r = outcome(&index(
        "<a:avLst><a:gd name=\"adj\" fmla=\"val 2\"/><a:gd name=\"adj\" fmla=\"+- adj 3 0\"/></a:avLst><a:gdLst><a:gd name=\"g\" fmla=\"*/ adj w 2\"/></a:gdLst><a:pathLst/>",
    ));
    let g = &r["geometry"];
    assert_eq!(g["adjustments"][1]["value"], 5.0);
    assert_eq!(g["guides"][0]["value"], 54_000.0);
    assert_eq!(
        g["adjustments"][1]["dependencies"][0]["origin"],
        g["adjustments"][0]["origin"]
    );
    assert_eq!(
        g["guides"][0]["dependencies"][1],
        json!({"kind":"builtin","name":"w"})
    );
}
#[test]
fn forward_references_and_invalid_formulas_never_synthesize_zero() {
    for (formula, kind, issue) in [
        ("val future", "unknownReference", None),
        ("imaginary 1", "formula", Some("unknownOperation")),
        ("val", "formula", Some("arity")),
        ("val 1 2", "formula", Some("arity")),
        ("*/ 1 2 0", "formula", Some("divisionByZero")),
        ("+/ 1 2 0", "formula", Some("divisionByZero")),
        ("at2 0 0", "formula", Some("undefinedDirection")),
        ("cat2 1 0 0", "formula", Some("undefinedDirection")),
        ("tan 1 cd4", "formula", Some("tangentPole")),
        ("val NaN", "unknownReference", None),
    ] {
        let r = outcome(&index(&format!(
            "<a:gdLst><a:gd name=\"g\" fmla=\"{formula}\"/><a:gd name=\"future\" fmla=\"val 1\"/></a:gdLst><a:pathLst/>"
        )));
        assert_eq!(r["status"], "unresolved", "{formula}");
        assert_eq!(r["reason"]["kind"], kind, "{formula}");
        if let Some(issue) = issue {
            assert_eq!(r["reason"]["issue"], issue, "{formula}");
        }
    }
}
#[test]
fn scalar_coordinates_units_angles_and_all_path_commands_resolve() {
    let r = outcome(&index(
        r#"<a:avLst><a:gd name="adj" fmla="val 100"/></a:avLst><a:ahLst><a:ahXY gdRefX="adj" minX="-20" maxX="w"><a:pos x="adj" y="h"/></a:ahXY><a:ahPolar gdRefR="adj" gdRefAng="adj" minAng="0" maxAng="cd2"><a:pos x="hc" y="vc"/></a:ahPolar></a:ahLst><a:cxnLst><a:cxn ang="cd4"><a:pos x="l" y="b"/></a:cxn></a:cxnLst><a:rect l="l" t="t" r="r" b="b"/><a:pathLst><a:path w="21600" h="10800"><a:moveTo><a:pt x="2.5cm" y="+0001"/></a:moveTo><a:lnTo><a:pt x="wd2" y="hd2"/></a:lnTo><a:arcTo wR="wd2" hR="hd2" stAng="cd4" swAng="-5400000"/><a:quadBezTo><a:pt x="1" y="2"/><a:pt x="3" y="4"/></a:quadBezTo><a:cubicBezTo><a:pt x="1" y="2"/><a:pt x="3" y="4"/><a:pt x="5" y="6"/></a:cubicBezTo><a:close/></a:path></a:pathLst>"#,
    ));
    assert_eq!(r["status"], "resolved");
    let g = &r["geometry"];
    assert_eq!(g["textRect"]["right"], 21600.0);
    assert_eq!(g["connections"][0]["angle"], 5_400_000.0);
    assert_eq!(g["handles"][0]["guideX"], g["adjustments"][0]["origin"]);
    let c = &g["paths"][0]["commands"];
    assert_eq!(c[0]["to"]["x"], 900000.0);
    assert_eq!(c[1]["to"]["x"], 10800.0);
    assert_eq!(c[2]["sweepAngle"], -5_400_000.0);
    assert_eq!(c[3]["to"]["x"], 3.0);
    assert_eq!(c[4]["control2"]["y"], 4.0);
    assert_eq!(c[5]["kind"], "close");
    assert!(g["paths"][0]["fill"].is_null());
}
#[test]
fn unsupported_metadata_and_nonfinite_math_remain_explicit() {
    for (inner, kind) in [
        (
            "<a:gdLst><a:gd name=\"w\" fmla=\"val 3\"/></a:gdLst><a:pathLst/>",
            "reservedGuide",
        ),
        (
            "<a:gdLst><a:gd name=\"\" fmla=\"val 3\"/></a:gdLst><a:pathLst/>",
            "invalidGuideName",
        ),
        (
            "<a:ahLst><a:ahXY gdRefX=\"w\"><a:pos x=\"0\" y=\"0\"/></a:ahXY></a:ahLst><a:pathLst/>",
            "invalidHandleReference",
        ),
        (
            "<a:pathLst><a:path><a:arcTo wR=\"1\" hR=\"1\" stAng=\"2147483648\" swAng=\"0\"/></a:path></a:pathLst>",
            "invalidAngle",
        ),
        (
            "<a:pathLst><a:path><a:moveTo><a:pt x=\"27273042316901\" y=\"0\"/></a:moveTo></a:path></a:pathLst>",
            "invalidCoordinate",
        ),
        (
            "<a:pathLst><a:path owned=\"unknown\"/></a:pathLst>",
            "retainedContent",
        ),
    ] {
        assert_eq!(outcome(&index(inner))["reason"]["kind"], kind);
    }
    let large = "9".repeat(309);
    let r = outcome(&index(&format!(
        "<a:gdLst><a:gd name=\"g\" fmla=\"val {large}\"/></a:gdLst><a:pathLst/>"
    )));
    assert_eq!(r["reason"]["kind"], "numericRange");
}
#[test]
fn budget_and_cancellation_abort_instead_of_returning_partial_objects() {
    let i = index("<a:gdLst><a:gd name=\"g\" fmla=\"val 2\"/></a:gdLst><a:pathLst/>");
    let mut r = request(&i);
    r.objects.push(r.objects[0]);
    for limits in [
        GeometryLimits {
            max_queries: 1,
            ..Default::default()
        },
        GeometryLimits {
            max_values: 1,
            ..Default::default()
        },
        GeometryLimits {
            max_steps: 1,
            ..Default::default()
        },
        GeometryLimits {
            max_lexical_bytes: 4,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            query(&i, &r, limits, &|| false),
            Err(PptxError::Limit(_))
        ));
    }
    assert!(matches!(
        query(&i, &r, GeometryLimits::default(), &|| true),
        Err(PptxError::Cancelled)
    ));
    let a = query(&i, &r, GeometryLimits::default(), &|| false).unwrap();
    assert_eq!(a.objects.len(), 2);
    assert_eq!(
        serde_json::to_value(&a.objects[0]).unwrap(),
        serde_json::to_value(&a.objects[1]).unwrap()
    );
    r.objects = vec![u32::MAX];
    assert!(query(&i, &r, GeometryLimits::default(), &|| false).is_err());
}

fn preset_index(name: &str, overrides: &str) -> SourceIndex {
    let mut i = index(&format!("<a:avLst>{overrides}</a:avLst><a:pathLst/>"));
    let g = i.surfaces.get_mut(SLIDE).unwrap().objects[0]
        .geometry
        .as_mut()
        .unwrap();
    let geometry::SourceGeometryDefinition::Custom(custom) = &g.definition else {
        panic!()
    };
    g.definition = geometry::SourceGeometryDefinition::Preset {
        preset: name.to_owned().try_into().unwrap(),
        adjustments: custom.adjustments.clone(),
    };
    i
}
#[test]
fn every_catalog_preset_evaluates_at_three_aspect_ratios() {
    let catalog: Value = serde_json::from_str(include_str!(
        "../../../components/drawingml-presets/catalog.json"
    ))
    .unwrap();
    let names = catalog["definitions"].as_object().unwrap();
    assert_eq!(names.len(), 187);
    for name in names.keys() {
        let mut i = preset_index(name, "");
        for (w, h) in [
            (1_000_001, 1_000_001),
            (2_000_003, 700_001),
            (700_001, 2_000_003),
        ] {
            let size = &mut i.surfaces.get_mut(SLIDE).unwrap().objects[0]
                .resolution
                .size
                .as_mut()
                .unwrap()
                .value;
            size.width = mo_common::Emu::new(w);
            size.height = mo_common::Emu::new(h);
            let o = outcome(&i);
            assert_eq!(o["status"], "resolved", "{name} {w}x{h}: {o}");
            for path in o["geometry"]["paths"].as_array().unwrap() {
                assert_eq!(path["origin"]["kind"], "preset");
                assert_eq!(path["origin"]["preset"], name.as_str());
            }
        }
    }
}
#[test]
fn preset_adjustments_preserve_physical_override_and_dependency_identity() {
    let i = preset_index(
        "triangle",
        "<a:gd name=\"adj\" fmla=\"val 25000\"/><a:gd name=\"adj\" fmla=\"+- adj 10000 0\"/>",
    );
    let o = outcome(&i);
    assert_eq!(o["status"], "resolved");
    let g = &o["geometry"];
    let a = &g["adjustments"];
    assert_eq!(a[0]["origin"]["kind"], "preset");
    assert_eq!(a[1]["origin"]["kind"], "document");
    assert_eq!(a[2]["value"], 35000.0);
    assert_eq!(a[2]["dependencies"][0]["origin"], a[1]["origin"]);
    assert_eq!(g["handles"][0]["guideX"], a[2]["origin"]);
    assert!(g["guides"].as_array().unwrap().iter().any(|g| {
        g["dependencies"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["origin"] == a[2]["origin"])
    }));
    // The apex follows the document adjustment, retaining native edit semantics.
    assert!(
        g["paths"][0]["commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["to"]["x"] == 7560.0 && c["to"]["y"] == 0.0)
    );
}
#[test]
fn preset_cache_is_query_local_and_charged_and_overrides_do_not_leak() {
    let i = preset_index("rect", "");
    let mut r = request(&i);
    r.objects = vec![r.objects[0]; 8];
    // Repeated evaluation is charged, but immutable template parsing happens once.
    let v = query(
        &i,
        &r,
        GeometryLimits {
            max_steps: 500,
            ..Default::default()
        },
        &|| false,
    )
    .unwrap();
    assert_eq!(v.objects.len(), 8);
    assert!(matches!(
        query(
            &i,
            &r,
            GeometryLimits {
                max_steps: 20,
                ..Default::default()
            },
            &|| false
        ),
        Err(PptxError::Xml(mo_xml::XmlError::Limit(_))) | Err(PptxError::Limit(_))
    ));
    let calls = std::cell::Cell::new(0);
    let result = query(&i, &r, GeometryLimits::default(), &|| {
        calls.set(calls.get() + 1);
        calls.get() > 30
    });
    assert!(matches!(
        result,
        Err(PptxError::Cancelled) | Err(PptxError::Xml(mo_xml::XmlError::Cancelled))
    ));
    let bad = outcome(&preset_index(
        "triangle",
        "<a:gd name=\"adj\" fmla=\"val missing\"/>",
    ));
    assert_eq!(bad["reason"]["origin"]["kind"], "document");
    let good = outcome(&preset_index("triangle", ""));
    assert_eq!(good["geometry"]["adjustments"][0]["value"], 50000.0);
    // Catalog-only ratios must not silently broaden custom geometry semantics.
    assert_eq!(
        outcome(&index(
            "<a:gdLst><a:gd name=\"g\" fmla=\"val wd32\"/></a:gdLst><a:pathLst/>"
        ))["reason"]["kind"],
        "unknownReference"
    );
}
