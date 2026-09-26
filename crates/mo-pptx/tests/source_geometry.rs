mod support;
use mo_opc::{Package, PackageLimits, PartName, RewritePlan};
use mo_pptx::{
    source::{geometry::*, *},
    *,
};
use serde_json::{Value, json};
const SLIDE: &str = "/ppt/slides/slide1.xml";
fn package(bytes: &[u8]) -> Package<&[u8]> {
    Package::open(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })
    .unwrap()
}
fn fixture(geometry: &str) -> Vec<u8> {
    let (d, defaults) = support::input();
    let bytes = export(
        &d,
        &defaults,
        &support::resources(),
        PptxLimits::default(),
        &|| false,
    )
    .unwrap();
    let p = package(&bytes);
    let part = PartName::new(SLIDE).unwrap();
    let mut xml = String::from_utf8(p.read_part(&part, 1 << 20, &|| false).unwrap()).unwrap();
    let first = xml.find("<p:sp>").unwrap();
    let a = first + xml[first..].find("<p:spPr>").unwrap();
    let b = a + xml[a..].find("</p:spPr>").unwrap() + 9;
    xml.replace_range(a..b, &format!("<p:spPr>{geometry}</p:spPr>"));
    let mut plan = RewritePlan::new();
    plan.replace_part(part, xml.into_bytes()).unwrap();
    plan.to_bytes(&p, &|| false).unwrap()
}
fn inspect(bytes: &[u8]) -> Result<SourceIndex, PptxError> {
    inspect_source(&package(bytes), SourceLimits::default(), &|| false)
}
fn geometry(index: &SourceIndex) -> Value {
    serde_json::to_value(index.surfaces[SLIDE].objects[0].geometry.as_ref()).unwrap()
}
const COMPLETE: &str = r#"<a:custGeom><a:avLst><a:gd name="adj" fmla="val +0001"/><a:gd name="adj" fmla="val 2"/></a:avLst><a:gdLst><a:gd name="width" fmla="*/ w adj 100000"/></a:gdLst><a:ahLst><a:ahXY gdRefX="adj" minX="-20" maxX="width"><a:pos x="+0001" y="2.5cm"/></a:ahXY><a:ahPolar gdRefR="adj" minR="0" gdRefAng="angle" maxAng="cd2"><a:pos x="w" y="h"/></a:ahPolar></a:ahLst><a:cxnLst><a:cxn ang="cd4"><a:pos x="hc" y="vc"/></a:cxn></a:cxnLst><a:rect l="l" t="t" r="r" b="b"/><a:pathLst><a:path w="0" h="21600" fill="lightenLess" stroke="0" extrusionOk="false"><a:moveTo><a:pt x="-0007" y="0"/></a:moveTo><a:lnTo><a:pt x="w" y="h"/></a:lnTo><a:arcTo wR="wd2" hR="hd2" stAng="cd4" swAng="-5400000"/><a:quadBezTo><a:pt x="1" y="2"/><a:pt x="3" y="4"/></a:quadBezTo><a:cubicBezTo><a:pt x="1" y="2"/><a:pt x="3" y="4"/><a:pt x="5" y="6"/></a:cubicBezTo><a:close/></a:path><a:path/></a:pathLst></a:custGeom>"#;
#[test]
fn complete_geometry_keeps_every_command_handle_connection_and_lexical_value() {
    let index = inspect(&fixture(COMPLETE)).unwrap();
    let g = geometry(&index);
    let d = &g["definition"];
    assert_eq!(d["kind"], "custom");
    assert_eq!(d["adjustments"]["entries"][0]["formula"], "val +0001");
    assert_eq!(d["adjustments"]["entries"][1]["name"], "adj");
    assert_eq!(d["handles"]["entries"][0]["position"]["y"], "2.5cm");
    assert_eq!(d["handles"]["entries"][1]["kind"], "polar");
    assert_eq!(d["connections"]["entries"][0]["angle"], "cd4");
    assert_eq!(d["textRect"]["right"], "r");
    let path = &d["paths"]["entries"][0];
    assert_eq!(path["width"], "0");
    assert_eq!(path["fill"], "lightenLess");
    assert_eq!(path["stroke"], false);
    assert_eq!(
        path["commands"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["kind"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["move", "line", "arc", "quadratic", "cubic", "close"]
    );
    assert_eq!(path["commands"][0]["to"]["x"], "-0007");
    assert_eq!(path["commands"][4]["control2"]["x"], "3");
    assert_eq!(path["commands"][4]["to"]["x"], "5");
    assert!(g["retainedOrdinals"].as_array().unwrap().is_empty());
}
#[test]
fn absent_geometry_optional_lists_and_path_defaults_remain_distinct() {
    assert!(geometry(&inspect(&fixture("")).unwrap()).is_null());
    let preset = geometry(&inspect(&fixture("<a:prstGeom prst=\"rect\"/>")).unwrap());
    assert!(preset["definition"]["adjustments"].is_null());
    let preset = geometry(
        &inspect(&fixture(
            "<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom>",
        ))
        .unwrap(),
    );
    assert_eq!(preset["definition"]["adjustments"]["entries"], json!([]));
    let g = geometry(
        &inspect(&fixture(
            "<a:custGeom><a:pathLst><a:path/></a:pathLst></a:custGeom>",
        ))
        .unwrap(),
    );
    let p = &g["definition"]["paths"]["entries"][0];
    for name in ["width", "height", "fill", "stroke", "extrusionOk"] {
        assert!(p[name].is_null());
    }
    let index = inspect(&fixture("<a:prstGeom prst=\"ellipse\"/>")).unwrap();
    let SourceGeometryDefinition::Preset { preset, .. } = &index.surfaces[SLIDE].objects[0]
        .geometry
        .as_ref()
        .unwrap()
        .definition
    else {
        panic!()
    };
    assert_eq!(preset.name(), "ellipse");
}
#[test]
fn invalid_geometry_grammar_and_extents_are_rejected() {
    for xml in [
        "<a:prstGeom/>",
        "<a:prstGeom prst=\"imaginary\"/>",
        "<a:prstGeom prst=\"rect\"><a:gdLst/></a:prstGeom>",
        "<a:custGeom/>",
        "<a:custGeom><a:pathLst/><a:avLst/></a:custGeom>",
        "<a:custGeom><a:pathLst/><a:pathLst/></a:custGeom>",
        "<a:custGeom><a:gdLst><a:gd name=\"g\"/></a:gdLst><a:pathLst/></a:custGeom>",
        "<a:custGeom><a:ahLst><a:ahXY/></a:ahLst><a:pathLst/></a:custGeom>",
        "<a:custGeom><a:cxnLst><a:cxn ang=\"0\"/></a:cxnLst><a:pathLst/></a:custGeom>",
        "<a:prstGeom prst=\"rect\"/><a:prstGeom prst=\"ellipse\"/>",
    ] {
        assert!(inspect(&fixture(xml)).is_err(), "{xml}");
    }
    for inside in [
        "<a:moveTo/>",
        "<a:lnTo><a:pt x=\"0\"/></a:lnTo>",
        "<a:quadBezTo><a:pt x=\"0\" y=\"0\"/></a:quadBezTo>",
        "<a:cubicBezTo><a:pt x=\"0\" y=\"0\"/><a:pt x=\"0\" y=\"0\"/></a:cubicBezTo>",
        "<a:close><a:pt x=\"0\" y=\"0\"/></a:close>",
        "<a:arcTo wR=\"1\" hR=\"1\" stAng=\"0\"/>",
        "not geometry",
    ] {
        assert!(
            inspect(&fixture(&format!(
                "<a:custGeom><a:pathLst><a:path>{inside}</a:path></a:pathLst></a:custGeom>"
            )))
            .is_err(),
            "{inside}"
        );
    }
    for attr in [
        "w=\"-1\"",
        "h=\"27273042316901\"",
        "fill=\"invented\"",
        "stroke=\"yes\"",
        "extrusionOk=\"TRUE\"",
    ] {
        assert!(
            inspect(&fixture(&format!(
                "<a:custGeom><a:pathLst><a:path {attr}/></a:pathLst></a:custGeom>"
            )))
            .is_err()
        );
    }
}
#[test]
fn formulas_are_preserved_without_pretending_they_have_been_evaluated() {
    let g=geometry(&inspect(&fixture("<a:custGeom><a:gdLst><a:gd name=\"\" fmla=\"not a known formula\"/></a:gdLst><a:pathLst><a:path><a:moveTo><a:pt x=\"unknown guide\" y=\"999999999999999999999999999\"/></a:moveTo></a:path></a:pathLst></a:custGeom>")).unwrap());
    assert_eq!(
        g["definition"]["guides"]["entries"][0]["formula"],
        "not a known formula"
    );
    assert_eq!(
        g["definition"]["paths"]["entries"][0]["commands"][0]["to"]["y"],
        "999999999999999999999999999"
    );
}
#[test]
fn unknown_attributes_bind_their_own_nodes_and_extension_decoys_do_not_escape() {
    let xml = "<a:custGeom owned=\"root\"><a:pathLst><a:path><a:moveTo><a:pt x=\"0\" y=\"0\" owned=\"point\"/></a:moveTo></a:path></a:pathLst></a:custGeom>";
    let g = geometry(&inspect(&fixture(xml)).unwrap());
    assert_eq!(
        g["retainedOrdinals"],
        json!([
            g["sourceOrdinal"],
            g["definition"]["paths"]["entries"][0]["commands"][0]["to"]["sourceOrdinal"]
        ])
    );
    let g=geometry(&inspect(&fixture("<a:prstGeom prst=\"rect\"/><a:extLst><a:ext uri=\"owned\"><a:prstGeom prst=\"invalid decoy\"/></a:ext></a:extLst>")).unwrap());
    assert_eq!(g["definition"]["preset"], "rect");
}
#[test]
fn geometry_budgets_cover_package_work_and_cancellation() {
    let bytes = fixture(COMPLETE);
    let p = package(&bytes);
    for limits in [
        SourceLimits {
            max_geometry_elements: 2,
            ..Default::default()
        },
        SourceLimits {
            max_geometry_attribute_bytes: 10,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            inspect_source(&p, limits, &|| false),
            Err(PptxError::Xml(mo_xml::XmlError::Limit(_)))
        ));
    }
    assert!(matches!(
        inspect_source(&p, SourceLimits::default(), &|| true),
        Err(PptxError::Cancelled)
    ));
}
#[test]
fn geometry_element_budget_is_shared_across_the_whole_package() {
    fn nodes(value: &Value) -> usize {
        match value {
            Value::Object(object) => {
                usize::from(object.contains_key("sourceOrdinal"))
                    + object.values().map(nodes).sum::<usize>()
            }
            Value::Array(array) => array.iter().map(nodes).sum(),
            _ => 0,
        }
    }
    let bytes = fixture(COMPLETE);
    let index = inspect(&bytes).unwrap();
    let count = index
        .surfaces
        .values()
        .flat_map(|surface| &surface.objects)
        .filter_map(|object| object.geometry.as_ref())
        .map(|g| nodes(&serde_json::to_value(g).unwrap()))
        .sum::<usize>();
    let p = package(&bytes);
    assert!(count > nodes(&geometry(&index)));
    let limits = SourceLimits {
        max_geometry_elements: count,
        ..Default::default()
    };
    assert!(inspect_source(&p, limits, &|| false).is_ok());
    assert!(matches!(
        inspect_source(
            &p,
            SourceLimits {
                max_geometry_elements: count - 1,
                ..limits
            },
            &|| false
        ),
        Err(PptxError::Xml(mo_xml::XmlError::Limit("geometry elements")))
    ));
}
#[test]
fn actual_text_edit_preserves_geometry_declarations_and_source_ordinals() {
    let bytes = fixture(COMPLETE);
    let p = package(&bytes);
    let index = inspect(&bytes).unwrap();
    let object = &index.surfaces[SLIDE].objects[0];
    let edits = SourceTextEdits {
        expected_source_sha256: index.source_sha256.clone(),
        edits: vec![SourceTextEdit {
            target: SourceTextTarget {
                part: SLIDE.into(),
                object_id: object.native_id,
                paragraph: 0,
                run: 0,
            },
            expected_text: object.paragraphs[0][0].text.clone(),
            replacement: "Geometry stays native 中文".into(),
        }],
    };
    let candidate = edit_source_text(&p, &edits, SourceLimits::default(), &|| false).unwrap();
    assert_eq!(geometry(&inspect(&candidate).unwrap()), geometry(&index));
}
