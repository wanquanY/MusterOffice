#[allow(dead_code)]
mod support;
use mo_opc::{Package, PartName, RewritePlan};
use mo_pptx::{export, source::inspect_source};

const SLIDE: &str = "/ppt/slides/slide1.xml";
const URI: &str = "{BB962C8B-B14F-4D97-AF65-F5344CB8AC3E}";

fn fixture(payload: &str, uri: &str) -> Vec<u8> {
    rewrite(|xml| {
        let extension = format!("<p:extLst><p:ext uri=\"{uri}\">{payload}</p:ext></p:extLst>");
        xml.replace("</p:cSld>", &format!("{extension}</p:cSld>"))
    })
}

fn rewrite(edit: impl FnOnce(String) -> String) -> Vec<u8> {
    let (document, defaults) = support::input();
    let bytes = export(
        &document,
        &defaults,
        &support::resources(),
        Default::default(),
        &|| false,
    )
    .unwrap();
    let package = Package::open(
        bytes.as_slice(),
        bytes.len() as u64,
        Default::default(),
        &|| false,
    )
    .unwrap();
    let name = PartName::new(SLIDE).unwrap();
    let xml = String::from_utf8(package.read_part(&name, 1 << 20, &|| false).unwrap()).unwrap();
    let mut plan = RewritePlan::new();
    plan.replace_part(name, edit(xml).into_bytes()).unwrap();
    plan.to_bytes(&package, &|| false).unwrap()
}

fn object_fixture(value: &str, extra: &str) -> Vec<u8> {
    rewrite(|mut xml| {
        let at = xml.match_indices("<p:cNvPr ").nth(1).unwrap().0;
        let end = at + xml[at..].find("/>").unwrap();
        let extension = format!(
            ">\
            <a:extLst><a:ext uri=\"{{FF2B5EF4-FFF2-40B4-BE49-F238E27FC236}}\">\
            <a16:creationId xmlns:a16=\"http://schemas.microsoft.com/office/drawing/2014/main\" id=\"{value}\"/>\
            {extra}</a:ext></a:extLst></p:cNvPr>"
        );
        xml.replace_range(end..end + 2, &extension);
        xml
    })
}

#[test]
fn object_creation_metadata_does_not_hide_unknown_siblings() {
    for extra in ["", "<x:visual xmlns:x=\"urn:unknown\"/>"] {
        let bytes = object_fixture("{3FFBDBFC-C28F-4FA7-8E05-59FBC97B4CAF}", extra);
        let package = Package::open(
            bytes.as_slice(),
            bytes.len() as u64,
            Default::default(),
            &|| false,
        )
        .unwrap();
        let index = inspect_source(&package, Default::default(), &|| false).unwrap();
        let objects = &index.surfaces[SLIDE].objects;
        assert_eq!(
            objects.iter().all(|o| o.visual_issues.is_empty()),
            extra.is_empty()
        );
        let retained = package
            .read_part(&PartName::new(SLIDE).unwrap(), 1 << 20, &|| false)
            .unwrap();
        assert!(
            String::from_utf8(retained)
                .unwrap()
                .contains("3FFBDBFC-C28F-4FA7-8E05-59FBC97B4CAF")
        );
    }
}

#[test]
fn malformed_object_creation_identifier_is_rejected() {
    for value in ["", "not-a-guid", "{3FFBDBFC-C28F-4FA7-8E05-59FBC97B4CAZ}"] {
        let bytes = object_fixture(value, "");
        let package = Package::open(
            bytes.as_slice(),
            bytes.len() as u64,
            Default::default(),
            &|| false,
        )
        .unwrap();
        assert!(inspect_source(&package, Default::default(), &|| false).is_err());
    }
}

#[test]
fn slide_creation_id_is_retained_nonvisual_metadata() {
    let payload = "<p14:creationId xmlns:p14=\"http://schemas.microsoft.com/office/powerpoint/2010/main\" val=\"2089702301\"/>";
    let bytes = fixture(payload, URI);
    let package = Package::open(
        bytes.as_slice(),
        bytes.len() as u64,
        Default::default(),
        &|| false,
    )
    .unwrap();
    let index = inspect_source(&package, Default::default(), &|| false).unwrap();
    assert!(index.surfaces[SLIDE].visual_issues.is_empty());
    let retained = package
        .read_part(&PartName::new(SLIDE).unwrap(), 1 << 20, &|| false)
        .unwrap();
    assert!(String::from_utf8(retained).unwrap().contains(payload));
}

#[test]
fn unknown_extensions_and_extra_visual_payload_are_still_reported() {
    for (payload, uri) in [
        ("<x:visual xmlns:x=\"urn:unknown\"/>", "urn:unknown"),
        ("<x:visual xmlns:x=\"urn:unknown\"/>", URI),
        (
            "<p14:creationId xmlns:p14=\"http://schemas.microsoft.com/office/powerpoint/2010/main\" val=\"1\" extra=\"1\"/>",
            URI,
        ),
    ] {
        let bytes = fixture(payload, uri);
        let package = Package::open(
            bytes.as_slice(),
            bytes.len() as u64,
            Default::default(),
            &|| false,
        )
        .unwrap();
        let index = inspect_source(&package, Default::default(), &|| false).unwrap();
        assert!(!index.surfaces[SLIDE].visual_issues.is_empty());
    }
}

#[test]
fn malformed_creation_identifier_is_rejected() {
    for value in ["bad", "-1", "4294967296"] {
        let bytes = fixture(
            &format!(
                "<p14:creationId xmlns:p14=\"http://schemas.microsoft.com/office/powerpoint/2010/main\" val=\"{value}\"/>"
            ),
            URI,
        );
        let package = Package::open(
            bytes.as_slice(),
            bytes.len() as u64,
            Default::default(),
            &|| false,
        )
        .unwrap();
        assert!(inspect_source(&package, Default::default(), &|| false).is_err());
    }
}
