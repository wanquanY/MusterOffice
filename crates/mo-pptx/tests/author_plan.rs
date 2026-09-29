use mo_common::{Emu, ObjectId};
use mo_pptx::{
    AuthorPlan, ExportDefaults, PptxError, PptxLimits, export, export_plan_to,
    source::{SourceLimits, inspect_source},
};
use mo_presentation_model::*;
use serde_json::{Value, json};
mod support;

// Physical XML ordinals and sealed package hashes are deliberately different
// from semantic plan addresses. Compare the entire declaration graph after
// rebinding text edges to structure, without removing any declaration values.
fn canonical(mut value: Value) -> Value {
    match &mut value {
        Value::Object(object) => {
            for key in [
                "sourceOrdinal",
                "sha256",
                "sourceSha256",
                "byteLength",
                "textBodyOrdinal",
                "gridOrdinal",
            ] {
                object.remove(key);
            }
            if let Some(Value::Array(notices)) = object.get_mut("notices") {
                notices.retain(|v| !matches!(v.as_str(), Some("partial native projection; styles, layout, advanced objects and playback unresolved" | "line declarations parsed; style inheritance and other format families unresolved")));
            }
            if let Some(text) = object.get_mut("text")
                && let (Some(nodes), Some(roots)) = (text.get("nodes"), text.get("roots"))
            {
                fn node(nodes: &Value, id: &Value) -> Value {
                    let mut value = nodes[id.to_string()].clone();
                    let children = value["children"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|id| node(nodes, id))
                        .collect::<Vec<_>>();
                    value.as_object_mut().unwrap().remove("parent");
                    value["children"] = Value::Array(children);
                    canonical(value)
                }
                let roots = roots
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|r| json!({"owner":r["owner"],"cell":r["cell"],"node":node(nodes,&r["sourceOrdinal"])}))
                    .collect::<Vec<_>>();
                assert!(text["effectNodes"].as_object().unwrap().is_empty());
                *text = json!(roots);
            }
            for value in object.values_mut() {
                *value = canonical(value.take());
            }
        }
        Value::Array(values) => {
            for value in values {
                *value = canonical(value.take());
            }
        }
        _ => (),
    }
    value
}

#[test]
fn author_projection_and_verified_native_readback_have_identical_declarations() {
    let (document, defaults) = support::input();
    assert_readback(&document, &defaults);
}

fn assert_readback(document: &Document, defaults: &ExportDefaults) {
    let before = document.clone();
    let plan = AuthorPlan::new(document, defaults, Default::default(), &|| false).unwrap();
    let output = export_plan_to(
        &plan,
        &support::resources(),
        Vec::new(),
        PptxLimits::default().package,
        &|| false,
    )
    .unwrap();
    let readback = inspect_source(output.package(), SourceLimits::default(), &|| false).unwrap();
    for index in [plan.declarations(), &readback] {
        for surface in index.surfaces.values() {
            for object in &surface.objects {
                if let Some(root) = object.text_body_ordinal {
                    assert!(
                        surface
                            .text
                            .roots
                            .iter()
                            .any(|r| r.owner == Some(object.native_id) && r.source_ordinal == root)
                    );
                }
            }
        }
    }
    let left = canonical(serde_json::to_value(plan.declarations()).unwrap());
    let right = canonical(serde_json::to_value(&readback).unwrap());
    fn compare(path: String, left: &Value, right: &Value) {
        match (left, right) {
            (Value::Object(a), Value::Object(b)) => {
                assert_eq!(
                    a.keys().collect::<Vec<_>>(),
                    b.keys().collect::<Vec<_>>(),
                    "{path}: fields"
                );
                for (key, left) in a {
                    compare(format!("{path}/{key}"), left, &b[key]);
                }
            }
            (Value::Array(a), Value::Array(b)) => {
                assert_eq!(a.len(), b.len(), "{path}: length");
                for (i, (left, right)) in a.iter().zip(b).enumerate() {
                    compare(format!("{path}/{i}"), left, right);
                }
            }
            _ => assert_eq!(left, right, "{path}"),
        }
    }
    compare(String::new(), &left, &right);
    assert_eq!(document, &before);
}

