//! Owned XML fixtures exercise effect inheritance without modifying user files.
#[allow(dead_code)]
#[path = "../../../tools/test-support/source_resource_page.rs"]
mod support;
use mo_presentation_compile::source_page::compile;
use support::*;
const SHADOW: &str = r#"<a:effectLst><a:outerShdw blurRad="40000" dist="23000" dir="5400000" rotWithShape="0"><a:schemeClr val="phClr"><a:alpha val="35000"/></a:schemeClr></a:outerShdw></a:effectLst>"#;
fn file(direct: &str, reference: &str) -> Vec<u8> {
    let shape =
        drawing_shape(20, &solid("CC3300")).replace("</p:spPr>", &format!("{direct}</p:spPr>"));
    let shape = shape.replace("</p:sp>", &format!("{reference}</p:sp>"));
    let b = fixture(&shape);
    let index = read(&b);
    let theme = index.surfaces[SLIDE]
        .theme_selection
        .format
        .as_ref()
        .unwrap()
        .part
        .clone();
    rewrite(&b, &theme, |mut xml| {
        let start = xml.find("<a:effectStyleLst>").unwrap();
        let end =
            start + xml[start..].find("</a:effectStyleLst>").unwrap() + "</a:effectStyleLst>".len();
        xml.replace_range(start..end,&format!("<a:effectStyleLst><a:effectStyle>{SHADOW}</a:effectStyle><a:effectStyle>{SHADOW}</a:effectStyle><a:effectStyle>{SHADOW}</a:effectStyle></a:effectStyleLst>"));
        xml
    })
}
fn reference(index: u32) -> String {
    format!(
        r#"<p:style><a:effectRef idx="{index}"><a:srgbClr val="2255AA"/></a:effectRef></p:style>"#
    )
}
#[test]
fn theme_effect_color_and_alpha_are_resolved_before_group_emission() {
    let index = read(&file("", &reference(2)));
    let page = compile(&index, &request(&index), &|| false).unwrap();
    let groups = &page.raster.scene.opacity_groups;
    assert_eq!(groups.len(), 1);
    let shadow = groups[0].outer_shadow.as_ref().unwrap();
    assert_eq!(shadow.rgba, [34, 85, 170, 89]);
    assert_eq!(shadow.sigma.raw(), 20000i128 << 32);
    assert_eq!(shadow.offset.y.raw(), 23000i128 << 32);
    assert!(shadow.offset.x.raw().abs() < 64);
}
#[test]
fn explicit_empty_list_and_zero_reference_block_theme_effects() {
    for (direct, style) in [("<a:effectLst/>", reference(2)), ("", reference(0))] {
        let index = read(&file(direct, &style));
        assert!(
            compile(&index, &request(&index), &|| false)
                .unwrap()
                .raster
                .scene
                .opacity_groups
                .is_empty()
        );
    }
}
#[test]
fn direct_effects_override_theme_and_keep_own_color() {
    let direct = SHADOW.replace(
        r#"<a:schemeClr val="phClr"><a:alpha val="35000"/></a:schemeClr>"#,
        r#"<a:srgbClr val="FF0000"/>"#,
    );
    let index = read(&file(&direct, &reference(2)));
    assert_eq!(
        compile(&index, &request(&index), &|| false)
            .unwrap()
            .raster
            .scene
            .opacity_groups[0]
            .outer_shadow
            .as_ref()
            .unwrap()
            .rgba,
        [255, 0, 0, 255]
    );
}
#[test]
fn unknown_effects_and_retained_style_content_never_disappear() {
    for direct in [
        r#"<a:effectLst><a:glow rad="100"><a:srgbClr val="FF0000"/></a:glow></a:effectLst>"#,
        r#"<a:effectDag/>"#,
        r#"<a:effectLst><a:outerShdw sx="120000"><a:srgbClr val="FF0000"/></a:outerShdw></a:effectLst>"#,
    ] {
        let index = read(&file(direct, &reference(2)));
        assert!(compile(&index, &request(&index), &|| false).is_err());
    }
    let index = read(&file("", &reference(8)));
    assert!(compile(&index, &request(&index), &|| false).is_err());
}
