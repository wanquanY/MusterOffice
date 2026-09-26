use mo_xml::*;
use std::cell::Cell;

fn edit(ordinal: usize, old: &str, new: &str) -> TextReplacement {
    TextReplacement {
        element_ordinal: ordinal,
        expected_name: ExpandedName {
            namespace: "urn:a".into(),
            local: "t".into(),
        },
        expected_text: old.into(),
        replacement: new.into(),
    }
}
fn rewrite(input: &[u8], edits: &[TextReplacement]) -> Result<Vec<u8>, XmlError> {
    rewrite_text(input, edits, TextRewriteLimits::default(), &|| false)
}

#[test]
fn text_rewrite_retains_unknown_attributes_siblings_comments_and_pi_bytes() {
    let source = b"<?xml version='1.0'?><a:r xmlns:a='urn:a' xmlns:f='urn:future'>\r\n<a:t f:keep = 'opaque'>old&amp;<![CDATA[ text]]><!--keep--><?future data?>tail</a:t><f:node untouched='yes'/></a:r>";
    let output = rewrite(source, &[edit(1, "old& texttail", "new <&>\r\n中🚀")]).unwrap();
    let expected = "<?xml version='1.0'?><a:r xmlns:a='urn:a' xmlns:f='urn:future'>\r\n<a:t f:keep = 'opaque'>new &lt;&amp;&gt;&#xD;\n中🚀<!--keep--><?future data?></a:t><f:node untouched='yes'/></a:r>";
    assert_eq!(output, expected.as_bytes());
    let mut texts = String::new();
    scan(&output, XmlLimits::default(), |event| {
        if let XmlEvent::Text { text, depth: 2, .. } = event {
            texts.push_str(text);
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(texts, "new <&>\r\n中🚀");
}

#[test]
fn empty_elements_expand_without_dropping_attributes_and_textless_comments_survive() {
    let source = b"<r xmlns='urn:a'><t future='x' /><t><!--retain--></t><t>old</t></r>";
    let output = rewrite(
        source,
        &[
            edit(1, "", "first"),
            edit(2, "", "second"),
            edit(3, "old", ""),
        ],
    )
    .unwrap();
    assert_eq!(
        output,
        b"<r xmlns='urn:a'><t future='x' >first</t><t>second<!--retain--></t><t></t></r>"
    );
}

#[test]
fn preconditions_and_nested_elements_prevent_wrong_or_destructive_edits() {
    let source = b"<r xmlns='urn:a'><t>old</t><t><future/>old</t></r>";
    for edits in [
        vec![edit(1, "wrong", "new")],
        vec![edit(0, "", "new")],
        vec![edit(2, "old", "new")],
        vec![edit(99, "", "new")],
        vec![edit(1, "old", "a"), edit(1, "old", "b")],
    ] {
        assert!(matches!(
            rewrite(source, &edits),
            Err(XmlError::EditConflict(_))
        ));
    }
    assert!(rewrite(source, &[edit(1, "old", "\0")]).is_err());
}

fn encode(text: &str, encoding: XmlEncoding, bom: bool) -> Vec<u8> {
    let mut bytes = match (encoding, bom) {
        (XmlEncoding::Utf8, true) => vec![0xEF, 0xBB, 0xBF],
        (XmlEncoding::Utf16Le, true) => vec![0xFF, 0xFE],
        (XmlEncoding::Utf16Be, true) => vec![0xFE, 0xFF],
        _ => vec![],
    };
    if encoding == XmlEncoding::Utf8 {
        bytes.extend_from_slice(text.as_bytes());
    } else {
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&if encoding == XmlEncoding::Utf16Le {
                unit.to_le_bytes()
            } else {
                unit.to_be_bytes()
            });
        }
    }
    bytes
}

#[test]
fn utf8_and_utf16_encodings_boms_and_original_lexical_forms_are_preserved() {
    for encoding in [
        XmlEncoding::Utf8,
        XmlEncoding::Utf16Le,
        XmlEncoding::Utf16Be,
    ] {
        for bom in [false, true] {
            let declaration = if encoding == XmlEncoding::Utf8 {
                "UTF-8"
            } else {
                "UTF-16"
            };
            let source = format!(
                "<?xml version='1.0' encoding='{declaration}'?><r xmlns='urn:a' keep='é🚀'><t>A&amp;B</t><t>é</t><!--中--></r>"
            );
            let bytes = encode(&source, encoding, bom);
            let output = rewrite(&bytes, &[edit(1, "A&B", "C🚀"), edit(2, "é", "é")]).unwrap();
            assert_eq!(
                output,
                encode(
                    &source.replace("A&amp;B", "C🚀").replace(">é<", ">é<"),
                    encoding,
                    bom
                )
            );
            assert_eq!(rewrite(&bytes, &[]).unwrap(), bytes);
            assert_eq!(rewrite(&bytes, &[edit(1, "A&B", "A&B")]).unwrap(), bytes);
        }
    }
}

#[test]
fn rewriting_honors_span_output_and_cancellation_budgets() {
    let bytes = b"<r xmlns='urn:a'><t>&amp;&lt;</t></r>";
    let edits = [edit(1, "&<", "changed")];
    let spans = TextRewriteLimits {
        max_text_spans: 1,
        ..TextRewriteLimits::default()
    };
    assert!(matches!(
        rewrite_text(bytes, &edits, spans, &|| false),
        Err(XmlError::Limit(_))
    ));
    let limits = TextRewriteLimits {
        xml: XmlLimits {
            max_bytes: bytes.len(),
            ..XmlLimits::default()
        },
        ..TextRewriteLimits::default()
    };
    assert!(matches!(
        rewrite_text(bytes, &[edit(1, "&<", "&&&&&&&&")], limits, &|| false),
        Err(XmlError::Limit(_))
    ));
    let calls = Cell::new(0);
    let cancelled = || {
        calls.set(calls.get() + 1);
        calls.get() > 4
    };
    assert!(matches!(
        rewrite_text(bytes, &edits, TextRewriteLimits::default(), &cancelled),
        Err(XmlError::Cancelled)
    ));
}
