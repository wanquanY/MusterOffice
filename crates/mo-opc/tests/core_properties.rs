use mo_opc::*;
const CORE: &str = "http://schemas.openxmlformats.org/package/2006/metadata/core-properties";
const DC: &str = "http://purl.org/dc/elements/1.1/";
fn n(s: &str) -> PartName {
    PartName::new(s).unwrap()
}
fn fixture(core: Option<&[u8]>) -> Vec<u8> {
    let mut p = PackageBuilder::new();
    p.add_part(
        n("/document.xml"),
        "application/xml".into(),
        b"<document/>".to_vec(),
    )
    .unwrap();
    p.add_part(
        n("/media/opaque.bin"),
        "application/octet-stream".into(),
        vec![0, 255, 1],
    )
    .unwrap();
    let mut rels = vec![
        Relationship::new(
            &RelationshipSource::Package,
            "main".into(),
            "urn:main".into(),
            "/document.xml".into(),
            false,
        )
        .unwrap(),
    ];
    if let Some(core) = core {
        p.add_part(
            n("/metadata/properties.xml"),
            CORE_PROPERTIES_TYPE.into(),
            core.to_vec(),
        )
        .unwrap();
        rels.push(
            Relationship::new(
                &RelationshipSource::Package,
                "core".into(),
                CORE_PROPERTIES_RELATIONSHIP.into(),
                "metadata/properties.xml".into(),
                false,
            )
            .unwrap(),
        );
    }
    p.set_relationships(RelationshipSource::Package, rels)
        .unwrap();
    p.to_bytes(Default::default(), &|| false).unwrap()
}
fn open(b: &[u8]) -> Package<&[u8]> {
    Package::open(b, b.len() as u64, Default::default(), &|| false).unwrap()
}
#[test]
fn title_edit_preserves_unknown_core_properties_and_unrelated_part_bytes() {
    let xml = format!(
        "<c:coreProperties xmlns:c='{CORE}' xmlns:d='{DC}' xmlns:f='urn:future'><d:creator>Someone</d:creator><!--stable--><d:title f:keep = 'yes'>old&amp;<![CDATA[old]]><!--inside--></d:title><f:opaque keep='all'/></c:coreProperties>"
    );
    for mode in 0..3 {
        let encode = |s: &str| match mode {
            0 => s.as_bytes().to_vec(),
            1 => [
                vec![0xff, 0xfe],
                s.encode_utf16().flat_map(u16::to_le_bytes).collect(),
            ]
            .concat(),
            _ => [
                vec![0xfe, 0xff],
                s.encode_utf16().flat_map(u16::to_be_bytes).collect(),
            ]
            .concat(),
        };
        let input = fixture(Some(&encode(&xml)));
        let source = open(&input);
        assert_eq!(
            read_core_properties(&source, Default::default(), &|| false)
                .unwrap()
                .unwrap()
                .title,
            "old&old"
        );
        let mut edit = RewritePlan::new();
        edit.set_core_title(&source, "中<&>\r\n🚀", &|| false)
            .unwrap();
        let bytes = edit.to_bytes(&source, &|| false).unwrap();
        let result = open(&bytes);
        let properties = read_core_properties(&result, Default::default(), &|| false)
            .unwrap()
            .unwrap();
        assert_eq!(properties.title, "中<&>\r\n🚀");
        assert_eq!(
            result
                .read_part(&properties.part, 1 << 20, &|| false)
                .unwrap(),
            encode(&xml.replace("old&amp;<![CDATA[old]]>", "中&lt;&amp;&gt;&#xD;\n🚀"))
        );
        for (part, info) in source.parts() {
            if *part != properties.part {
                assert_eq!(result.parts()[part].sha256, info.sha256);
            }
        }
        assert_eq!(result.relationships(), source.relationships());
    }
}
#[test]
fn absent_title_and_absent_core_part_have_typed_graph_additions_and_no_op_identity() {
    for xml in [
        None,
        Some(format!(
            "<c:coreProperties xmlns:c='{CORE}'><!--keep--><c:keywords>stable</c:keywords></c:coreProperties>"
        )),
        Some(format!("<c:coreProperties xmlns:c='{CORE}'/>")),
    ] {
        let input = fixture(xml.as_ref().map(|s| s.as_bytes()));
        let source = open(&input);
        let mut unchanged = RewritePlan::new();
        unchanged.set_core_title(&source, "", &|| false).unwrap();
        assert_eq!(unchanged.to_bytes(&source, &|| false).unwrap(), input);
        let mut edit = RewritePlan::new();
        edit.set_core_title(&source, "Added <& title", &|| false)
            .unwrap();
        let result = edit.to_bytes(&source, &|| false).unwrap();
        let actual = open(&result);
        assert_eq!(
            read_core_properties(&actual, Default::default(), &|| false)
                .unwrap()
                .unwrap()
                .title,
            "Added <& title"
        );
        assert_eq!(
            actual.parts()[&n("/document.xml")].sha256,
            source.parts()[&n("/document.xml")].sha256
        );
        assert_eq!(
            actual.parts()[&n("/media/opaque.bin")].sha256,
            source.parts()[&n("/media/opaque.bin")].sha256
        );
        assert_eq!(
            actual.relationships()[&RelationshipSource::Package]
                .iter()
                .filter(|r| r.relationship_type == CORE_PROPERTIES_RELATIONSHIP)
                .count(),
            1
        );
        assert_eq!(
            actual.content_types().content_type(
                &read_core_properties(&actual, Default::default(), &|| false)
                    .unwrap()
                    .unwrap()
                    .part
            ),
            Some(CORE_PROPERTIES_TYPE)
        );
    }
}
#[test]
fn title_rejects_ambiguous_structured_corrupt_and_conflicting_material() {
    for content in [
        "<d:title>one</d:title><d:title>two</d:title>",
        "<d:title><d:nested/></d:title>",
    ] {
        let input = fixture(Some(
            format!(
                "<c:coreProperties xmlns:c='{CORE}' xmlns:d='{DC}'>{content}</c:coreProperties>"
            )
            .as_bytes(),
        ));
        assert!(read_core_properties(&open(&input), Default::default(), &|| false).is_err());
    }
    let bytes = fixture(None);
    let source = open(&bytes);
    let mut edit = RewritePlan::new();
    edit.set_core_title(&source, "new", &|| false).unwrap();
    assert!(edit.set_core_title(&source, "other", &|| false).is_err());
    let other = fixture(Some(
        format!("<c:coreProperties xmlns:c='{CORE}'/>").as_bytes(),
    ));
    assert!(edit.to_bytes(&open(&other), &|| false).is_err());
    assert!(matches!(
        RewritePlan::new().set_core_title(&source, "x", &|| true),
        Err(OpcError::Cancelled)
    ));
    assert!(
        RewritePlan::new()
            .set_core_title(&source, "bad\u{0}", &|| false)
            .is_err()
    );
}

