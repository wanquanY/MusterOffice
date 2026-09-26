use mo_xml::*;
use std::cell::Cell;

fn name(ns: &str, local: &str) -> ExpandedName {
    ExpandedName {
        namespace: ns.into(),
        local: local.into(),
    }
}
fn edit(
    ordinal: usize,
    attribute: ExpandedName,
    old: Option<&str>,
    new: Option<&str>,
) -> AttributeEdit {
    AttributeEdit {
        element_ordinal: ordinal,
        expected_element: name("urn:a", "t"),
        attribute,
        expected_value: old.map(str::to_owned),
        value: new.map(str::to_owned),
        insertion_name: None,
    }
}
fn apply(input: &[u8], edits: &[AttributeEdit]) -> Result<Vec<u8>, XmlError> {
    rewrite_attributes(input, edits, AttributeRewriteLimits::default(), &|| false)
}
fn encode(text: &str, encoding: XmlEncoding, bom: bool) -> Vec<u8> {
    let mut out = match (encoding, bom) {
        (XmlEncoding::Utf8, true) => vec![0xef, 0xbb, 0xbf],
        (XmlEncoding::Utf16Le, true) => vec![0xff, 0xfe],
        (XmlEncoding::Utf16Be, true) => vec![0xfe, 0xff],
        _ => vec![],
    };
    if encoding == XmlEncoding::Utf8 {
        out.extend_from_slice(text.as_bytes());
    } else {
        for n in text.encode_utf16() {
            out.extend_from_slice(&if encoding == XmlEncoding::Utf16Le {
                n.to_le_bytes()
            } else {
                n.to_be_bytes()
            });
        }
    }
    out
}

#[test]
fn edits_preserve_encoding_bom_quotes_entities_unknown_nodes_and_all_other_bytes() {
    for enc in [
        XmlEncoding::Utf8,
        XmlEncoding::Utf16Le,
        XmlEncoding::Utf16Be,
    ] {
        for bom in [false, true] {
            let declaration = if enc == XmlEncoding::Utf8 {
                "UTF-8"
            } else {
                "UTF-16"
            };
            let source = format!(
                "<?xml version='1.0' encoding='{declaration}'?><a:r xmlns:a='urn:a' xmlns:f='urn:f'>\r\n<a:t f:keep = 'opaque&amp;x' x = 'old&#65;' y=\"drop\" z='&amp;' ><![CDATA[<child/>]]><!--中🚀--><?x keep?><f:future/></a:t></a:r>"
            );
            let mut edits = vec![
                edit(1, name("", "x"), Some("oldA"), Some("<&>\"'\r\n\t中🚀")),
                edit(1, name("", "y"), Some("drop"), None),
                edit(1, name("", "z"), Some("&"), Some("&")),
                edit(1, name("urn:f", "new"), None, Some("")),
            ];
            edits[3].insertion_name = Some("f:new".into());
            let expected = source
                .replace("'old&#65;'", "'&lt;&amp;>\"&apos;&#xD;&#xA;&#x9;中🚀'")
                .replace("y=\"drop\"", "")
                .replace("z='&amp;' >", "z='&amp;'  f:new=\"\">");
            let bytes = encode(&source, enc, bom);
            let result = apply(&bytes, &edits).unwrap();
            assert_eq!(result, encode(&expected, enc, bom));
            assert_eq!(apply(&bytes, &[]).unwrap(), bytes);
            assert_eq!(
                apply(
                    &bytes,
                    &[edit(1, name("", "x"), Some("oldA"), Some("oldA"))]
                )
                .unwrap(),
                bytes
            );
            assert_eq!(
                apply(&bytes, &[edit(1, name("", "missing"), None, None)]).unwrap(),
                bytes
            );
        }
    }
}

