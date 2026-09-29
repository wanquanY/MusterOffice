use mo_common::*;
use mo_presentation_edit::{Operation, OperationEntry, Snapshot, Transaction, prepare};
use mo_presentation_model::*;
use mo_presentation_template::*;
use mo_timeline::*;
use std::collections::{BTreeMap, BTreeSet};
#[path = "../../mo-presentation-edit/tests/support/mod.rs"]
mod support;

fn parameter(s: &str) -> TemplateParameterId {
    TemplateParameterId::new(s).unwrap()
}
fn text_target() -> ParameterTarget {
    ParameterTarget::TextRun {
        object: support::id(),
        paragraph: support::paragraph_id(),
        run: support::run_id(),
        min_scalars: 0,
        max_scalars: 1_000_000,
    }
}
fn fixture() -> (Snapshot, TemplateDefinition) {
    let mut document = support::document();
    support::connector(&mut document);
    let theme = ThemeId::new("theme").unwrap();
    let color = Rgba {
        red: 0,
        green: 0,
        blue: 0,
        alpha: 255,
    };
    document.themes.insert(
        theme.clone(),
        Theme {
            id: theme.clone(),
            name: "Reusable".into(),
            colors: BTreeMap::from([(ThemeColor::Accent1, color), (ThemeColor::Accent2, color)]),
            default_text: Default::default(),
        },
    );
    let resource = ResourceId::new("image").unwrap();
    document.resources.insert(
        resource.clone(),
        Resource {
            id: resource.clone(),
            kind: ResourceKind::Picture,
            sha256: Digest::try_from("a".repeat(64)).unwrap(),
            media_type: "image/png".into(),
        },
    );
    document.timelines.insert(
        support::slide_id(),
        Timeline {
            format: TimelineVersion::V01,
            tree: None,
            nodes: vec![TimingNode {
                id: TimingNodeId::new("animation").unwrap(),
                restart: Default::default(),
                start: TimeCondition::Click {
                    target: Some(support::id()),
                    delay: RationalTime::new(0, 1).unwrap(),
                }
                .into(),
                end_conditions: vec![],
                duration: RationalTime::new(1, 1).unwrap(),
                repeat_milli: 1000.into(),
                repeat_duration: None,
                fill: FillMode::Freeze,
                time_transform: None,
                effect: Effect::Rotation {
                    composition: Default::default(),
                    target: support::id(),
                    from: 0,
                    to: 1_200_000,
                },
            }],
        },
    );
    let source = Snapshot::new(document, Default::default()).unwrap();
    let targets = [
        ("text", text_target()),
        (
            "image",
            ParameterTarget::Resource {
                resource,
                media_types: BTreeSet::from(["image/png".into(), "image/jpeg".into()]),
            },
        ),
        (
            "accent1",
            ParameterTarget::ThemeColor {
                theme: theme.clone(),
                slot: ThemeColor::Accent1,
            },
        ),
        (
            "accent2",
            ParameterTarget::ThemeColor {
                theme,
                slot: ThemeColor::Accent2,
            },
        ),
        (
            "geometry",
            ParameterTarget::Transform {
                object: support::id(),
            },
        ),
    ];
    let definition = TemplateDefinition {
        format: TemplateVersion::V1,
        source: TemplateSource::of(&source),
        parameters: targets
            .into_iter()
            .map(|(id, target)| {
                (
                    parameter(id),
                    Parameter {
                        label: id.into(),
                        required: id == "text",
                        target,
                    },
                )
            })
            .collect(),
    };
    (source, definition)
}
fn prepared() -> Template {
    let (source, definition) = fixture();
    Template::new(source, definition, Default::default(), &|| false).unwrap()
}
fn request(template: &Template) -> InstantiateRequest {
    InstantiateRequest {
        request_id: RequestId::new("instantiate:1").unwrap(),
        template_digest: template.digest().clone(),
        document_id: DocumentId::new("instance:1").unwrap(),
        bindings: BTreeMap::from([(
            parameter("text"),
            BindingValue::Text("中文😀e\u{301}".into()),
        )]),
    }
}
fn run_text(document: &Document) -> &str {
    let ObjectContent::Shape {
        text: Some(body), ..
    } = &document.objects[&support::id()].content
    else {
        panic!()
    };
    let InlineContent::Text { text } = &body.paragraphs[0].runs[0].content else {
        panic!()
    };
    text
}