#[test]
fn missing_metadata_graph_supports_nonstandard_names_and_capacity_limits() {
    for occupied in [
        "/docProps",
        "/docProps/core.xml",
        "/docProps/core.xml/child",
    ] {
        let mut builder = PackageBuilder::new();
        builder
            .add_part(n(occupied), "application/octet-stream".into(), vec![42])
            .unwrap();
        let bytes = builder.to_bytes(Default::default(), &|| false).unwrap();
        let source = open(&bytes);
        assert!(source.relationships().is_empty());
        let mut edit = RewritePlan::new();
        edit.set_core_title(&source, "created", &|| false).unwrap();
        let output = edit.to_bytes(&source, &|| false).unwrap();
        let result = open(&output);
        assert_eq!(
            result.parts()[&n(occupied)].sha256,
            source.parts()[&n(occupied)].sha256
        );
        assert_eq!(
            read_core_properties(&result, Default::default(), &|| false)
                .unwrap()
                .unwrap()
                .title,
            "created"
        );
        let mut erase = RewritePlan::new();
        erase.set_core_title(&result, "", &|| false).unwrap();
        let erased = erase.to_bytes(&result, &|| false).unwrap();
        assert_eq!(
            read_core_properties(&open(&erased), Default::default(), &|| false)
                .unwrap()
                .unwrap()
                .title,
            ""
        );
        let limits = PackageLimits {
            max_parts: 1,
            ..Default::default()
        };
        let bounded =
            Package::open(bytes.as_slice(), bytes.len() as u64, limits, &|| false).unwrap();
        assert!(matches!(
            RewritePlan::new().set_core_title(&bounded, "x", &|| false),
            Err(OpcError::Limit(_))
        ));
    }
}
