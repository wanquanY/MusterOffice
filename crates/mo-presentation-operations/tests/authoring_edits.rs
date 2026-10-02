use mo_common::{DocumentId, ObjectId, RequestId, ResourceId, SlideId};
use mo_presentation_edit::SnapshotRecord;
use mo_presentation_model::{ObjectContent, Resource};
use mo_presentation_operations::{compose::authoring::AuthoringAction, *};
use serde_json::{Value, json};

fn resource(id: &str, digest: &str) -> Resource {
    serde_json::from_value(
        json!({"id":id,"kind":"picture","sha256":digest.repeat(32),"mediaType":"image/png"}),
    )
    .unwrap()
}
fn run(
    value: Value,
    base: Option<&SnapshotRecord>,
    resources: Vec<Resource>,
) -> Result<MutationCandidate, Failure> {
    let action: AuthoringAction = serde_json::from_value(value).unwrap();
    let action = action.bind(
        DocumentId::new("document:test").unwrap(),
        base.map(|s| s.revision.clone()),
        resources,
        &|| false,
    )?;
    compute_mutation(
        &Computation {
            request_id: &RequestId::new("request:test").unwrap(),
            profile_id: OperationProfile::AuthorModel,
            action: &action,
        },
        base.cloned(),
        &|| false,
    )
}
fn base() -> MutationCandidate {
    run(json!({"kind":"create","title":"Original","pageSize":{"width":"12192000","height":"6858000"},"slides":[{"id":"slide:1","name":"Opening","elements":[
        {"id":"object:text","frame":{"x":"0","y":"0","width":"4000000","height":"900000"},"text":{"text":"Keep this text"}},
        {"id":"object:picture","frame":{"x":"5000000","y":"0","width":"1000000","height":"1000000"},"picture":{"resource":"image:1","crop":{"left":1000,"top":0,"right":0,"bottom":0}}}
    ]}]}), None, vec![resource("image:1", "ab")]).unwrap()
}
#[test]
fn ordinary_edits_preserve_identity_text_and_picture_crop() {
    let first = base();
    let before = first.snapshot();
    let edits = json!({"kind":"edit","edits":[
        {"kind":"setTitle","title":"Revised"},
        {"kind":"setSlideName","slide":"slide:1","name":"Decision"},
        {"kind":"setSlideBackground","slide":"slide:1","background":{"kind":"inherit"}},
        {"kind":"setFill","object":"object:text","fill":{"kind":"inherit"}},
        {"kind":"setStroke","object":"object:text","stroke":{"kind":"inherit"}},
        {"kind":"setGeometry","object":"object:text","geometry":{"kind":"roundRectangle","radius":"100000"}},
        {"kind":"replacePicture","object":"object:picture","resource":"image:2"},
        {"kind":"insertElement","slide":"slide:1","index":1,"element":{"id":"object:added","frame":{"x":"0","y":"2000000","width":"4000000","height":"900000"},"text":{"text":"Added"}}}
    ]});
    let result = run(edits, Some(before), vec![resource("image:2", "cd")]).unwrap();
    let after = result.snapshot();
    assert_eq!(after.document.title, "Revised");
    let slide = &after.document.slides[&SlideId::new("slide:1").unwrap()];
    assert_eq!(slide.name, "Decision");
    assert_eq!(
        slide
            .objects
            .iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>(),
        ["object:text", "object:added", "object:picture"]
    );
    let picture_id = ObjectId::new("object:picture").unwrap();
    assert_eq!(
        before.document.objects[&picture_id].transform,
        after.document.objects[&picture_id].transform
    );
    let ObjectContent::Picture { resource, crop } = &after.document.objects[&picture_id].content
    else {
        panic!()
    };
    assert_eq!(resource.as_str(), "image:2");
    assert_eq!(crop.left, 1000);
    let text_id = ObjectId::new("object:text").unwrap();
    let ObjectContent::Shape { text: old, .. } = &before.document.objects[&text_id].content else {
        panic!()
    };
    let ObjectContent::Shape { text: new, .. } = &after.document.objects[&text_id].content else {
        panic!()
    };
    assert_eq!(old, new);
    assert_ne!(before.revision, after.revision);
}
#[test]
fn resource_collision_and_wrong_object_reject_the_entire_edit() {
    let first = base();
    let before = first.snapshot();
    let original = serde_json::to_value(before).unwrap();
    let edits = json!({"kind":"edit","edits":[{"kind":"setTitle","title":"Must roll back"},{"kind":"replacePicture","object":"object:picture","resource":"image:1"}]});
    assert!(run(edits.clone(), Some(before), vec![resource("image:1", "cd")]).is_err());
    assert!(run(edits.clone(), Some(before), vec![]).is_err());
    assert!(run(edits, Some(before), vec![resource("image:1", "ab")]).is_ok());
    assert!(run(json!({"kind":"edit","edits":[{"kind":"setTitle","title":"Must roll back"},{"kind":"replacePicture","object":"object:text","resource":"image:2"}]}), Some(before), vec![resource("image:2", "cd")]).is_err());
    assert_eq!(serde_json::to_value(before).unwrap(), original);
    assert_eq!(
        before.document.resources[&ResourceId::new("image:1").unwrap()]
            .sha256
            .as_str(),
        "ab".repeat(32)
    );
}
