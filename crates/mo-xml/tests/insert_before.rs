use mo_xml::*;
#[test]
fn ordered_insertion_preserves_utf16_source_spans_and_namespaces() {
    let source = "<r xmlns=\"urn:source\"><!--keep--><size value=\"原文\"/></r>";
    let bytes: Vec<u8> = [0xff, 0xfe]
        .into_iter()
        .chain(source.encode_utf16().flat_map(u16::to_le_bytes))
        .collect();
    let expected = ExpandedName {
        namespace: "urn:source".into(),
        local: "size".into(),
    };
    let inserted = "<p:list xmlns:p=\"urn:new\"><p:item/></p:list>";
    let result = insert_before(&bytes, 1, &expected, inserted, Default::default(), &|| {
        false
    })
    .unwrap();
    let decoded = String::from_utf16(
        &result[2..]
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    assert_eq!(
        decoded,
        source.replace("<size", &format!("{inserted}<size"))
    );
    assert!(
        insert_before(
            &bytes,
            0,
            &ExpandedName {
                namespace: "urn:source".into(),
                local: "r".into()
            },
            inserted,
            Default::default(),
            &|| false
        )
        .is_err()
    );
    assert!(insert_before(&bytes, 1, &expected, inserted, Default::default(), &|| true).is_err());
}
