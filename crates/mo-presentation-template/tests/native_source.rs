//! Real native packages prove that scope isolation does not reconstruct or
//! flatten source content, including source extensions outside editable fields.
use mo_common::*;
use mo_opc::{Package, PartName, RewritePlan};
use mo_presentation_edit::Snapshot;
use mo_presentation_model::*;
use mo_presentation_template::*;
use std::collections::BTreeMap;
#[path = "../../mo-pptx/tests/support/mod.rs"]
mod support;

fn package(bytes: &[u8]) -> Package<&[u8]> {
    Package::open(bytes, bytes.len() as u64, Default::default(), &|| false).unwrap()
}
fn native() -> Vec<u8> {
    let (document, defaults) = support::input();
    let bytes = mo_pptx::export(
        &document,
        &defaults,
        &support::resources(),
        Default::default(),
        &|| false,
    )
    .unwrap();
    let source = package(&bytes);
    let slide = PartName::new("/ppt/slides/slide1.xml").unwrap();
    let xml = String::from_utf8(source.read_part(&slide, 1 << 24, &|| false).unwrap()).unwrap();
    let xml=xml.replace("</p:sld>","<p:extLst><p:ext uri=\"owned-template-fixture\"><x:opaque xmlns:x=\"urn:musteroffice:owned\" value=\"retain &amp; isolate\"/></p:ext></p:extLst></p:sld>");
    let mut plan = RewritePlan::new();
    plan.replace_part(slide, xml.into_bytes()).unwrap();
    plan.to_bytes(&source, &|| false).unwrap()
}
fn imported(bytes: &[u8]) -> Snapshot {
    let document = mo_pptx::source::document::import_document(
        &package(bytes),
        DocumentId::new("source").unwrap(),
        ResourceId::new("source:package").unwrap(),
        Default::default(),
        &|| false,
    )
    .unwrap();
    Snapshot::new(document, Default::default()).unwrap()
}
fn definition(snapshot: &Snapshot) -> (TemplateDefinition, ObjectId, RunId) {
    let (object, paragraph, run) = snapshot
        .document()
        .objects
        .iter()
        .find_map(|(id, o)| {
            if let ObjectContent::RetainedSource { paragraphs, .. } = &o.content
                && matches!(&o.parent, ContainerId::Slide(_))
            {
                let p = paragraphs.first()?;
                let r = p.runs.iter().find(|r| r.kind == RetainedRunKind::Text)?;
                return Some((id.clone(), p.id.clone(), r.id.clone()));
            }
            None
        })
        .unwrap();
    let parameters = BTreeMap::from([(
        TemplateParameterId::new("heading").unwrap(),
        Parameter {
            label: "Heading".into(),
            required: true,
            target: ParameterTarget::TextRun {
                object: object.clone(),
                paragraph,
                run: run.clone(),
                min_scalars: 0,
                max_scalars: 500,
            },
        },
    )]);
    (
        TemplateDefinition {
            format: TemplateVersion::V1,
            source: TemplateSource::of(snapshot),
            parameters,
        },
        object,
        run,
    )
}

#[test]
fn imported_template_instances_rewrite_only_bound_text_and_preserve_native_parts() {
    let bytes = native();
    let source = package(&bytes);
    let snapshot = imported(&bytes);
    let (definition, object, run) = definition(&snapshot);
    let binding = snapshot
        .document()
        .source_bindings
        .as_ref()
        .unwrap()
        .objects[&object]
        .clone();
    let source_record = snapshot.clone().into_record();
    let template =
        Template::new(snapshot, definition.clone(), Default::default(), &|| false).unwrap();
    let original_description = template.describe(&|| false).unwrap();
    for (index, value) in ["原生模板 😀 <&>", "Another isolated native instance"]
        .into_iter()
        .enumerate()
    {
        let request = InstantiateRequest {
            request_id: RequestId::new(format!("create:{index}")).unwrap(),
            template_digest: template.digest().clone(),
            document_id: DocumentId::new(format!("instance:{index}")).unwrap(),
            bindings: BTreeMap::from([(
                TemplateParameterId::new("heading").unwrap(),
                BindingValue::Text(value.into()),
            )]),
        };
        let instance = template.instantiate(&request, &|| false).unwrap();
        let plan = mo_pptx::source::document::SourcePlan::new(
            &instance.snapshot.document,
            &source,
            Default::default(),
            &|| false,
        )
        .unwrap();
        let output = plan.write(&source, &|| false).unwrap();
        let actual = package(&output);
        assert_eq!(source.relationships(), actual.relationships());
        let reread = mo_pptx::source::document::import_document(
            &actual,
            DocumentId::new("verify").unwrap(),
            ResourceId::new("source:package").unwrap(),
            Default::default(),
            &|| false,
        )
        .unwrap();
        let reread_id = reread
            .source_bindings
            .as_ref()
            .unwrap()
            .objects
            .iter()
            .find(|(_, b)| b.part == binding.part && b.native_id == binding.native_id)
            .unwrap()
            .0;
        let ObjectContent::RetainedSource { paragraphs, .. } = &reread.objects[reread_id].content
        else {
            panic!()
        };
        assert_eq!(
            paragraphs[binding.runs[&run].paragraph as usize].runs[binding.runs[&run].run as usize]
                .text,
            value
        );
        for part in source.parts().keys() {
            let before = source.read_part(part, 1 << 24, &|| false).unwrap();
            let after = actual.read_part(part, 1 << 24, &|| false).unwrap();
            if part.as_str() == binding.part {
                assert_ne!(before, after);
                assert!(
                    String::from_utf8(after)
                        .unwrap()
                        .contains("value=\"retain &amp; isolate\"")
                );
            } else {
                assert_eq!(before, after, "{part}");
            }
        }
        assert_eq!(template.describe(&|| false).unwrap(), original_description);
        if let Some(root) = std::env::var_os("MO_TEMPLATE_TEST_OUTPUT") {
            let root = std::path::PathBuf::from(root);
            std::fs::create_dir_all(&root).unwrap();
            std::fs::write(root.join(format!("instance-{index}.pptx")), output).unwrap();
            std::fs::write(
                root.join(format!("instance-{index}.json")),
                serde_json::to_vec(&instance).unwrap(),
            )
            .unwrap();
            std::fs::write(root.join("source.pptx"), &bytes).unwrap();
            let calculation = TemplateRequest {
                version: TemplateComputationVersion::V1,
                source: source_record.clone(),
                definition: definition.clone(),
                action: TemplateAction::Instantiate { request },
            };
            std::fs::write(
                root.join(format!("native-{index}.request.json")),
                serde_json::to_vec(&calculation).unwrap(),
            )
            .unwrap();
            std::fs::write(
                root.join(format!("native-{index}.response.json")),
                serde_json::to_vec(&TemplateResponse::Instantiated {
                    instance: Box::new(instance),
                })
                .unwrap(),
            )
            .unwrap();
        }
    }
}

