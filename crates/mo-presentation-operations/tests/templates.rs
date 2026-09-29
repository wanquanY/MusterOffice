use mo_common::*;
use mo_presentation_edit::{Operation, OperationEntry, Snapshot};
use mo_presentation_operations::*;
use mo_presentation_template::*;
use std::{cell::Cell, collections::BTreeMap};
#[path = "../../mo-presentation-edit/tests/support/mod.rs"]
mod support;

fn input() -> Invocation {
    let mut document = support::document();
    support::connector(&mut document);
    let source = Snapshot::new(document, Default::default()).unwrap();
    let definition = TemplateDefinition {
        format: TemplateVersion::V1,
        source: TemplateSource::of(&source),
        parameters: BTreeMap::from([(
            TemplateParameterId::new("title").unwrap(),
            Parameter {
                label: "Title".into(),
                required: true,
                target: ParameterTarget::TextRun {
                    object: support::id(),
                    paragraph: support::paragraph_id(),
                    run: support::run_id(),
                    min_scalars: 0,
                    max_scalars: 1000,
                },
            },
        )]),
    };
    let template = Template::new(
        source.clone(),
        definition.clone(),
        Default::default(),
        &|| false,
    )
    .unwrap();
    Invocation {
        request: OperationRequest {
            contract_version: ContractVersion::V1,
            request_id: RequestId::new("instantiate").unwrap(),
            profile_id: OperationProfile::AuthorModel,
            action: DocumentAction::InstantiateTemplate {
                document_id: DocumentId::new("instance").unwrap(),
                definition: Box::new(definition),
                template_digest: template.digest().clone(),
                bindings: BTreeMap::from([(
                    TemplateParameterId::new("title").unwrap(),
                    BindingValue::Text("中文😀 template".into()),
                )]),
            },
        },
        snapshot: Some(Box::new(source.into_record())),
    }
}
fn compute(input: &Invocation) -> Result<MutationCandidate, Failure> {
    input.validate()?;
    compute_mutation(
        &input.request.computation(),
        input.snapshot.as_deref().cloned(),
        &|| false,
    )
}

fn describe_input() -> Invocation {
    let mut input = input();
    let DocumentAction::InstantiateTemplate { definition, .. } = input.request.action else {
        panic!()
    };
    input.request.action = DocumentAction::DescribeTemplate { definition };
    input
}