#[test]
fn instances_are_isolated_and_keep_internal_graphs_styles_and_resource_identity() {
    let (source, definition) = fixture();
    let original = source.document().clone();
    let template = Template::new(source, definition, Default::default(), &|| false).unwrap();
    let description = template.describe(&|| false).unwrap();
    let mut input = request(&template);
    let red = Rgba {
        red: 255,
        green: 0,
        blue: 0,
        alpha: 128,
    };
    let blue = Rgba {
        red: 0,
        green: 0,
        blue: 255,
        alpha: 255,
    };
    input
        .bindings
        .insert(parameter("accent1"), BindingValue::Color(red));
    input
        .bindings
        .insert(parameter("accent2"), BindingValue::Color(blue));
    let resource = ResourceId::new("image").unwrap();
    let mut replacement = original.resources[&resource].clone();
    replacement.sha256 = Digest::try_from("b".repeat(64)).unwrap();
    input.bindings.insert(
        parameter("image"),
        BindingValue::Resource(replacement.clone()),
    );
    let result = template.instantiate(&input, &|| false).unwrap();
    assert_eq!(result, template.instantiate(&input, &|| false).unwrap());
    assert_eq!(run_text(&result.snapshot.document), "中文😀e\u{301}");
    assert_eq!(result.snapshot.document.resources[&resource], replacement);
    let colors = &result.snapshot.document.themes[&ThemeId::new("theme").unwrap()].colors;
    assert_eq!(colors[&ThemeColor::Accent1], red);
    assert_eq!(colors[&ThemeColor::Accent2], blue);
    assert_eq!(result.snapshot.document.timelines, original.timelines);
    assert_eq!(result.snapshot.document.slides, original.slides);
    assert_eq!(
        result.snapshot.document.objects[&ObjectId::new("connector:1").unwrap()],
        original.objects[&ObjectId::new("connector:1").unwrap()]
    );
    assert_eq!(
        result.receipt.scope_map.local_id_policy,
        LocalIdPolicy::Preserve
    );
    assert_eq!(result.receipt.scope_map.source_document, original.id);
    assert_eq!(
        result.receipt.scope_map.instance_document,
        input.document_id
    );
    let mut expected = original.clone();
    expected.id = input.document_id.clone();
    if let ObjectContent::Shape {
        text: Some(body), ..
    } = &mut expected.objects.get_mut(&support::id()).unwrap().content
    {
        body.paragraphs[0].runs[0].content = InlineContent::Text {
            text: "中文😀e\u{301}".into(),
        };
    }
    expected.resources.insert(resource, replacement);
    expected
        .themes
        .get_mut(&ThemeId::new("theme").unwrap())
        .unwrap()
        .colors = BTreeMap::from([(ThemeColor::Accent1, red), (ThemeColor::Accent2, blue)]);
    assert_eq!(result.snapshot.document, expected);
    input.document_id = DocumentId::new("instance:2").unwrap();
    let second = template.instantiate(&input, &|| false).unwrap();
    assert_ne!(result.snapshot.revision, second.snapshot.revision);
    assert_eq!(template.describe(&|| false).unwrap(), description);
    let instance = Snapshot::restore(result.snapshot, Default::default()).unwrap();
    let changed = prepare(
        &instance,
        &Transaction {
            document_id: instance.document().id.clone(),
            base_revision: instance.revision().clone(),
            request_id: RequestId::new("later").unwrap(),
            operations: vec![OperationEntry {
                operation_id: OperationId::new("rename").unwrap(),
                operation: Operation::SetTitle {
                    title: "Changed independently".into(),
                },
            }],
        },
        Default::default(),
    )
    .unwrap();
    assert_ne!(
        changed.snapshot.document().title,
        second.snapshot.document.title
    );
    assert_eq!(template.describe(&|| false).unwrap(), description);
}

