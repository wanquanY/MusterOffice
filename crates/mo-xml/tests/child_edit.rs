use mo_xml::*;
fn name() -> ExpandedName {
    ExpandedName {
        namespace: "urn:parent".into(),
        local: "root".into(),
    }
}
const CHILD: &str = "<q:title xmlns:q='urn:child'>中&lt;&amp;🚀</q:title>";
#[test]
fn append_retains_original_bytes_and_handles_empty_parents_in_all_encodings() {
    for (input, expected) in [
        (
            "<p:root xmlns:p='urn:parent' a = 'keep'><!--keep--><f:x xmlns:f='urn:future'/></p:root>",
            format!(
                "<p:root xmlns:p='urn:parent' a = 'keep'><!--keep--><f:x xmlns:f='urn:future'/>{CHILD}</p:root>"
            ),
        ),
        (
            "<p:root xmlns:p='urn:parent' />",
            format!("<p:root xmlns:p='urn:parent' >{CHILD}</p:root>"),
        ),
    ] {
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
            let output = append_child(
                &encode(input),
                0,
                &name(),
                CHILD,
                Default::default(),
                &|| false,
            )
            .unwrap();
            assert_eq!(output, encode(&expected));
        }
    }
}
#[test]
fn append_rejects_unbound_children_wrong_parent_capacity_and_cancellation() {
    let input = b"<root xmlns='urn:parent'/>";
    for invalid in [
        "<q:unbound/>",
        "<a/><b/>",
        "<?xml version='1.0'?><a/>",
        "<?keep p?><a/>",
        "<!DOCTYPE x><x/>",
        "<a/>",
        "<q:a xmlns:q='urn:a'><b/></q:a>",
    ] {
        assert!(append_child(input, 0, &name(), invalid, Default::default(), &|| false).is_err());
    }
    assert!(
        append_child(
            input,
            0,
            &name(),
            "<a xmlns=''/>",
            Default::default(),
            &|| false
        )
        .is_ok()
    );
    assert!(append_child(input, 2, &name(), CHILD, Default::default(), &|| false).is_err());
    assert!(
        append_child(
            input,
            0,
            &name(),
            CHILD,
            XmlLimits {
                max_bytes: 80,
                ..Default::default()
            },
            &|| false
        )
        .is_err()
    );
    assert!(matches!(
        append_child(input, 0, &name(), CHILD, Default::default(), &|| true),
        Err(XmlError::Cancelled)
    ));
}
