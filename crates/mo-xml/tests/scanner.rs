use mo_xml::*;

fn checked(text: &str) -> Result<XmlSummary, XmlError> {
    scan(text.as_bytes(), XmlLimits::default(), |_| Ok(()))
}

#[test]
fn prefixes_are_resolved_and_default_namespaces_do_not_apply_to_attributes() {
    let mut names = Vec::new();
    scan(br#"<r xmlns="urn:root" xmlns:p="urn:one"><p:c a="1" p:a="2"/><c xmlns="urn:child"/><c/></r>"#, XmlLimits::default(), |event| {
        if let XmlEvent::Start { element, .. } = event { names.push(element.clone()); }
        Ok(())
    }).unwrap();
    assert!(names[0].name.is("urn:root", "r"));
    assert!(names[1].name.is("urn:one", "c"));
    assert!(names[1].attributes[0].name.is("", "a"));
    assert!(names[1].attributes[1].name.is("urn:one", "a"));
    assert!(names[2].name.is("urn:child", "c"));
    assert!(names[3].name.is("urn:root", "c"));
}

#[test]
fn character_data_and_attributes_follow_xml_normalization_without_unicode_folding() {
    let mut content = String::new();
    let mut value = String::new();
    scan(
        "<r a=\"a\r\nb&#xA;c\">e\u{301}&amp;😀&#x4e2d;<![CDATA[<raw>\r\n]]></r>".as_bytes(),
        XmlLimits::default(),
        |event| {
            match event {
                XmlEvent::Start { element, .. } => value = element.attribute("a").unwrap().into(),
                XmlEvent::Text { text, .. } => content.push_str(text),
                _ => (),
            }
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(value, "a b\nc");
    assert_eq!(content, "e\u{301}&😀中<raw>\n");
}

#[test]
fn dtd_external_entities_and_malformed_documents_are_rejected() {
    for xml in [
        "<!DOCTYPE r SYSTEM 'file:///secret'><r/>",
        "<!DOCTYPE r [<!ENTITY x 'abc'>]><r>&x;</r>",
        "<r>&unknown;</r>",
        "<r>&#0;</r>",
        "<r a='&#0;'/>",
        "<r/><s/>",
        "<r>",
        "text<r/>",
        "<r><!-- a--b --></r>",
        "<r>]]></r>",
        "<r p:a='1'/>",
        "<1invalid/>",
        "<p:r xmlns:p='urn:r' xmlns:q='urn:r' p:a='1' q:a='2'/>",
        "<r xmlns:xml='wrong'/>",
        "<r xmlns:p=''/>",
        "<r xmlns:xmlns='urn:illegal'/>",
        "<r a='1' a='2'/>",
        "<r a='<bad'/>",
        "<?xml version='1.1'?><r/>",
        " <r/><?xml version='1.0'?>",
    ] {
        assert!(checked(xml).is_err(), "accepted {xml}");
    }
}

#[test]
fn limits_bound_depth_elements_attributes_and_text() {
    for (xml, limits) in [
        (
            "<r><a/></r>",
            XmlLimits {
                max_depth: 1,
                ..XmlLimits::default()
            },
        ),
        (
            "<r><a/></r>",
            XmlLimits {
                max_elements: 1,
                ..XmlLimits::default()
            },
        ),
        (
            "<r xmlns:p='u' a='1'/>",
            XmlLimits {
                max_attributes: 1,
                ..XmlLimits::default()
            },
        ),
        (
            "<r a='123'/>",
            XmlLimits {
                max_attribute_bytes: 2,
                ..XmlLimits::default()
            },
        ),
        (
            "<r>ab&amp;c</r>",
            XmlLimits {
                max_text_bytes: 3,
                ..XmlLimits::default()
            },
        ),
        (
            "<r/>",
            XmlLimits {
                max_bytes: 3,
                ..XmlLimits::default()
            },
        ),
    ] {
        assert!(matches!(
            scan(xml.as_bytes(), limits, |_| Ok(())),
            Err(XmlError::Limit(_))
        ));
    }
}

#[test]
fn utf16_and_bom_utf8_are_supported_without_lossy_replacement() {
    let xml = "<?xml version='1.0' encoding='UTF-16'?><页>😀</页>";
    for big in [false, true] {
        let mut input = if big {
            vec![0xFE, 0xFF]
        } else {
            vec![0xFF, 0xFE]
        };
        input.extend(xml.encode_utf16().flat_map(|c| {
            if big {
                c.to_be_bytes()
            } else {
                c.to_le_bytes()
            }
        }));
        let mut content = String::new();
        let report = scan(&input, XmlLimits::default(), |event| {
            if let XmlEvent::Text { text, .. } = event {
                content.push_str(text);
            }
            Ok(())
        })
        .unwrap();
        assert_eq!(content, "😀");
        assert_eq!(
            report.encoding,
            if big {
                XmlEncoding::Utf16Be
            } else {
                XmlEncoding::Utf16Le
            }
        );
    }
    assert!(scan(b"\xEF\xBB\xBF<r/>", XmlLimits::default(), |_| Ok(())).is_ok());
    for invalid in [
        b"\xFF\xFE\x00\xD8".as_slice(),
        b"\xFF\xFE\x01",
        b"<r>\xFF</r>",
        b"<?xml version='1.0' encoding='UTF-16'?><r/>",
    ] {
        assert!(scan(invalid, XmlLimits::default(), |_| Ok(())).is_err());
    }
}

#[test]
fn namespaces_restore_after_nested_shadowing() {
    let mut names = Vec::new();
    scan(
        b"<p:r xmlns:p='u'><a xmlns:p='v'><p:b/></a><p:b/></p:r>",
        XmlLimits::default(),
        |e| {
            if let XmlEvent::Start { element, .. } = e {
                names.push(element.name.clone());
            }
            Ok(())
        },
    )
    .unwrap();
    assert!(names[2].is("v", "b"));
    assert!(names[3].is("u", "b"));
}

#[test]
fn xml_declarations_are_checked_and_cancellation_is_observable() {
    for xml in [
        "<?xml version='1.0' unknown='yes'?><r/>",
        "<?xml version='1.0' version='1.0'?><r/>",
        "<?xml version='1.0' standalone='maybe'?><r/>",
        "<?xml version='1.0' standalone='yes' encoding='UTF-8'?><r/>",
    ] {
        assert!(checked(xml).is_err(), "accepted {xml}");
    }
    let calls = std::cell::Cell::new(0);
    let error = scan_with_control(
        b"<r><a/><b/><c/></r>",
        XmlLimits::default(),
        &|| {
            let n = calls.get();
            calls.set(n + 1);
            n > 4
        },
        |_| Ok(()),
    )
    .unwrap_err();
    assert!(matches!(error, XmlError::Cancelled));
}