#[test]
fn normalized_values_and_quote_escaping_recover_the_exact_requested_unicode() {
    let source = b"<r xmlns='urn:a'><t a='A\r\nB\tC&#xD;&#xA;&#x9;&quot;' b=\"O&apos;K\"/></r>";
    let output = apply(
        source,
        &[
            edit(
                1,
                name("", "a"),
                Some("A B C\r\n\t\""),
                Some("\t\n\r'\"<&>"),
            ),
            edit(1, name("", "b"), Some("O'K"), Some("\"'")),
        ],
    )
    .unwrap();
    assert_eq!(
        output,
        b"<r xmlns='urn:a'><t a='&#x9;&#xA;&#xD;&apos;\"&lt;&amp;>' b=\"&quot;'\"/></r>"
    );
    scan(&output, XmlLimits::default(), |event| {
        if let XmlEvent::Start { element, .. } = event
            && element.name.local == "t"
        {
            assert_eq!(element.attribute("a"), Some("\t\n\r'\"<&>"));
            assert_eq!(element.attribute("b"), Some("\"'"));
        }
        Ok(())
    })
    .unwrap();
}

#[test]
fn insertions_are_deterministic_and_bind_in_the_target_namespace_scope() {
    let source=b"<r xmlns='urn:a' xmlns:p='urn:one' xmlns:q='urn:one'><t xmlns:p='urn:two' q:a='old' /></r>";
    let mut added = edit(1, name("urn:two", "b"), None, Some("new"));
    added.insertion_name = Some("p:b".into());
    let first = vec![
        added.clone(),
        edit(1, name("urn:one", "a"), Some("old"), Some("changed")),
        edit(1, name("", "z"), None, Some("plain")),
    ];
    let mut second = first.clone();
    second.reverse();
    let actual = apply(source, &first).unwrap();
    assert_eq!(actual, apply(source, &second).unwrap());
    assert_eq!(actual,b"<r xmlns='urn:a' xmlns:p='urn:one' xmlns:q='urn:one'><t xmlns:p='urn:two' q:a='changed'  z=\"plain\" p:b=\"new\"/></r>");
    for wrong in [
        None,
        Some("q:b"),
        Some("unbound:b"),
        Some("p:other"),
        Some("p:b:c"),
    ] {
        added.insertion_name = wrong.map(str::to_owned);
        assert!(apply(source, &[added.clone()]).is_err());
    }
}

#[test]
fn atomic_conflicts_never_turn_absence_or_empty_values_into_an_unchecked_edit() {
    let source = b"<r xmlns='urn:a'><t x='' y='old'/><t x='later'/></r>";
    for change in [
        edit(1, name("", "x"), None, Some("new")),
        edit(1, name("", "missing"), Some(""), None),
        edit(1, name("", "y"), Some("wrong"), Some("new")),
        edit(99, name("", "x"), None, None),
        edit(0, name("", "x"), None, Some("new")),
    ] {
        assert!(matches!(
            apply(
                source,
                &[edit(2, name("", "x"), Some("later"), Some("early")), change]
            ),
            Err(XmlError::EditConflict(_))
        ));
    }
    let duplicate = edit(1, name("", "x"), Some(""), None);
    assert!(matches!(
        apply(source, &[duplicate.clone(), duplicate]),
        Err(XmlError::EditConflict(_))
    ));
    let output = apply(
        source,
        &[
            edit(1, name("", "x"), Some(""), None),
            edit(1, name("", "y"), Some("old"), Some("")),
        ],
    )
    .unwrap();
    assert_eq!(output, b"<r xmlns='urn:a'><t  y=''/><t x='later'/></r>");
}

