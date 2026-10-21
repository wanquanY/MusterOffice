//! Original XML fixtures for native master/layout header/footer availability.
#[allow(dead_code)]
#[path = "../../../tools/test-support/source_resource_page.rs"]
mod support;
use mo_opc::{Package, PackageLimits};
use mo_pptx::source::{SourceLimits, inspect_source};
use mo_presentation_compile::source_page::{SourcePageError, SourcePageIssue, compile};
use support::*;
const MASTER: &str = "/ppt/slideMasters/slideMaster2.xml";

fn declared(declaration: &str, placeholder: &str) -> Vec<u8> {
    let bytes = rewrite(&fixture(""), SLIDE, |s| {
        s.replace("showMasterSp=\"0\"", "showMasterSp=\"1\"")
    });
    rewrite(&bytes, MASTER, |mut s| {
        let start = s.find("<p:cSld").unwrap();
        let end = s.find("</p:cSld>").unwrap() + "</p:cSld>".len();
        s.replace_range(start..end, r#"<p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>"#);
        let shape = drawing_shape(500, &solid("CC3300")).replace(
            "<p:nvPr/>",
            &format!("<p:nvPr><p:ph type=\"{placeholder}\"/></p:nvPr>"),
        );
        s.replace("</p:spTree>", &(shape + "</p:spTree>")).replace(
            "</p:sldMaster>",
            &(declaration.to_owned() + "</p:sldMaster>"),
        )
    })
}

#[test]
fn explicit_disabled_master_placeholders_produce_no_ink_but_source_is_retained() {
    for (attribute, kind) in [
        ("dt", "dt"),
        ("ftr", "ftr"),
        ("hdr", "hdr"),
        ("sldNum", "sldNum"),
    ] {
        for disabled in ["0", "false"] {
            let bytes = declared(&format!("<p:hf {attribute}=\"{disabled}\"/>"), kind);
            let index = read(&bytes);
            let source = &index.surfaces[MASTER];
            assert!(source.visual_issues.is_empty());
            assert!(source.header_footer.is_some());
            assert!(source.objects.iter().any(|o| o.native_id == 500));
            let plan = compile(&index, &request(&index), &|| false).unwrap();
            let layer = plan
                .info
                .layers
                .iter()
                .find(|layer| layer.part == MASTER)
                .unwrap();
            assert!(layer.hidden_objects.contains(&500));
            assert!(!layer.objects.contains(&500));
        }
    }
}

#[test]
fn enabled_template_footer_declarations_supply_inheritance_without_extra_ink() {
    for declaration in [
        "",
        "<p:hf/>",
        "<p:hf ftr=\"1\"/>",
        "<p:hf ftr=\"true\"/>",
        "<p:hf dt=\"0\"/>",
    ] {
        let bytes = declared(declaration, "ftr");
        let index = read(&bytes);
        let plan = compile(&index, &request(&index), &|| false).unwrap();
        let master = plan.info.layers.iter().find(|l| l.part == MASTER).unwrap();
        assert!(master.template_placeholders.contains(&500));
        assert!(!master.objects.contains(&500));
        assert!(!master.hidden_objects.contains(&500));
    }
}

#[test]
fn malformed_duplicate_and_unknown_declarations_are_not_whitelisted() {
    for declaration in ["<p:hf ftr=\"no\"/>", "<p:hf/><p:hf/>"] {
        let bytes = declared(declaration, "ftr");
        let package = Package::open(
            bytes.as_slice(),
            bytes.len() as u64,
            PackageLimits::default(),
            &|| false,
        )
        .unwrap();
        assert!(inspect_source(&package, SourceLimits::default(), &|| false).is_err());
    }
    for declaration in ["<p:hf custom=\"0\"/>", "<p:hf><p:unknown/></p:hf>"] {
        let bytes = declared(declaration, "ftr");
        let index = read(&bytes);
        assert!(compile(&index, &request(&index), &|| false).is_err());
    }
}

#[test]
fn nearest_layout_declaration_controls_inherited_footer_availability() {
    for (master, layout, hidden) in [("0", "1", false), ("1", "0", true), ("0", "", false)] {
        let bytes = declared(&format!("<p:hf ftr=\"{master}\"/>"), "ftr");
        let index = read(&bytes);
        let layout_part = index.surfaces[SLIDE].links.layout.as_ref().unwrap();
        let declaration = if layout.is_empty() {
            "<p:hf/>".to_owned()
        } else {
            format!("<p:hf ftr=\"{layout}\"/>")
        };
        let bytes = rewrite(&bytes, layout_part, |s| {
            s.replace("</p:sldLayout>", &(declaration + "</p:sldLayout>"))
        });
        let index = read(&bytes);
        let result = compile(&index, &request(&index), &|| false);
        if hidden {
            assert!(
                result
                    .unwrap()
                    .info
                    .layers
                    .iter()
                    .find(|l| l.part == MASTER)
                    .unwrap()
                    .hidden_objects
                    .contains(&500)
            );
        } else {
            assert!(
                result
                    .unwrap()
                    .info
                    .layers
                    .iter()
                    .find(|l| l.part == MASTER)
                    .unwrap()
                    .template_placeholders
                    .contains(&500)
            );
        }
    }
}

#[test]
fn ordinary_footer_templates_do_not_create_implicit_slide_content() {
    for kind in ["dt", "ftr", "sldNum"] {
        let bytes = declared("", kind);
        let index = read(&bytes);
        let layout = index.surfaces[SLIDE].links.layout.as_ref().unwrap();
        let declaration = drawing_shape(501, &solid("00CC33")).replace(
            "<p:nvPr/>",
            &format!("<p:nvPr><p:ph type=\"{kind}\" idx=\"50\"/></p:nvPr>"),
        );
        let bytes = rewrite(&bytes, layout, |s| {
            s.replace("</p:spTree>", &(declaration + "</p:spTree>"))
        });
        let index = read(&bytes);
        let plan = compile(&index, &request(&index), &|| false).unwrap();
        assert!(
            plan.info
                .layers
                .iter()
                .find(|l| l.part == MASTER)
                .unwrap()
                .template_placeholders
                .contains(&500)
        );
        assert!(
            plan.info
                .layers
                .iter()
                .find(|l| l.part == *layout)
                .unwrap()
                .template_placeholders
                .contains(&501)
        );

        let instance = drawing_shape(502, &solid("0033CC")).replace(
            "<p:nvPr/>",
            &format!("<p:nvPr><p:ph type=\"{kind}\" idx=\"50\"/></p:nvPr>"),
        );
        let bytes = rewrite(&bytes, SLIDE, |s| {
            s.replace("</p:spTree>", &(instance + "</p:spTree>"))
        });
        let index = read(&bytes);
        let plan = compile(&index, &request(&index), &|| false).unwrap();
        let slide = plan.info.layers.iter().find(|l| l.part == SLIDE).unwrap();
        assert!(slide.objects.contains(&502));
        assert!(!slide.template_placeholders.contains(&502));
    }
}

#[test]
fn notes_only_special_placeholders_remain_explicitly_unsupported() {
    for kind in ["hdr", "sldImg"] {
        let index = read(&declared("", kind));
        assert!(matches!(compile(&index, &request(&index), &|| false),
            Err(SourcePageError::Mapping { issue, .. }) if matches!(*issue, SourcePageIssue::SpecialPlaceholder {})));
    }
}
