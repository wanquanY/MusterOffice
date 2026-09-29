use mo_xml::*;

fn removal(ordinal: usize, local: &str) -> ElementRemoval {
    ElementRemoval {
        element_ordinal: ordinal,
        expected_name: ExpandedName {
            namespace: "urn:test".into(),
            local: local.into(),
        },
    }
}

#[test]
fn removes_selected_subtrees_preserving_unaffected_bytes_and_encodings() {
    let input = "<?xml version=\"1.0\"?><r xmlns='urn:test'>\n<!--保留--><x a='1'/><keep>原文</keep><x>remove<child/></x></r>";
    let expected = "<?xml version=\"1.0\"?><r xmlns='urn:test'>\n<!--保留--><keep>原文</keep></r>";
    for encode in [utf8 as fn(&str) -> Vec<u8>, utf16le, utf16be] {
        let source = encode(input);
        let output = remove_elements(
            &source,
            &[removal(1, "x"), removal(3, "x")],
            XmlLimits::default(),
            &|| false,
        )
        .unwrap();
        assert_eq!(output, encode(expected));
        assert_eq!(
            remove_elements(&source, &[], XmlLimits::default(), &|| false).unwrap(),
            source
        );
    }
}

#[test]
fn refuses_root_missing_changed_duplicate_overlapping_and_cancelled_requests() {
    let source = b"<r xmlns='urn:test'><x><child/></x></r>";
    for request in [
        vec![removal(0, "r")],
        vec![removal(9, "x")],
        vec![removal(1, "y")],
        vec![removal(1, "x"), removal(1, "x")],
        vec![removal(1, "x"), removal(2, "child")],
    ] {
        assert!(matches!(
            remove_elements(source, &request, XmlLimits::default(), &|| false),
            Err(XmlError::EditConflict(_))
        ));
    }
    assert!(matches!(
        remove_elements(source, &[], XmlLimits::default(), &|| true),
        Err(XmlError::Cancelled)
    ));
    assert!(remove_elements(b"<r><broken></r>", &[], XmlLimits::default(), &|| false).is_err());
}

fn utf8(s: &str) -> Vec<u8> {
    [vec![0xef, 0xbb, 0xbf], s.as_bytes().to_vec()].concat()
}
fn utf16le(s: &str) -> Vec<u8> {
    [
        vec![0xff, 0xfe],
        s.encode_utf16().flat_map(u16::to_le_bytes).collect(),
    ]
    .concat()
}
fn utf16be(s: &str) -> Vec<u8> {
    [
        vec![0xfe, 0xff],
        s.encode_utf16().flat_map(u16::to_be_bytes).collect(),
    ]
    .concat()
}