#[test]
fn exact_source_pins_and_changed_definitions_cannot_be_reused() {
    for field in 0..3 {
        let (source, mut definition) = fixture();
        match field {
            0 => definition.source.document_id = DocumentId::new("other").unwrap(),
            1 => definition.source.revision = Digest::try_from("c".repeat(64)).unwrap(),
            _ => definition.source.semantic_digest = Digest::try_from("d".repeat(64)).unwrap(),
        }
        assert!(matches!(
            Template::new(source, definition, Default::default(), &|| false),
            Err(TemplateError::SourceConflict)
        ));
    }
    let template = prepared();
    let mut input = request(&template);
    input.template_digest = Digest::try_from("e".repeat(64)).unwrap();
    assert!(matches!(
        template.instantiate(&input, &|| false),
        Err(TemplateError::TemplateConflict { .. })
    ));
    input = request(&template);
    input.document_id = template.definition().source.document_id.clone();
    assert!(matches!(
        template.instantiate(&input, &|| false),
        Err(TemplateError::InvalidTemplate(_))
    ));
    let (source, mut definition) = fixture();
    definition
        .parameters
        .get_mut(&parameter("text"))
        .unwrap()
        .label = "Another declared template".into();
    assert_ne!(
        template.digest(),
        Template::new(source, definition, Default::default(), &|| false)
            .unwrap()
            .digest()
    );
}

#[test]
fn required_unknown_wrong_type_and_duplicate_targets_fail_atomically() {
    let template = prepared();
    let before = template.describe(&|| false).unwrap();
    let mut missing = request(&template);
    missing.bindings.clear();
    let mut unknown = request(&template);
    unknown
        .bindings
        .insert(parameter("undeclared"), BindingValue::Text("bad".into()));
    let mut wrong = request(&template);
    wrong.bindings.insert(
        parameter("text"),
        BindingValue::Color(Rgba {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 0,
        }),
    );
    for input in [missing, unknown, wrong] {
        assert!(matches!(
            template.instantiate(&input, &|| false),
            Err(TemplateError::Parameter { .. })
        ));
        assert_eq!(template.describe(&|| false).unwrap(), before);
    }
    let (source, mut definition) = fixture();
    definition.parameters.insert(
        parameter("alias"),
        definition.parameters[&parameter("text")].clone(),
    );
    assert!(matches!(
        Template::new(source, definition, Default::default(), &|| false),
        Err(TemplateError::Parameter { .. })
    ));
}

#[test]
fn constraints_count_scalars_and_preserve_empty_text_as_an_explicit_value() {
    let (source, mut definition) = fixture();
    if let ParameterTarget::TextRun { max_scalars, .. } = &mut definition
        .parameters
        .get_mut(&parameter("text"))
        .unwrap()
        .target
    {
        *max_scalars = 5;
    }
    let template = Template::new(source, definition, Default::default(), &|| false).unwrap();
    let mut input = request(&template);
    // Five scalars; UTF-8 bytes and UTF-16 code units are longer.
    assert_eq!(
        run_text(
            &template
                .instantiate(&input, &|| false)
                .unwrap()
                .snapshot
                .document
        ),
        "中文😀e\u{301}"
    );
    input
        .bindings
        .insert(parameter("text"), BindingValue::Text("123456".into()));
    assert!(matches!(
        template.instantiate(&input, &|| false),
        Err(TemplateError::Parameter { .. })
    ));
    input
        .bindings
        .insert(parameter("text"), BindingValue::Text(String::new()));
    assert_eq!(
        run_text(
            &template
                .instantiate(&input, &|| false)
                .unwrap()
                .snapshot
                .document
        ),
        ""
    );
}

#[test]
fn resource_bindings_cannot_retarget_identity_kind_or_unlisted_media() {
    let template = prepared();
    let BindingValue::Resource(original) =
        template.describe(&|| false).unwrap().examples[&parameter("image")].clone()
    else {
        panic!()
    };
    for field in 0..3 {
        let mut resource = original.clone();
        match field {
            0 => resource.id = ResourceId::new("unbound").unwrap(),
            1 => resource.kind = ResourceKind::Font,
            _ => resource.media_type = "image/svg+xml".into(),
        }
        let mut input = request(&template);
        input
            .bindings
            .insert(parameter("image"), BindingValue::Resource(resource));
        assert!(matches!(
            template.instantiate(&input, &|| false),
            Err(TemplateError::Parameter { .. })
        ));
    }
}

