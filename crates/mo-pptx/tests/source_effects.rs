mod support;
use mo_opc::{Package, PackageLimits, PartName, RewritePlan};
use mo_pptx::{
    source::{effects::*, fill::*, *},
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
fn base() -> Vec<u8> {
    let (d, defaults) = support::input();
    export(
        &d,
        &defaults,
        &support::resources(),
        PptxLimits::default(),
        &|| false,
    )
    .unwrap()
}
fn rewrite(bytes: &[u8], part: &str, change: impl FnOnce(String) -> String) -> Vec<u8> {
    let p = package(bytes);
    let name = PartName::new(part).unwrap();
    let xml = String::from_utf8(p.read_part(&name, 1 << 20, &|| false).unwrap()).unwrap();
    let mut plan = RewritePlan::new();
    plan.replace_part(name, change(xml).into_bytes()).unwrap();
    plan.to_bytes(&p, &|| false).unwrap()
}
fn fixture(fill: &str, style: &str) -> Vec<u8> {
    rewrite(&base(), SLIDE, |mut xml| {
        let first = xml.find("<p:sp>").unwrap();
        let a = first + xml[first..].find("<p:spPr>").unwrap();
        let b = a + xml[a..].find("</p:spPr>").unwrap() + 9;
        xml.replace_range(a..b, &format!("<p:spPr>{fill}</p:spPr>{style}"));
        xml
    })
}
fn inspect(bytes: &[u8]) -> Result<SourceIndex, PptxError> {
    inspect_source(&package(bytes), SourceLimits::default(), &|| false)
}
fn object(index: &SourceIndex) -> &SourceObject {
    &index.surfaces[SLIDE].objects[0]
}

fn color() -> &'static str {
    "<a:srgbClr val=\"123456\"><a:alphaMod val=\"12.123456789%\"/></a:srgbClr>"
}
fn dag(body: &str) -> Vec<u8> {
    fixture(&format!("<a:effectDag>{body}</a:effectDag>"), "")
}
#[test]
fn all_thirty_native_effect_kinds_are_typed_and_bound_to_flat_nodes() {
    let c = color();
    let cases=[
        ("<a:cont/>".into(),"container"),("<a:effect ref=\"  owned  reference  \"/>".into(),"reference"),
        ("<a:alphaBiLevel thresh=\"50%\"/>".into(),"alphaBiLevel"),("<a:alphaCeiling/>".into(),"alphaCeiling"),
        ("<a:alphaFloor/>".into(),"alphaFloor"),(format!("<a:alphaInv>{c}</a:alphaInv>"),"alphaInverse"),
        ("<a:alphaMod><a:cont/></a:alphaMod>".into(),"alphaModulate"),("<a:alphaModFix amt=\"150.123456789%\"/>".into(),"alphaModulateFixed"),
        ("<a:alphaOutset rad=\"-0.000000001cm\"/>".into(),"alphaOutset"),("<a:alphaRepl a=\"100000\"/>".into(),"alphaReplace"),
        ("<a:biLevel thresh=\"0\"/>".into(),"biLevel"),("<a:blend blend=\"screen\"><a:cont/></a:blend>".into(),"blend"),
        ("<a:blur rad=\"0\" grow=\"0\"/>".into(),"blur"),
        (format!("<a:clrChange useA=\"false\"><a:clrFrom>{c}</a:clrFrom><a:clrTo>{c}</a:clrTo></a:clrChange>"),"colorChange"),
        (format!("<a:clrRepl>{c}</a:clrRepl>"),"colorReplace"),(format!("<a:duotone>{c}{c}</a:duotone>"),"duotone"),
        ("<a:fill><a:noFill/></a:fill>".into(),"fill"),("<a:fillOverlay blend=\"mult\"><a:gradFill/></a:fillOverlay>".into(),"fillOverlay"),
        (format!("<a:glow rad=\"27273042316900\">{c}</a:glow>"),"glow"),("<a:grayscl/>".into(),"grayscale"),
        ("<a:hsl hue=\"21599999\" sat=\"-100%\" lum=\"100000\"/>".into(),"hsl"),
        (format!("<a:innerShdw blurRad=\"1234\" dist=\"123\" dir=\"0\">{c}</a:innerShdw>"),"innerShadow"),
        ("<a:lum bright=\"-100000\" contrast=\"12.34%\"/>".into(),"luminance"),
        (format!("<a:outerShdw sx=\"-20%\" sy=\"150.123456789%\" kx=\"-5399999\" ky=\"5399999\" algn=\"br\" rotWithShape=\"false\">{c}</a:outerShdw>"),"outerShadow"),
        (format!("<a:prstShdw prst=\"shdw20\">{c}</a:prstShdw>"),"presetShadow"),
        ("<a:reflection stA=\"100.00%\" stPos=\"0\" endA=\"0\" endPos=\"100000\" fadeDir=\"0\"/>".into(),"reflection"),
        ("<a:relOff tx=\"123456789.0001%\" ty=\"-200000\"/>".into(),"relativeOffset"),
        ("<a:softEdge rad=\"0\"/>".into(),"softEdge"),("<a:tint hue=\"60000\" amt=\"-12.34%\"/>".into(),"tint"),
        ("<a:xfrm tx=\"123.000000001pt\" ty=\"-27273042329600\" sx=\"0\"/>".into(),"transform"),
    ];
    assert_eq!(cases.len(), 30);
    for (xml, kind) in cases {
        let i = inspect(&dag(&xml)).unwrap();
        let surface = &i.surfaces[SLIDE];
        let SourceEffectPropertiesDefinition::Dag { root } =
            object(&i).effects.as_ref().unwrap().definition
        else {
            panic!()
        };
        let SourceEffectDefinition::Container(container) = &surface.effect_nodes[&root].definition
        else {
            panic!()
        };
        let id = container.nodes[0];
        assert!(id > root);
        let v = serde_json::to_value(&surface.effect_nodes[&id]).unwrap();
        assert_eq!(v["definition"]["kind"], kind, "{xml}");
        assert_eq!(v["sourceOrdinal"], id);
        assert!(
            surface
                .effect_nodes
                .values()
                .all(|n| n.retained_ordinals.is_empty())
        );
        if kind == "reference" {
            assert_eq!(v["definition"]["reference"], "owned reference");
        }
        if kind == "glow" {
            assert_eq!(
                v["definition"]["color"]["transforms"][0]["value"],
                "12.123456789%"
            );
        }
    }
}
#[test]
fn ordered_list_and_empty_list_are_distinct_from_absence_and_dag() {
    let absent = inspect(&fixture("", "")).unwrap();
    assert!(object(&absent).effects.is_none());
    let empty = inspect(&fixture("<a:effectLst/>", "")).unwrap();
    assert!(
        object(&empty)
            .effects
            .as_ref()
            .unwrap()
            .is_explicitly_empty_list()
    );
    let empty_dag = inspect(&dag("")).unwrap();
    assert!(
        !object(&empty_dag)
            .effects
            .as_ref()
            .unwrap()
            .is_explicitly_empty_list()
    );
    let c = color();
    let xml = format!(
        "<a:effectLst><a:blur/><a:fillOverlay blend=\"over\"><a:noFill/></a:fillOverlay><a:glow>{c}</a:glow><a:innerShdw>{c}</a:innerShdw><a:outerShdw>{c}</a:outerShdw><a:prstShdw prst=\"shdw1\">{c}</a:prstShdw><a:reflection/><a:softEdge rad=\"0\"/></a:effectLst>"
    );
    let i = inspect(&fixture(&xml, "")).unwrap();
    let p = object(&i).effects.as_ref().unwrap();
    let SourceEffectPropertiesDefinition::List { nodes } = &p.definition else {
        panic!()
    };
    assert_eq!(nodes.len(), 8);
    assert!(nodes.windows(2).all(|w| w[0] < w[1]));
}
#[test]
fn deep_fill_effect_recursion_is_flat_in_source_and_json() {
    let mut xml = "<a:blur/>".to_owned();
    for _ in 0..24 {
        xml = format!(
            "<a:fillOverlay blend=\"over\"><a:blipFill><a:blip r:embed=\"owned\">{xml}</a:blip></a:blipFill></a:fillOverlay>"
        );
    }
    let i = inspect(&dag(&xml)).unwrap();
    assert_eq!(i.surfaces[SLIDE].effect_nodes.len(), 26);
    let raw = serde_json::to_string(&i).unwrap();
    let restored: SourceIndex = mo_common::from_json_str(&raw).unwrap();
    assert_eq!(i, restored);
    for n in i.surfaces[SLIDE].effect_nodes.values() {
        if let SourceEffectDefinition::FillOverlay(v) = &n.definition {
            let SourceFillDefinition::Image(img) = &v.fill.definition else {
                panic!()
            };
            let ids = &img.blip.as_ref().unwrap().effect_nodes;
            assert_eq!(ids.len(), 1);
            assert!(i.surfaces[SLIDE].effect_nodes.contains_key(&ids[0]));
        }
    }
}
#[test]
fn declaration_defaults_are_not_fabricated_and_lexical_precision_is_retained() {
    let i=inspect(&dag("<a:blur/><a:blur rad=\"0\" grow=\"0\"/><a:alphaInv/><a:alphaOutset rad=\"-0.000000001cm\"/><a:relOff tx=\"+001234\"/>" )).unwrap();
    let nodes: Vec<_> = i.surfaces[SLIDE]
        .effect_nodes
        .values()
        .skip(1)
        .map(|n| serde_json::to_value(&n.definition).unwrap())
        .collect();
    assert_eq!(nodes[0]["radius"], Value::Null);
    assert_eq!(nodes[1]["radius"], "0");
    assert_eq!(nodes[1]["grow"], false);
    assert_eq!(nodes[2]["color"], Value::Null);
    assert_eq!(nodes[3]["radius"], "-0.000000001cm");
    assert_eq!(nodes[4]["translateX"], "+001234");
}
#[test]
fn unknown_properties_are_scoped_to_their_effect_node_and_never_mean_no_effects() {
    let i=inspect(&dag("<a:blur future=\"x\"/><a:cont name=\"n\"><a:glow rad=\"0\"><a:srgbClr val=\"123456\" future=\"x\"/></a:glow></a:cont>" )).unwrap();
    let surface = &i.surfaces[SLIDE];
    let nodes: Vec<_> = surface.effect_nodes.values().collect();
    assert!(nodes[0].retained_ordinals.is_empty());
    assert_eq!(nodes[1].retained_ordinals, vec![nodes[1].source_ordinal]);
    assert!(nodes[2].retained_ordinals.is_empty());
    assert_eq!(nodes[3].retained_ordinals.len(), 1);
    let i = inspect(&fixture("<a:effectLst future=\"x\"/>", "")).unwrap();
    assert!(
        !object(&i)
            .effects
            .as_ref()
            .unwrap()
            .is_explicitly_empty_list()
    );
}
#[test]
fn effect_reference_color_and_three_dimensional_style_bindings_are_preserved() {
    let b = fixture(
        "",
        r#"<p:style><a:effectRef idx="4294967295"><a:schemeClr val="accent1"/></a:effectRef></p:style>"#,
    );
    let b = rewrite(&b, "/ppt/theme/theme2.xml", |x| {
        x.replacen(
            "<a:effectLst/>",
            "<a:effectLst/><a:scene3d><owned/></a:scene3d><a:sp3d/>",
            1,
        )
    });
    let i = inspect(&b).unwrap();
    let r = object(&i).effect_reference.as_ref().unwrap();
    assert_eq!(r.index, u32::MAX);
    assert!(r.color.is_some());
    let s = i.themes["/ppt/theme/theme2.xml"]
        .format_scheme
        .as_ref()
        .unwrap()
        .effects[0]
        .effect_style
        .as_ref()
        .unwrap();
    assert!(s.effects.is_explicitly_empty_list());
    assert_eq!(s.retained_ordinals.len(), 2);
}
#[test]
fn native_effect_grammar_rejects_wrong_scopes_choices_order_and_cardinality() {
    let c = color();
    for xml in [
        "<a:alphaBiLevel/>",
        "<a:alphaRepl/>",
        "<a:softEdge/>",
        "<a:effect/>",
        "<a:blur><a:cont/></a:blur>",
        "<a:blend blend=\"over\"/>",
        "<a:alphaMod/>",
        "<a:glow/>",
        "<a:fill/>",
        "<a:fillOverlay><a:noFill/></a:fillOverlay>",
        "<a:fill><a:noFill/><a:solidFill/></a:fill>",
        "<a:alphaMod><a:cont/><a:cont/></a:alphaMod>",
        "<a:owned/>",
    ] {
        assert!(inspect(&dag(xml)).is_err(), "{xml}");
    }
    for xml in [
        format!("<a:duotone>{c}</a:duotone>"),
        format!("<a:duotone>{c}{c}{c}</a:duotone>"),
        format!("<a:alphaInv>{c}{c}</a:alphaInv>"),
        format!("<a:clrChange><a:clrTo>{c}</a:clrTo><a:clrFrom>{c}</a:clrFrom></a:clrChange>"),
    ] {
        assert!(inspect(&dag(&xml)).is_err(), "{xml}");
    }
    for xml in [
        "<a:effectLst><a:alphaCeiling/></a:effectLst>",
        "<a:effectLst><a:blur/><a:blur/></a:effectLst>",
        "<a:effectLst><a:reflection/><a:blur/></a:effectLst>",
        "<a:effectLst/><a:effectDag/>",
        "<a:blipFill><a:blip><a:outerShdw/></a:blip></a:blipFill>",
    ] {
        assert!(inspect(&fixture(xml, "")).is_err(), "{xml}");
    }
}
#[test]
fn all_numeric_ranges_and_native_enums_are_checked_before_publication() {
    for xml in [
        "<a:blur rad=\"-1\"/>",
        "<a:blur rad=\"27273042316901\"/>",
        "<a:blur rad=\"1cm\"/>",
        "<a:blur grow=\"yes\"/>",
        "<a:hsl hue=\"21600000\"/>",
        "<a:hsl hue=\"-1\"/>",
        "<a:hsl sat=\"100.01%\"/>",
        "<a:hsl lum=\"-100001\"/>",
        "<a:reflection stA=\"-1\"/>",
        "<a:reflection stPos=\"100001\"/>",
        "<a:reflection endA=\"100.01%\"/>",
        "<a:xfrm kx=\"5400000\"/>",
        "<a:xfrm ky=\"-5400000\"/>",
        "<a:alphaModFix amt=\"-1\"/>",
        "<a:alphaOutset rad=\"27273042316901\"/>",
        "<a:cont type=\"invalid\"/>",
        "<a:blend blend=\"invalid\"><a:cont/></a:blend>",
        "<a:reflection algn=\"invalid\"/>",
    ] {
        assert!(inspect(&dag(xml)).is_err(), "{xml}");
    }
    for n in [0, 21] {
        assert!(
            inspect(&dag(&format!(
                "<a:prstShdw prst=\"shdw{n}\">{}</a:prstShdw>",
                color()
            )))
            .is_err()
        );
    }
}
#[test]
fn all_preset_shadows_and_blend_modes_are_recognized_without_evaluation() {
    for n in 1..=20 {
        let i = inspect(&dag(&format!(
            "<a:prstShdw prst=\"shdw{n}\">{}</a:prstShdw>",
            color()
        )))
        .unwrap();
        assert_eq!(i.surfaces[SLIDE].effect_nodes.len(), 2);
    }
    for blend in ["over", "mult", "screen", "darken", "lighten"] {
        inspect(&dag(&format!(
            "<a:blend blend=\"{blend}\"><a:cont/></a:blend>"
        )))
        .unwrap();
    }
}
#[test]
fn effect_reads_obey_shared_paint_limits_and_cancellation() {
    let b = dag("<a:cont name=\"a long owned name\"><a:blur/></a:cont>");
    for limits in [
        SourceLimits {
            max_paint_elements: 0,
            ..Default::default()
        },
        SourceLimits {
            max_paint_attribute_bytes: 1,
            ..Default::default()
        },
    ] {
        assert!(inspect_source(&package(&b), limits, &|| false).is_err());
    }
    assert!(matches!(
        inspect_source(&package(&b), SourceLimits::default(), &|| true),
        Err(PptxError::Cancelled)
    ));
}
#[test]
fn actual_text_edit_preserves_flat_effect_catalog_and_nested_fills() {
    let b = dag(&format!(
        "<a:glow>{}</a:glow><a:fillOverlay blend=\"screen\"><a:blipFill><a:blip r:embed=\"owned\"><a:grayscl/></a:blip></a:blipFill></a:fillOverlay>",
        color()
    ));
    let before = inspect(&b).unwrap();
    let o = object(&before);
    let q:SourceTextEdits=serde_json::from_value(json!({"expectedSourceSha256":before.source_sha256,"edits":[{"target":{"part":SLIDE,"objectId":o.native_id,"paragraph":0,"run":0},"expectedText":o.paragraphs[0][0].text,"replacement":"effect graph 中文 & < >"}]})).unwrap();
    let candidate = edit_source_text(&package(&b), &q, SourceLimits::default(), &|| false).unwrap();
    let after = inspect(&candidate).unwrap();
    assert_eq!(
        before.surfaces[SLIDE].effect_nodes,
        after.surfaces[SLIDE].effect_nodes
    );
    assert_eq!(object(&before).effects, object(&after).effects);
}