#[test]
fn native_identity_scope_is_pinned_across_nested_instances_and_tampering_is_rejected() {
    let bytes = native();
    let source = imported(&bytes);
    let (specification, _, _) = definition(&source);
    let original_scope = source.document().id.clone();
    assert!(
        source
            .document()
            .source_bindings
            .as_ref()
            .unwrap()
            .identity_scope
            .is_none()
    );
    // Older stored source documents retain their canonical wire representation.
    assert!(
        !serde_json::to_string(source.document())
            .unwrap()
            .contains("identityScope")
    );
    let template = Template::new(source, specification, Default::default(), &|| false).unwrap();
    let instantiate = |template: &Template, id: &str| {
        template
            .instantiate(
                &InstantiateRequest {
                    request_id: RequestId::new(id).unwrap(),
                    template_digest: template.digest().clone(),
                    document_id: DocumentId::new(id).unwrap(),
                    bindings: BTreeMap::from([(
                        TemplateParameterId::new("heading").unwrap(),
                        BindingValue::Text("nested".into()),
                    )]),
                },
                &|| false,
            )
            .unwrap()
    };
    let first = instantiate(&template, "first");
    let source = Snapshot::restore(first.snapshot, Default::default()).unwrap();
    let (definition, _, _) = definition(&source);
    let nested = Template::new(source, definition, Default::default(), &|| false).unwrap();
    let mut second = instantiate(&nested, "second");
    assert_eq!(
        second
            .snapshot
            .document
            .source_bindings
            .as_ref()
            .unwrap()
            .identity_scope,
        Some(original_scope)
    );
    let original = package(&bytes);
    assert!(
        mo_pptx::source::document::SourcePlan::new(
            &second.snapshot.document,
            &original,
            Default::default(),
            &|| false
        )
        .is_ok()
    );
    second
        .snapshot
        .document
        .source_bindings
        .as_mut()
        .unwrap()
        .identity_scope = Some(DocumentId::new("forged").unwrap());
    assert!(
        mo_pptx::source::document::SourcePlan::new(
            &second.snapshot.document,
            &original,
            Default::default(),
            &|| false
        )
        .is_err()
    );
}

#[test]
fn template_admission_obeys_protected_native_run_constraints() {
    let bytes = native();
    let source = imported(&bytes);
    let (mut definition, object, run) = definition(&source);
    let mut record = source.into_record();
    record
        .document
        .source_bindings
        .as_mut()
        .unwrap()
        .objects
        .get_mut(&object)
        .unwrap()
        .runs
        .get_mut(&run)
        .unwrap()
        .constraint = Some(NativeEditConstraint::TimingReferences);
    // A constrained source projection remains a valid document, but cannot be
    // falsely advertised as an editable template parameter.
    let source = Snapshot::new(record.document, Default::default()).unwrap();
    definition.source = TemplateSource::of(&source);
    assert!(matches!(
        Template::new(source, definition, Default::default(), &|| false),
        Err(TemplateError::Edit(_))
    ));
}

#[test]
fn immutable_source_package_is_not_a_resource_replacement_parameter() {
    let bytes = native();
    let source = imported(&bytes);
    let resource = source
        .document()
        .source_bindings
        .as_ref()
        .unwrap()
        .resource
        .clone();
    let media_type = source.document().resources[&resource].media_type.clone();
    let definition = TemplateDefinition {
        format: TemplateVersion::V1,
        source: TemplateSource::of(&source),
        parameters: BTreeMap::from([(
            TemplateParameterId::new("provenance").unwrap(),
            Parameter {
                label: "Not replaceable".into(),
                required: true,
                target: ParameterTarget::Resource {
                    resource,
                    media_types: std::collections::BTreeSet::from([media_type]),
                },
            },
        )]),
    };
    assert!(matches!(
        Template::new(source, definition, Default::default(), &|| false),
        Err(TemplateError::Parameter { .. })
    ));
}
