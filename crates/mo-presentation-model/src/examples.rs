//! Executable schema guidance, not a second authoring grammar. Empty maps and
//! inherited styles remain real model declarations resolved by delivery inputs.
use serde_json::{Value, json};

pub(crate) fn document() -> Value {
    json!({
        "format": "musteroffice.presentation/0.1-draft",
        "id": "document:example",
        "title": "Editable presentation",
        "pageSize": {"width": "12192000", "height": "6858000"},
        "slideOrder": ["slide:1"],
        "slides": {
            "slide:1": {
                "id": "slide:1", "name": "Introduction", "layout": null,
                "objects": ["object:title"], "hidden": false,
                "background": {"kind":"value", "value":{"kind":"solid", "color":{
                    "kind":"srgb", "rgba":{"red":255,"green":255,"blue":255,"alpha":255}
                }}}
            }
        },
        "objects": {
            "object:title": {
                "id":"object:title", "parent":{"kind":"slide", "id":"slide:1"},
                "transform": {
                    "origin":{"x":"914400", "y":"914400"},
                    "size":{"width":"10363200", "height":"914400"},
                    "rotation":0, "flipHorizontal":false, "flipVertical":false
                },
                "appearance": {
                    "fill":{"kind":"value", "value":{"kind":"none"}},
                    "stroke":{"kind":"value", "value":{"kind":"none"}}
                },
                "accessibility":{"title":"Title", "description":"Editable text", "decorative":false},
                "content": {
                    "kind":"shape", "geometry":{"kind":"rectangle"},
                    "text": {
                        "style": {
                            "size":{"kind":"value", "value":"457200"},
                            "color":{"kind":"value", "value":{"kind":"srgb", "rgba":{
                                "red":22,"green":34,"blue":56,"alpha":255
                            }}}
                        },
                        "paragraphs":[{
                            "id":"paragraph:title", "style":{}, "defaultRunStyle":{},
                            "runs":[{"id":"run:title", "style":{}, "content":{"kind":"text", "text":"Your title"}}]
                        }],
                        "insets":{"left":"0", "top":"0", "right":"0", "bottom":"0"},
                        "wrap":true, "overflow":"report"
                    }
                }
            }
        },
        "themes":{}, "masters":{}, "layouts":{}, "fonts":{}, "resources":{}
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Document, ValidationLimits, validate};

    #[test]
    fn discovered_example_is_a_complete_valid_native_document() {
        let schema = serde_json::to_value(schemars::schema_for!(Document)).unwrap();
        let example = schema.pointer("/examples/0").unwrap();
        assert_eq!(*example, document());
        assert!(serde_json::to_vec(example).unwrap().len() < 3000);
        let model: Document = serde_json::from_value(example.clone()).unwrap();
        let report = validate(&model, ValidationLimits::default());
        assert!(report.is_valid(), "{report:?}");
        assert_eq!(model.slide_order.len(), 1);
        assert_eq!(model.objects.len(), 1);
        assert!(model.fonts.is_empty());
        assert!(model.resources.is_empty());
        let canonical = serde_json::to_value(&model).unwrap();
        let reread: Document = serde_json::from_value(canonical).unwrap();
        assert_eq!(
            model.semantic_digest().unwrap(),
            reread.semantic_digest().unwrap()
        );
    }
}