#[test]
fn namespace_declarations_and_invalid_characters_cannot_change_unrelated_meanings() {
    let source = b"<r xmlns='urn:a'><t/></r>";
    for attr in [
        name("", "xmlns"),
        name("http://www.w3.org/2000/xmlns/", "p"),
        name("", "x:y"),
        name("", "bad name"),
    ] {
        assert!(apply(source, &[edit(1, attr, None, Some("urn:other"))]).is_err());
    }
    for bad in ["\0", "\u{1}", "\u{fffe}"] {
        assert!(apply(source, &[edit(1, name("", "x"), None, Some(bad))]).is_err());
    }
    let mut xml = edit(
        1,
        name("http://www.w3.org/XML/1998/namespace", "space"),
        None,
        Some("preserve"),
    );
    xml.insertion_name = Some("xml:space".into());
    assert!(apply(source, &[xml]).is_ok());
    for malformed in [
        b"<r xmlns='urn:a'><t x='1' x='2'/></r>".as_slice(),
        b"<!DOCTYPE r [<!ENTITY e 'x'>]><r xmlns='urn:a'><t x='&e;'/></r>",
        b"<r xmlns='urn:a'><t x='<bad'/></r>",
    ] {
        assert!(apply(malformed, &[]).is_err());
    }
}

#[test]
fn adjacent_deletions_and_insertions_preserve_valid_self_closing_and_nonempty_tags() {
    for tail in ["/>", " ></t>"] {
        let source = format!("<r xmlns='urn:a'><t a = '1' b=\"2\" c='3'{tail}</r>");
        let output = apply(
            source.as_bytes(),
            &[
                edit(1, name("", "a"), Some("1"), None),
                edit(1, name("", "b"), Some("2"), None),
                edit(1, name("", "c"), Some("3"), None),
                edit(1, name("", "d"), None, Some("4")),
            ],
        )
        .unwrap();
        let expected = source
            .replace("a = '1'", "")
            .replace("b=\"2\"", "")
            .replace("c='3'", "")
            .replace(
                tail,
                if tail == "/>" {
                    " d=\"4\"/>"
                } else {
                    "  d=\"4\"></t>"
                },
            );
        assert_eq!(output, expected.as_bytes());
    }
}

#[test]
fn physical_ordinals_include_ignored_and_alternate_markup_without_flattening() {
    let source=b"<r xmlns='urn:a' xmlns:m='urn:mc'><m:AlternateContent><m:Choice><t x='choice'/></m:Choice><m:Fallback><t x='fallback'/></m:Fallback></m:AlternateContent></r>";
    let output = apply(
        source,
        &[edit(5, name("", "x"), Some("fallback"), Some("new"))],
    )
    .unwrap();
    assert_eq!(
        output,
        String::from_utf8(source.to_vec())
            .unwrap()
            .replace("'fallback'", "'new'")
            .as_bytes()
    );
}

#[test]
fn edit_input_result_attribute_and_cancellation_limits_apply_atomically() {
    let source = b"<r xmlns='urn:a'><t x='old'/></r>";
    let edits = [edit(1, name("", "x"), Some("old"), Some("&&&&&&&&"))];
    for limits in [
        AttributeRewriteLimits {
            max_edits: 0,
            ..Default::default()
        },
        AttributeRewriteLimits {
            max_edit_bytes: 1,
            ..Default::default()
        },
        AttributeRewriteLimits {
            xml: XmlLimits {
                max_bytes: source.len(),
                ..Default::default()
            },
            ..Default::default()
        },
        AttributeRewriteLimits {
            xml: XmlLimits {
                max_attribute_bytes: 12,
                ..Default::default()
            },
            ..Default::default()
        },
    ] {
        assert!(matches!(
            rewrite_attributes(source, &edits, limits, &|| false),
            Err(XmlError::Limit(_))
        ));
    }
    let count = Cell::new(0usize);
    let checks = || {
        count.set(count.get() + 1);
        false
    };
    rewrite_attributes(source, &edits, Default::default(), &checks).unwrap();
    for stop in 1..=count.get() {
        let n = Cell::new(0);
        let check = || {
            n.set(n.get() + 1);
            n.get() >= stop
        };
        assert!(
            matches!(
                rewrite_attributes(source, &edits, Default::default(), &check),
                Err(XmlError::Cancelled)
            ),
            "checkpoint {stop}"
        );
    }
}
