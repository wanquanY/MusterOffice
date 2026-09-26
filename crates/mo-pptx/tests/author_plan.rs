use mo_pptx::{
    AuthorPlan, PptxLimits, export_plan_to,
    source::{SourceLimits, inspect_source},
};
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
                    .map(|r| json!({"owner":r["owner"],"node":node(nodes,&r["sourceOrdinal"])}))
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
    let plan = AuthorPlan::new(&document, &defaults, Default::default(), &|| false).unwrap();
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
}