fn visible_stroke() -> Inherited<Stroke> {
    Inherited::Value(Stroke::Solid {
        width: Emu::new(114300),
        color: Color::Theme {
            slot: ThemeColor::Accent2,
        },
        cap: Some(LineCap::Round),
        join: Some(LineJoin::Bevel {}),
    })
}

fn grouped() -> (Document, ExportDefaults) {
    let (mut d, defaults) = support::input();
    // Wrap existing content on every kind of surface, preserving its own line
    // declarations. The slide's existing group becomes a nested group.
    let prototype = d.objects[&ObjectId::new("group:2").unwrap()].clone();
    let owners = [
        ContainerId::Master(d.masters.keys().next().unwrap().clone()),
        ContainerId::Layout(d.layouts.keys().next().unwrap().clone()),
        ContainerId::Slide(d.slide_order[1].clone()),
    ];
    for (i, owner) in owners.into_iter().enumerate() {
        let id = ObjectId::new(format!("wrapper:{i}")).unwrap();
        let roots = match &owner {
            ContainerId::Master(id) => &mut d.masters.get_mut(id).unwrap().objects,
            ContainerId::Layout(id) => &mut d.layouts.get_mut(id).unwrap().objects,
            ContainerId::Slide(id) => &mut d.slides.get_mut(id).unwrap().objects,
            ContainerId::Group(_) => unreachable!(),
        };
        let children = std::mem::replace(roots, vec![id.clone()]);
        for child in &children {
            d.objects.get_mut(child).unwrap().parent = ContainerId::Group(id.clone());
        }
        let mut group = prototype.clone();
        group.id = id.clone();
        group.parent = owner;
        group.content = ObjectContent::Group {
            children,
            viewport: d.page_size,
        };
        d.objects.insert(id, group);
    }
    for object in d.objects.values_mut() {
        if matches!(object.content, ObjectContent::Group { .. }) {
            object.appearance.stroke = Inherited::Value(Stroke::None {});
        }
    }
    d.objects
        .get_mut(&ObjectId::new("child:2a").unwrap())
        .unwrap()
        .appearance
        .stroke = visible_stroke();
    d.objects
        .get_mut(&ObjectId::new("child:2b").unwrap())
        .unwrap()
        .appearance
        .stroke = Inherited::Inherit;
    (d, defaults)
}

#[test]
fn explicit_group_no_stroke_preserves_nested_content_on_all_surface_kinds() {
    let (d, defaults) = grouped();
    assert_readback(&d, &defaults);
    let plan = AuthorPlan::new(&d, &defaults, Default::default(), &|| false).unwrap();
    let mut groups = 0;
    for object in plan
        .declarations()
        .surfaces
        .values()
        .flat_map(|s| &s.objects)
    {
        if object.kind == mo_pptx::source::SourceObjectKind::Group {
            groups += 1;
            assert!(object.line.is_none());
        }
    }
    assert_eq!(groups, 4);
    let mut implicit = d.clone();
    for object in implicit.objects.values_mut() {
        if matches!(object.content, ObjectContent::Group { .. }) {
            object.appearance.stroke = Inherited::Inherit;
        }
    }
    assert_ne!(
        implicit.semantic_digest().unwrap(),
        d.semantic_digest().unwrap()
    );
    let to_bytes = |document| {
        export(
            document,
            &defaults,
            &support::resources(),
            PptxLimits::default(),
            &|| false,
        )
        .unwrap()
    };
    assert_eq!(to_bytes(&d), to_bytes(&implicit));
}

#[test]
fn visible_group_stroke_still_requires_a_defined_mapping_without_mutating_children() {
    let (mut d, defaults) = grouped();
    d.objects
        .get_mut(&ObjectId::new("group:2").unwrap())
        .unwrap()
        .appearance
        .stroke = visible_stroke();
    let before = d.clone();
    assert!(
        matches!(AuthorPlan::new(&d, &defaults, Default::default(), &|| false),
        Err(PptxError::Unsupported(reason)) if reason == "group stroke semantics")
    );
    assert_eq!(d, before);
}

#[test]
fn native_table_author_plan_and_readback_share_cells_and_one_text_graph() {
    let request: Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/native-tables/request.json"
    ))
    .unwrap();
    let document: Document = serde_json::from_value(request["document"].clone()).unwrap();
    let defaults = serde_json::from_value(request["defaults"].clone()).unwrap();
    assert_readback(&document, &defaults);
}