#[test]
fn validation_and_editor_constraints_are_not_reimplemented_or_relaxed() {
    let template = prepared();
    let mut input = request(&template);
    let BindingValue::Transform(mut transform) =
        template.describe(&|| false).unwrap().examples[&parameter("geometry")]
    else {
        panic!()
    };
    transform.size.width = Emu::new(-1);
    input
        .bindings
        .insert(parameter("geometry"), BindingValue::Transform(transform));
    assert!(matches!(
        template.instantiate(&input, &|| false),
        Err(TemplateError::Edit(_))
    ));
    let (source, mut definition) = fixture();
    if let ParameterTarget::TextRun {
        min_scalars,
        max_scalars,
        ..
    } = &mut definition
        .parameters
        .get_mut(&parameter("text"))
        .unwrap()
        .target
    {
        *min_scalars = 10;
        *max_scalars = 1;
    }
    assert!(matches!(
        Template::new(source, definition, Default::default(), &|| false),
        Err(TemplateError::Parameter { .. })
    ));
}

#[test]
fn cancellation_at_each_observed_boundary_returns_no_instance_and_keeps_the_template() {
    use std::cell::Cell;
    let template = prepared();
    let input = request(&template);
    let before = template.describe(&|| false).unwrap();
    let calls = Cell::new(0);
    template
        .instantiate(&input, &|| {
            calls.set(calls.get() + 1);
            false
        })
        .unwrap();
    for stop in 1..=calls.get() {
        let current = Cell::new(0);
        let error = template
            .instantiate(&input, &|| {
                current.set(current.get() + 1);
                current.get() >= stop
            })
            .unwrap_err();
        assert!(matches!(
            error,
            TemplateError::Cancelled
                | TemplateError::Edit(mo_presentation_edit::EditError::Cancelled)
        ));
        assert_eq!(template.describe(&|| false).unwrap(), before);
    }
}

#[test]
fn admission_and_output_budgets_prevent_unbounded_templates() {
    let (source, definition) = fixture();
    for limits in [
        TemplateLimits {
            max_parameters: 0,
            ..Default::default()
        },
        TemplateLimits {
            max_bytes: 1,
            ..Default::default()
        },
        TemplateLimits {
            max_label_bytes: 0,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            Template::new(source.clone(), definition.clone(), limits, &|| false),
            Err(TemplateError::LimitExceeded(_))
        ));
    }
    let budget =
        mo_common::check_json_size(&(source.document(), &definition), usize::MAX, &|| false)
            .unwrap()
            + 2048;
    let template = Template::new(
        source,
        definition,
        TemplateLimits {
            max_bytes: budget,
            ..Default::default()
        },
        &|| false,
    )
    .unwrap();
    let mut input = request(&template);
    input.bindings.insert(
        parameter("text"),
        BindingValue::Text("x".repeat(budget - 600)),
    );
    assert!(mo_common::check_json_size(&input, budget, &|| false).is_ok());
    assert!(matches!(
        template.instantiate(&input, &|| false),
        Err(TemplateError::LimitExceeded(_))
    ));
}

#[test]
fn optional_bindings_preserve_the_original_value_without_writing_a_transaction() {
    let (source, mut definition) = fixture();
    let original = source.document().clone();
    definition
        .parameters
        .values_mut()
        .for_each(|p| p.required = false);
    let template = Template::new(source, definition, Default::default(), &|| false).unwrap();
    let mut input = request(&template);
    input.bindings.clear();
    let output = template.instantiate(&input, &|| false).unwrap();
    let mut expected = original;
    expected.id = input.document_id;
    assert_eq!(output.snapshot.document, expected);
    assert!(output.receipt.binding_transaction.is_none());
    assert!(output.receipt.bound_parameters.is_empty());
}