#[test]
fn description_validates_real_targets_and_provides_the_digest_used_for_instantiation() {
    let query = describe_input();
    let source = query.snapshot.clone().unwrap();
    let output = compute_inline(query.clone(), &|| false).unwrap();
    assert_eq!(output.request_digest, query.request.digest().unwrap());
    let ComputationResult::DescribedTemplate { description } = output.result else {
        panic!()
    };
    assert_eq!(description.definition.source.revision, source.revision);
    let template = Template::new(
        Snapshot::restore(*source.clone(), Default::default()).unwrap(),
        description.definition.clone(),
        Default::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(*description, template.describe(&|| false).unwrap());
    let mut instance = input();
    let DocumentAction::InstantiateTemplate {
        template_digest,
        bindings,
        ..
    } = &mut instance.request.action
    else {
        panic!()
    };
    *template_digest = description.template_digest;
    *bindings = description.examples;
    let created = compute(&instance).unwrap();
    let mut expected = source.document.clone();
    expected.id = created.document_id().clone();
    assert_eq!(created.snapshot().document, expected);
    assert!(
        compute(&query).is_err(),
        "read-only result cannot be mistaken for a mutation"
    );
    assert_eq!(query.snapshot, Some(source));
}

#[test]
fn description_rejects_missing_stale_and_nonexistent_targets_and_is_cancellable() {
    let query = describe_input();
    let mut missing = query.clone();
    missing.snapshot = None;
    assert_eq!(
        compute_inline(missing, &|| false).unwrap_err().code,
        FailureCode::NotFound
    );
    let mut stale = query.clone();
    stale.snapshot.as_mut().unwrap().revision = Digest::from_sha256([3; 32]);
    assert_eq!(
        compute_inline(stale, &|| false).unwrap_err().code,
        FailureCode::RevisionConflict
    );
    let mut invalid = query.clone();
    let DocumentAction::DescribeTemplate { definition } = &mut invalid.request.action else {
        panic!()
    };
    let ParameterTarget::TextRun { object, .. } =
        &mut definition.parameters.values_mut().next().unwrap().target
    else {
        panic!()
    };
    *object = ObjectId::new("missing-target").unwrap();
    assert_eq!(
        compute_inline(invalid, &|| false).unwrap_err().code,
        FailureCode::InputInvalid
    );
    let calls = Cell::new(0);
    compute_inline(query.clone(), &|| {
        calls.set(calls.get() + 1);
        false
    })
    .unwrap();
    for stop in 1..=calls.get() {
        let n = Cell::new(0);
        let failed = compute_inline(query.clone(), &|| {
            n.set(n.get() + 1);
            n.get() == stop
        });
        assert_eq!(failed.unwrap_err().code, FailureCode::Cancelled, "{stop}");
    }
}

#[test]
fn instance_receipt_distinguishes_source_computation_from_new_document_history() {
    let input = input();
    let original = input.snapshot.clone().unwrap();
    let candidate = compute(&input).unwrap();
    assert_eq!(candidate.request_digest(), &input.request.digest().unwrap());
    assert!(candidate.base_revision().is_none() && candidate.base_semantic_digest().is_none());
    assert!(candidate.receipt().transaction.is_none());
    let template = candidate.receipt().template.as_ref().unwrap();
    assert_eq!(template.source.revision, original.revision);
    assert_eq!(template.scope_map.source_document, original.document.id);
    assert_eq!(
        template.scope_map.instance_document,
        *candidate.document_id()
    );
    assert_eq!(template.revision, candidate.snapshot().revision);
    assert!(template.binding_transaction.is_some());
    let DocumentAction::InstantiateTemplate {
        definition,
        template_digest,
        document_id,
        bindings,
    } = &input.request.action
    else {
        panic!()
    };
    let pure = Template::new(
        Snapshot::restore(*original.clone(), Default::default()).unwrap(),
        *definition.clone(),
        Default::default(),
        &|| false,
    )
    .unwrap()
    .instantiate(
        &InstantiateRequest {
            request_id: input.request.request_id.clone(),
            template_digest: template_digest.clone(),
            document_id: document_id.clone(),
            bindings: bindings.clone(),
        },
        &|| false,
    )
    .unwrap();
    assert_eq!(candidate.snapshot(), &pure.snapshot);
    assert_eq!(template.as_ref(), &pure.receipt);
    let id = RequestId::new("edit-instance").unwrap();
    let action = DocumentAction::Apply {
        document_id: document_id.clone(),
        base_revision: candidate.snapshot().revision.clone(),
        operations: vec![OperationEntry {
            operation_id: OperationId::new("title").unwrap(),
            operation: Operation::SetTitle {
                title: "independent".into(),
            },
        }],
    };
    let edited = compute_mutation(
        &Computation {
            request_id: &id,
            profile_id: OperationProfile::AuthorModel,
            action: &action,
        },
        Some(pure.snapshot),
        &|| false,
    )
    .unwrap();
    assert!(edited.receipt().template.is_none());
    assert!(edited.receipt().transaction.is_some());
    assert_eq!(input.snapshot.as_deref(), Some(original.as_ref()));
    assert_eq!(compute(&input).unwrap().snapshot(), candidate.snapshot());
}

#[test]
fn source_and_definition_pins_are_checked_before_creating_an_instance() {
    let mut missing = input();
    missing.snapshot = None;
    assert_eq!(compute(&missing).err().unwrap().code, FailureCode::NotFound);
    let mut stale = input();
    let DocumentAction::InstantiateTemplate {
        template_digest, ..
    } = &mut stale.request.action
    else {
        panic!()
    };
    *template_digest = Digest::from_sha256([1; 32]);
    let failure = compute(&stale).err().unwrap();
    assert_eq!(failure.code, FailureCode::RevisionConflict);
    assert_eq!(failure.detail.unwrap()["code"], "TEMPLATE_CONFLICT");
    let mut stale = input();
    stale.snapshot.as_mut().unwrap().revision = Digest::from_sha256([2; 32]);
    assert_eq!(
        compute(&stale).err().unwrap().code,
        FailureCode::RevisionConflict
    );
    let mut corrupt = input();
    corrupt.snapshot.as_mut().unwrap().document.title.push('!');
    assert_eq!(
        compute(&corrupt).err().unwrap().code,
        FailureCode::InputInvalid
    );
    let mut same = input();
    let DocumentAction::InstantiateTemplate { document_id, .. } = &mut same.request.action else {
        panic!()
    };
    *document_id = same.snapshot.as_ref().unwrap().document.id.clone();
    assert_eq!(
        compute(&same).err().unwrap().code,
        FailureCode::InputInvalid
    );
}

#[test]
fn template_mutation_is_bounded_and_cancellable_at_every_observed_boundary() {
    let input = input();
    let count = Cell::new(0);
    let expected = compute_mutation(
        &input.request.computation(),
        input.snapshot.as_deref().cloned(),
        &|| {
            count.set(count.get() + 1);
            false
        },
    )
    .unwrap();
    for stop in 1..=count.get() {
        let calls = Cell::new(0);
        let result = compute_mutation(
            &input.request.computation(),
            input.snapshot.as_deref().cloned(),
            &|| {
                calls.set(calls.get() + 1);
                calls.get() == stop
            },
        );
        assert_eq!(result.err().unwrap().code, FailureCode::Cancelled, "{stop}");
    }
    assert_eq!(compute(&input).unwrap().snapshot(), expected.snapshot());
    let mut huge = input;
    let DocumentAction::InstantiateTemplate { bindings, .. } = &mut huge.request.action else {
        panic!()
    };
    bindings.insert(
        TemplateParameterId::new("title").unwrap(),
        BindingValue::Text("x".repeat(MAX_OPERATION_BYTES)),
    );
    assert_eq!(
        compute_mutation(
            &huge.request.computation(),
            huge.snapshot.map(|v| *v),
            &|| false
        )
        .err()
        .unwrap()
        .code,
        FailureCode::LimitExceeded
    );
}
