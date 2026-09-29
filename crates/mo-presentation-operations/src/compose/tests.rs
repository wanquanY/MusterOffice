use super::*;
use crate::*;
use mo_common::RequestId;
use mo_presentation_edit::Snapshot;
use mo_presentation_model::*;
use std::collections::BTreeSet;

fn content() -> PresentationContent {
    serde_json::from_value(example()).unwrap()
}
fn compute(
    content: PresentationContent,
    check: &dyn Fn() -> bool,
) -> Result<MutationCandidate, Failure> {
    compute_mutation(
        &Computation {
            request_id: &RequestId::new("request:compose-test").unwrap(),
            profile_id: OperationProfile::AuthorModel,
            action: &DocumentAction::Compose {
                presentation: Box::new(content),
            },
        },
        None,
        check,
    )
}

#[test]
fn compact_example_becomes_the_same_valid_native_model() {
    let schema = serde_json::to_value(schemars::schema_for!(PresentationContent)).unwrap();
    assert_eq!(schema.pointer("/examples/0"), Some(&example()));
    let c = content();
    let result = compute(c.clone(), &|| false).unwrap();
    let d = &result.snapshot().document;
    assert!(validate(d, ValidationLimits::default()).is_valid());
    let direct = Snapshot::new(
        c.to_document(&|| false).unwrap(),
        ValidationLimits::default(),
    )
    .unwrap();
    assert_eq!(result.snapshot().semantic_digest, *direct.semantic_digest());
    assert!(
        serde_json::to_vec(&example()).unwrap().len() * 3 < serde_json::to_vec(d).unwrap().len()
    );
    let object = d.objects.values().next().unwrap();
    assert_eq!(object.appearance.fill, Inherited::Value(Fill::None));
    let ObjectContent::Shape {
        text: Some(text), ..
    } = &object.content
    else {
        panic!("native text shape")
    };
    assert_eq!(text.style.size, Inherited::Value(Emu::new(457200)));
    assert_eq!(text.style.bold, Inherited::Value(true));
    assert_eq!(text.style.font, Inherited::Inherit);
    assert_eq!(text.overflow, OverflowPolicy::Report);
    assert!(d.resources.is_empty() && d.fonts.is_empty() && d.source_bindings.is_none());
}

#[test]
fn page_and_paint_order_and_identity_are_author_controlled() {
    let mut c = content();
    let mut second = c.slides[0].clone();
    second.id = SlideId::new("slide:2").unwrap();
    second.elements[0].id = ObjectId::new("object:second").unwrap();
    second.elements[0].frame.rotation = -5400000;
    second.elements[0].frame.flip_horizontal = true;
    c.slides.insert(0, second);
    let first = compute(c.clone(), &|| false).unwrap();
    let d = &first.snapshot().document;
    assert_eq!(
        d.slide_order
            .iter()
            .map(SlideId::as_str)
            .collect::<Vec<_>>(),
        ["slide:2", "slide:1"]
    );
    let second_id = ObjectId::new("object:second").unwrap();
    let o = &d.objects[&second_id];
    assert_eq!(
        o.parent,
        ContainerId::Slide(SlideId::new("slide:2").unwrap())
    );
    assert_eq!(o.transform.unwrap().rotation, -5400000);
    assert!(o.transform.unwrap().flip_horizontal);
    let get_ids = |d: &Document| {
        d.objects
            .iter()
            .map(|(id, o)| {
                let ObjectContent::Shape { text: Some(t), .. } = &o.content else {
                    panic!()
                };
                (id.clone(), t.paragraphs[0].id.clone())
            })
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    let original = get_ids(d);
    c.slides.reverse();
    c.slides[0].elements[0].text.as_mut().unwrap().text = "Replacement text".into();
    assert_eq!(
        original,
        get_ids(&compute(c, &|| false).unwrap().snapshot().document)
    );
    assert_ne!(original.values().next(), original.values().nth(1));
}

#[test]
fn newline_tab_empty_paragraphs_and_explicit_false_are_preserved() {
    let mut c = content();
    let text = c.slides[0].elements[0].text.as_mut().unwrap();
    text.text = "A\tB\r\n\rC\n".into();
    text.style.bold = Some(false);
    text.wrap = false;
    let result = compute(c, &|| false).unwrap();
    let ObjectContent::Shape { text: Some(t), .. } = &result
        .snapshot()
        .document
        .objects
        .values()
        .next()
        .unwrap()
        .content
    else {
        panic!()
    };
    assert_eq!(t.paragraphs.len(), 4);
    assert_eq!(t.style.bold, Inherited::Value(false));
    assert!(!t.wrap);
    assert_eq!(t.paragraphs[0].runs.len(), 3);
    assert_eq!(t.paragraphs[0].runs[1].content, InlineContent::Tab);
    let texts: Vec<_> = t
        .paragraphs
        .iter()
        .map(|p| {
            p.runs
                .iter()
                .map(|r| match &r.content {
                    InlineContent::Text { text } => text.as_str(),
                    InlineContent::Tab => "\t",
                    InlineContent::Break => "\n",
                })
                .collect::<String>()
        })
        .collect();
    assert_eq!(texts, ["A\tB", "", "C", ""]);
    let ids: Vec<_> = t
        .paragraphs
        .iter()
        .flat_map(|p| p.runs.iter().map(|r| r.id.as_str()))
        .collect();
    assert_eq!(ids.len(), ids.iter().collect::<BTreeSet<_>>().len());
}

#[test]
fn invalid_and_cancelled_inputs_never_publish_a_candidate() {
    assert_eq!(
        compute(content(), &|| true).err().unwrap().code,
        FailureCode::Cancelled
    );
    let mut c = content();
    c.slides.push(c.slides[0].clone());
    assert_eq!(
        compute(c, &|| false).err().unwrap().code,
        FailureCode::InputInvalid
    );
    let mut c = content();
    let duplicate = c.slides[0].elements[0].clone();
    c.slides[0].elements.push(duplicate);
    assert_eq!(
        compute(c, &|| false).err().unwrap().code,
        FailureCode::InputInvalid
    );
    let mut c = content();
    c.slides.clear();
    assert_eq!(
        compute(c, &|| false).err().unwrap().code,
        FailureCode::InputInvalid
    );
    let mut c = content();
    c.slides[0].elements[0].frame.width = Emu::new(-1);
    assert!(compute(c, &|| false).is_err());
    let mut value = example();
    value["slides"][0]["elements"][0]["html"] = "<b>not accepted</b>".into();
    assert!(serde_json::from_value::<PresentationContent>(value).is_err());
}

#[test]
fn expansion_is_bounded_before_materializing_arbitrary_text_runs() {
    let mut c = content();
    c.slides[0].elements[0].text.as_mut().unwrap().text = "\t".repeat(100000);
    assert_eq!(
        compute(c, &|| false).err().unwrap().code,
        FailureCode::LimitExceeded
    );
    let checks = std::cell::Cell::new(0usize);
    let mut c = content();
    c.slides[0].elements[0].text.as_mut().unwrap().text = "x\n".repeat(5000);
    assert_eq!(
        compute(c, &|| {
            checks.set(checks.get() + 1);
            checks.get() > 100
        })
        .err()
        .unwrap()
        .code,
        FailureCode::Cancelled
    );
}
