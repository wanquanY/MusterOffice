mod support;
use mo_common::*;
use mo_presentation_model::*;
use support::*;

#[test]
fn duplicate_and_orphaned_ownership_are_rejected() {
    let mut doc = document();
    doc.slides.get_mut(&slide_id()).unwrap().objects.push(id());
    assert!(
        validate(&doc, ValidationLimits::default())
            .issues
            .iter()
            .any(|i| i.code == ValidationCode::OwnershipConflict)
    );
    doc.slides.get_mut(&slide_id()).unwrap().objects.clear();
    assert!(!validate(&doc, ValidationLimits::default()).is_valid());
}

#[test]
fn limit_errors_cannot_be_hidden_by_zero_diagnostic_budget() {
    let mut doc = document();
    doc.page_size.width = Emu::ZERO;
    let report = validate(
        &doc,
        ValidationLimits {
            max_issues: 0,
            ..ValidationLimits::default()
        },
    );
    assert!(report.issues.is_empty());
    assert!(report.truncated);
    assert!(!report.is_valid());
}

#[test]
fn mismatched_resource_kind_and_missing_fonts_are_rejected() {
    let mut doc = document();
    let resource = ResourceId::new("resource:1").unwrap();
    doc.resources.insert(
        resource.clone(),
        Resource {
            id: resource.clone(),
            kind: ResourceKind::Audio,
            sha256: digest("fixture", &"fake").unwrap(),
            media_type: "audio/mpeg".into(),
        },
    );
    doc.objects.get_mut(&id()).unwrap().content = ObjectContent::Picture {
        resource,
        crop: Crop::default(),
    };
    assert!(
        validate(&doc, ValidationLimits::default())
            .issues
            .iter()
            .any(|i| i.path.ends_with("/resource"))
    );
    let mut doc = document();
    let ObjectContent::Shape {
        text: Some(body), ..
    } = &mut doc.objects.get_mut(&id()).unwrap().content
    else {
        unreachable!()
    };
    body.style.font = Inherited::Value(FontId::new("absent").unwrap());
    assert!(
        validate(&doc, ValidationLimits::default())
            .issues
            .iter()
            .any(|i| i.code == ValidationCode::MissingReference)
    );
}

#[test]
fn connectors_cannot_target_another_slide() {
    let mut doc = document();
    let connector = connector(&mut doc);
    let second = SlideId::new("slide:2").unwrap();
    doc.slide_order.push(second.clone());
    doc.slides.insert(
        second.clone(),
        Slide {
            id: second.clone(),
            name: String::new(),
            layout: None,
            objects: vec![connector.clone()],
            background: Inherited::Inherit,
            hidden: false,
        },
    );
    doc.objects.get_mut(&connector).unwrap().parent = ContainerId::Slide(second);
    doc.slides
        .get_mut(&slide_id())
        .unwrap()
        .objects
        .retain(|o| o != &connector);
    assert!(
        validate(&doc, ValidationLimits::default())
            .issues
            .iter()
            .any(|i| i.message.contains("same slide"))
    );
}

#[test]
fn unbounded_host_depth_setting_still_rejects_cycles_without_overflow() {
    let mut doc = document();
    let connector = connector(&mut doc);
    doc.slides.get_mut(&slide_id()).unwrap().objects.clear();
    doc.objects.get_mut(&connector).unwrap().parent = ContainerId::Group(id());
    let group = doc.objects.get_mut(&id()).unwrap();
    group.parent = ContainerId::Group(id());
    group.content = ObjectContent::Group {
        children: vec![id(), connector],
        viewport: doc.page_size,
    };
    let report = validate(
        &doc,
        ValidationLimits {
            max_group_depth: usize::MAX,
            ..ValidationLimits::default()
        },
    );
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.code == ValidationCode::ReferenceCycle)
    );
}
