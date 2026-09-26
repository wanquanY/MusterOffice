use mo_xml::{mce::*, *};

fn profile() -> Profile {
    Profile {
        understood_namespaces: [
            "urn:base",
            "urn:known",
            "http://www.w3.org/XML/1998/namespace",
            "",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        extension_elements: [ExpandedName {
            namespace: "urn:base".into(),
            local: "extensions".into(),
        }]
        .into_iter()
        .collect(),
    }
}
fn input(body: &str) -> String {
    format!(
        "<r xmlns='urn:base' xmlns:mc='{NAMESPACE}' xmlns:k='urn:known' xmlns:u='urn:unknown'>{body}</r>"
    )
}
#[derive(Debug, PartialEq, Eq)]
struct Start {
    name: String,
    depth: usize,
    ordinal: usize,
    branches: Vec<usize>,
    extension: bool,
}
fn read(text: &str) -> Result<(Vec<Start>, String, Summary), XmlError> {
    let mut nodes = Vec::new();
    let mut value = String::new();
    let summary = mce::scan(
        text.as_bytes(),
        XmlLimits::default(),
        &profile(),
        &|| false,
        |event| {
            if let mce::Event::Content {
                event,
                source_ordinal,
                alternate_ancestors,
                extension_content,
            } = event
            {
                match event {
                    XmlEvent::Start { element, depth, .. } => nodes.push(Start {
                        name: element.name.local.clone(),
                        depth,
                        ordinal: source_ordinal.unwrap(),
                        branches: alternate_ancestors.to_vec(),
                        extension: extension_content,
                    }),
                    XmlEvent::Text { text, .. } => value.push_str(text),
                    _ => {}
                }
            }
            Ok(())
        },
    )?;
    Ok((nodes, value, summary))
}

#[test]
fn chooses_first_understood_branch_with_original_ordinals_and_logical_depths() {
    let (nodes, text, summary) = read(&input("<mc:AlternateContent><mc:Choice Requires='u'><skip/></mc:Choice><mc:Choice Requires='k'><chosen>selected</chosen></mc:Choice><mc:Choice Requires='k'><later/></mc:Choice><mc:Fallback><fallback/></mc:Fallback></mc:AlternateContent>")).unwrap();
    assert_eq!(
        nodes.iter().map(|n| n.name.as_str()).collect::<Vec<_>>(),
        ["r", "chosen"]
    );
    assert_eq!(
        nodes[1],
        Start {
            name: "chosen".into(),
            depth: 1,
            ordinal: 5,
            branches: vec![1],
            extension: false
        }
    );
    assert_eq!(text, "selected");
    assert_eq!(
        summary.selections[0]
            .branches
            .iter()
            .map(|b| b.selected)
            .collect::<Vec<_>>(),
        [false, true, false, false]
    );
}

#[test]
fn fallback_nested_selection_and_no_matching_branch_preserve_sibling_positions() {
    let (nodes, text, summary) = read(&input("<mc:AlternateContent><mc:Choice Requires='u'/><mc:Fallback><mc:AlternateContent><mc:Choice Requires='k'><nested>yes</nested></mc:Choice></mc:AlternateContent></mc:Fallback></mc:AlternateContent><mc:AlternateContent><mc:Choice Requires='u'><absent/></mc:Choice></mc:AlternateContent><tail/>")).unwrap();
    assert_eq!(
        nodes
            .iter()
            .map(|n| (n.name.as_str(), n.depth))
            .collect::<Vec<_>>(),
        [("r", 0), ("nested", 1), ("tail", 1)]
    );
    assert_eq!(nodes[1].branches.len(), 2);
    assert_eq!(text, "yes");
    assert_eq!(summary.selections.len(), 3);
    assert!(summary.selections[2].branches.iter().all(|b| !b.selected));
}

#[test]
fn ignorable_and_process_content_resolve_namespace_scopes_not_prefix_spellings() {
    let text = format!(
        "<r xmlns='urn:base' xmlns:mc='{NAMESPACE}' xmlns:u='urn:unknown' xmlns:alias='urn:unknown' mc:Ignorable='u' mc:ProcessContent='alias:wrap' u:drop='yes'><u:gone><inside/></u:gone><alias:wrap><kept/></alias:wrap><child xmlns:u='urn:rebound'><u:foreign/></child><u:gone/></r>"
    );
    let (nodes, _, summary) = read(&text).unwrap();
    assert_eq!(
        nodes
            .iter()
            .map(|n| (n.name.as_str(), n.depth))
            .collect::<Vec<_>>(),
        [("r", 0), ("kept", 1), ("child", 1), ("foreign", 2)]
    );
    assert_eq!(summary.ignored_elements, 2);
    assert_eq!(summary.ignored_attributes, 1);
    assert_eq!(summary.unwrapped_elements, 1);
    // ProcessContent wildcards apply to expanded namespace names as well.
    assert_eq!(
        read(&text.replace("mc:ProcessContent='alias:wrap'", "mc:ProcessContent='u:*'"))
            .unwrap()
            .0
            .iter()
            .filter(|n| n.name == "inside")
            .count(),
        1
    );
}

#[test]
fn extension_content_suspends_mce_and_source_observer_sees_inactive_data() {
    let text = input(
        "<extensions mc:Ignorable='unbound'><mc:Fallback><u:raw/></mc:Fallback></extensions><mc:AlternateContent><mc:Choice Requires='u'><hidden/></mc:Choice><mc:Fallback><shown/></mc:Fallback></mc:AlternateContent>",
    );
    let (nodes, _, _) = read(&text).unwrap();
    assert!(
        nodes
            .iter()
            .filter(|n| ["extensions", "Fallback", "raw"].contains(&n.name.as_str()))
            .all(|n| n.extension)
    );
    let mut physical = Vec::new();
    mce::scan(
        text.as_bytes(),
        XmlLimits::default(),
        &profile(),
        &|| false,
        |event| {
            if let mce::Event::SourceElement { element, .. } = event {
                physical.push(element.name.local.clone());
            }
            Ok(())
        },
    )
    .unwrap();
    assert!(physical.contains(&"hidden".into()));
    assert!(!nodes.iter().any(|n| n.name == "hidden"));
}

#[test]
fn must_understand_only_applies_to_content_that_is_processed() {
    assert!(matches!(
        read(&input("<node mc:MustUnderstand='u'/>")),
        Err(XmlError::Compatibility(_))
    ));
    read(&input("<mc:AlternateContent><mc:Choice Requires='u' mc:MustUnderstand='u'><n mc:MustUnderstand='unbound'/></mc:Choice><mc:Fallback><ok mc:MustUnderstand='k'/></mc:Fallback></mc:AlternateContent>")).unwrap();
    read(&input(
        "<u:ignored mc:Ignorable='u' mc:MustUnderstand='u'/>",
    ))
    .unwrap();
    assert!(
        read(&input(
            "<u:wrap mc:Ignorable='u' mc:ProcessContent='u:wrap' mc:MustUnderstand='u'/>"
        ))
        .is_err()
    );
}

#[test]
fn malformed_mce_control_and_rule_declarations_are_not_silently_discarded() {
    for body in [
        "<mc:AlternateContent/>",
        "<mc:AlternateContent><mc:Fallback/></mc:AlternateContent>",
        "<mc:Choice Requires='k'/>",
        "<mc:AlternateContent><mc:Choice Requires=''/></mc:AlternateContent>",
        "<mc:AlternateContent><mc:Choice Requires='missing'/></mc:AlternateContent>",
        "<mc:AlternateContent><mc:Choice Requires='k'/><mc:Fallback/><mc:Choice Requires='k'/></mc:AlternateContent>",
        "<mc:AlternateContent><mc:Choice Requires='k'/><mc:Fallback/><mc:Fallback/></mc:AlternateContent>",
        "<mc:AlternateContent xml:lang='en'><mc:Choice Requires='k'/></mc:AlternateContent>",
        "<mc:AlternateContent arbitrary='x'><mc:Choice Requires='k'/></mc:AlternateContent>",
        "<mc:AlternateContent><mc:Choice Requires='k' k:attribute='x'/></mc:AlternateContent>",
        "<mc:AlternateContent><mc:Choice Requires='k'/><unrecognized/></mc:AlternateContent>",
        "<mc:AlternateContent><mc:Choice Requires='k'/><extensions/></mc:AlternateContent>",
        "<node mc:Ignorable='mc'/>",
        "<node mc:Ignorable='missing'/>",
        "<node mc:ProcessContent='u:*'/>",
        "<node mc:Ignorable='u' mc:ProcessContent='u:invalid:local'/>",
        "<u:wrap mc:Ignorable='u' mc:ProcessContent='u:*' xml:lang='en'/>",
        "<mc:AlternateContent>wrong<mc:Choice Requires='k'/></mc:AlternateContent>",
    ] {
        assert!(read(&input(body)).is_err(), "accepted {body}");
    }
    read(&input("<mc:AlternateContent mc:Ignorable='u'><u:future/><mc:Choice Requires='k'/><u:future/></mc:AlternateContent>")).unwrap();
}

#[test]
fn underlying_xml_limits_and_cancellation_apply_to_ignored_content() {
    let text = input("<u:ignored mc:Ignorable='u'><x><x><x/></x></x></u:ignored>");
    assert!(matches!(
        mce::scan(
            text.as_bytes(),
            XmlLimits {
                max_depth: 3,
                ..Default::default()
            },
            &profile(),
            &|| false,
            |_| Ok(())
        ),
        Err(XmlError::Limit(_))
    ));
    assert!(matches!(
        mce::scan(
            text.as_bytes(),
            XmlLimits::default(),
            &profile(),
            &|| true,
            |_| Ok(())
        ),
        Err(XmlError::Cancelled)
    ));
    assert!(read(&input("<mc:AlternateContent><mc:Choice Requires='u'><bad></mismatch></mc:Choice></mc:AlternateContent>")).is_err());
}