#[test]
fn selected_mce_effects_keep_physical_bindings_and_extensions_cannot_inject_nodes() {
    let b = fixture(
        r#"<mc:AlternateContent xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:u="urn:owned:future"><mc:Choice Requires="u"><a:effectDag><a:softEdge rad="invalid"/></a:effectDag></mc:Choice><mc:Fallback><a:effectLst><a:blur rad="2"/></a:effectLst></mc:Fallback></mc:AlternateContent><a:extLst><a:ext uri="owned"><a:effectDag><a:softEdge rad="invalid"/></a:effectDag></a:ext></a:extLst>"#,
        "",
    );
    let i = inspect(&b).unwrap();
    let surface = &i.surfaces[SLIDE];
    assert_eq!(surface.effect_nodes.len(), 1);
    let node = surface.effect_nodes.values().next().unwrap();
    assert!(matches!(node.definition, SourceEffectDefinition::Blur(_)));
    let SourceEffectPropertiesDefinition::List { nodes } =
        &object(&i).effects.as_ref().unwrap().definition
    else {
        panic!()
    };
    assert_eq!(nodes, &[node.source_ordinal]);
    let raw = String::from_utf8(
        package(&b)
            .read_part(&PartName::new(SLIDE).unwrap(), 1 << 20, &|| false)
            .unwrap(),
    )
    .unwrap();
    // Physical ordinal includes the inactive Choice, which precedes the selected effect.
    let expected = raw[..raw.find("<a:blur rad=\"2\"").unwrap()]
        .split('<')
        .skip(1)
        .filter(|s| !s.starts_with('/') && !s.starts_with('?'))
        .count();
    assert_eq!(node.source_ordinal as usize, expected);
}
