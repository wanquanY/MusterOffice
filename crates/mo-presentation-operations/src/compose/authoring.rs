//! Common authoring operations, compiled into the ordinary revisioned contract.
//! Hosts bind identity/revision and authorized resource metadata. No alternate
//! document, resource resolver, storage, or permission system lives here.
use super::{PlainText, PresentationContent, ShapeFrame, SlideContent};
use crate::{DocumentAction, Failure, FailureCode, MAX_OPERATION_BYTES};
use mo_common::{Digest, DocumentId, ObjectId, OperationId, SlideId};
use mo_presentation_edit::{DeletePolicy, Operation, OperationEntry};
use mo_presentation_model::{Accessibility, Point, Resource, Size, Transform};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum AuthoringAction {
    Create {
        title: String,
        page_size: Size,
        slides: Vec<SlideContent>,
    },
    Append {
        slides: Vec<SlideContent>,
    },
    Edit {
        edits: Vec<ContentEdit>,
    },
}

/// Targeted edits preserve object/slide identities and unrelated properties.
/// Replacing text is explicit: use the same PlainText contract as creation.
/// Deletions reject dependencies by default; no implicit cascade or page rebuild.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ContentEdit {
    SetTitle {
        title: String,
    },
    SetSlideName {
        slide: SlideId,
        name: String,
    },
    SetSlideBackground {
        slide: SlideId,
        background: mo_presentation_model::Inherited<mo_presentation_model::Fill>,
    },
    InsertElement {
        slide: SlideId,
        index: u32,
        element: super::ElementContent,
    },
    SetFill {
        object: ObjectId,
        fill: mo_presentation_model::Inherited<mo_presentation_model::Fill>,
    },
    SetStroke {
        object: ObjectId,
        stroke: mo_presentation_model::Inherited<mo_presentation_model::Stroke>,
    },
    SetGeometry {
        object: ObjectId,
        geometry: mo_presentation_model::Geometry,
    },
    ReplacePicture {
        object: ObjectId,
        resource: mo_common::ResourceId,
        #[serde(default)]
        crop: Option<mo_presentation_model::Crop>,
    },
    SetPictureCrop {
        object: ObjectId,
        crop: mo_presentation_model::Crop,
    },
    SetFrame {
        object: ObjectId,
        frame: ShapeFrame,
    },
    ReplaceText {
        object: ObjectId,
        text: PlainText,
    },
    SetAccessibility {
        object: ObjectId,
        accessibility: Accessibility,
    },
    MoveSlide {
        slide: SlideId,
        index: u32,
    },
    DeleteSlide {
        slide: SlideId,
    },
    DeleteObject {
        object: ObjectId,
    },
}

impl AuthoringAction {
    /// Resource references selected by the author. A host resolves metadata and
    /// bytes from its authority, then passes native Resource values to bind().
    pub fn picture_resources(&self) -> Vec<mo_common::ResourceId> {
        let picture = |element: &super::ElementContent| match element {
            super::ElementContent::Picture(p) => Some(p.picture.resource.clone()),
            _ => None,
        };
        let mut references = std::collections::BTreeSet::new();
        match self {
            Self::Create { slides, .. } | Self::Append { slides } => {
                references.extend(
                    slides
                        .iter()
                        .flat_map(|slide| &slide.elements)
                        .filter_map(picture),
                );
            }
            Self::Edit { edits } => {
                for edit in edits {
                    match edit {
                        ContentEdit::ReplacePicture { resource, .. } => {
                            references.insert(resource.clone());
                        }
                        ContentEdit::InsertElement { element, .. } => {
                            references.extend(picture(element));
                        }
                        _ => {}
                    }
                }
            }
        }
        references.into_iter().collect()
    }
    pub fn bind(
        self,
        id: DocumentId,
        base: Option<Digest>,
        resources: Vec<Resource>,
        check: &dyn Fn() -> bool,
    ) -> Result<DocumentAction, Failure> {
        crate::budget::check_size(&self, MAX_OPERATION_BYTES, "authoring bytes", check)?;
        let selected = self.picture_resources();
        Ok(match self {
            Self::Create {
                title,
                page_size,
                slides,
            } => {
                if base.is_some() {
                    return Err(invalid("creation cannot have a base revision"));
                }
                DocumentAction::Compose {
                    presentation: Box::new(PresentationContent {
                        id,
                        title,
                        page_size,
                        slides,
                        resources,
                    }),
                }
            }
            Self::Append { slides } => DocumentAction::Append {
                document_id: id,
                base_revision: base.ok_or_else(|| invalid("append requires a base revision"))?,
                slides,
                resources,
            },
            Self::Edit { edits } => {
                if edits.is_empty() {
                    return Err(invalid("edits must not be empty"));
                }
                let mut operations = Vec::with_capacity(edits.len());
                // Resource metadata must be supplied by the host for exactly
                // the references selected by this action, never by the model.
                let supplied = resources
                    .iter()
                    .map(|r| r.id.clone())
                    .collect::<std::collections::BTreeSet<_>>();
                if supplied.len() != resources.len()
                    || selected.iter().any(|id| !supplied.contains(id))
                    || supplied.len() != selected.len()
                {
                    return Err(invalid(
                        "edits require exactly their selected authorized picture resources",
                    ));
                }
                for resource in resources {
                    operations.push(OperationEntry {
                        operation_id: OperationId::new(format!("authoring:{}", operations.len()))
                            .expect("bounded generated id"),
                        operation: Operation::EnsureResource { resource },
                    });
                }
                for edit in edits {
                    if check() {
                        return Err(Failure::new(FailureCode::Cancelled, "authoring cancelled"));
                    }
                    let operation = match edit {
                        ContentEdit::SetTitle { title } => Operation::SetTitle { title },
                        ContentEdit::SetSlideName { slide, name } => {
                            Operation::SetSlideName { slide, name }
                        }
                        ContentEdit::SetSlideBackground { slide, background } => {
                            Operation::SetSlideBackground { slide, background }
                        }
                        ContentEdit::InsertElement {
                            slide,
                            index,
                            element,
                        } => Operation::InsertObject {
                            object: super::lower::lower_element(
                                &element,
                                &id,
                                mo_presentation_model::ContainerId::Slide(slide),
                                MAX_OPERATION_BYTES,
                                check,
                            )?,
                            index,
                        },
                        ContentEdit::SetFill { object, fill } => {
                            Operation::SetFill { object, fill }
                        }
                        ContentEdit::SetStroke { object, stroke } => {
                            Operation::SetStroke { object, stroke }
                        }
                        ContentEdit::SetGeometry { object, geometry } => {
                            Operation::SetGeometry { object, geometry }
                        }
                        ContentEdit::ReplacePicture {
                            object,
                            resource,
                            crop,
                        } => Operation::SetPicture {
                            object,
                            resource,
                            crop,
                        },
                        ContentEdit::SetPictureCrop { object, crop } => {
                            Operation::SetPictureCrop { object, crop }
                        }
                        ContentEdit::SetFrame { object, frame: f } => Operation::SetTransform {
                            object,
                            transform: Transform {
                                origin: Point { x: f.x, y: f.y },
                                size: Size {
                                    width: f.width,
                                    height: f.height,
                                },
                                rotation: f.rotation,
                                flip_horizontal: f.flip_horizontal,
                                flip_vertical: f.flip_vertical,
                            },
                        },
                        ContentEdit::ReplaceText { object, text } => Operation::SetText {
                            text: super::lower::lower_text(
                                &text,
                                &id,
                                &object,
                                MAX_OPERATION_BYTES,
                                check,
                            )?,
                            object,
                        },
                        ContentEdit::SetAccessibility {
                            object,
                            accessibility,
                        } => Operation::SetAccessibility {
                            object,
                            accessibility,
                        },
                        ContentEdit::MoveSlide { slide, index } => {
                            Operation::MoveSlide { slide, index }
                        }
                        ContentEdit::DeleteSlide { slide } => Operation::DeleteSlide {
                            slide,
                            policy: DeletePolicy::RejectDependencies,
                        },
                        ContentEdit::DeleteObject { object } => Operation::DeleteObject {
                            object,
                            policy: DeletePolicy::RejectDependencies,
                        },
                    };
                    operations.push(OperationEntry {
                        operation_id: OperationId::new(format!("authoring:{}", operations.len()))
                            .expect("bounded generated id"),
                        operation,
                    });
                }
                DocumentAction::Apply {
                    document_id: id,
                    base_revision: base
                        .ok_or_else(|| invalid("editing requires a base revision"))?,
                    operations,
                }
            }
        })
    }
}
fn invalid(message: &'static str) -> Failure {
    Failure::new(FailureCode::InputInvalid, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Computation, OperationProfile, compute_mutation};
    use mo_common::{RequestId, from_json_str_with_path};
    use serde_json::json;
    fn compute(
        action: &DocumentAction,
        base: Option<&mo_presentation_edit::SnapshotRecord>,
    ) -> crate::MutationCandidate {
        compute_mutation(
            &Computation {
                request_id: &RequestId::new("request:test").unwrap(),
                profile_id: OperationProfile::AuthorModel,
                action,
            },
            base.cloned(),
            &|| false,
        )
        .unwrap()
    }
    #[test]
    fn common_edit_preserves_identity_and_unrelated_native_fields_and_rejects_stale_base() {
        let content: super::super::PresentationContent =
            serde_json::from_value(super::super::example()).unwrap();
        let first = compute(
            &DocumentAction::Compose {
                presentation: Box::new(content),
            },
            None,
        );
        let before = first.snapshot();
        let edit: AuthoringAction = serde_json::from_value(json!({"kind":"edit","edits":[{"kind":"setFrame","object":"object:title","frame":{"x":"1","y":"2","width":"3000000","height":"900000"}}]})).unwrap();
        let action = edit
            .bind(
                before.document.id.clone(),
                Some(before.revision.clone()),
                vec![],
                &|| false,
            )
            .unwrap();
        let result = compute(&action, Some(before));
        let after = result.snapshot();
        let id = ObjectId::new("object:title").unwrap();
        assert_eq!(
            before.document.objects[&id].content,
            after.document.objects[&id].content
        );
        assert_eq!(
            before.document.objects[&id].accessibility,
            after.document.objects[&id].accessibility
        );
        assert_eq!(before.document.slide_order, after.document.slide_order);
        assert_ne!(before.revision, after.revision);
        assert!(
            compute_mutation(
                &Computation {
                    request_id: &RequestId::new("request:stale").unwrap(),
                    profile_id: OperationProfile::AuthorModel,
                    action: &action
                },
                Some(after.clone()),
                &|| false
            )
            .is_err()
        );
    }
    #[test]
    fn pictures_and_accessibility_keep_author_semantics_and_errors_name_the_actual_field() {
        let mut value = super::super::example();
        value["slides"][0]["elements"][0]["accessibility"] = json!({"decorative":true});
        let c: super::super::PresentationContent = serde_json::from_value(value.clone()).unwrap();
        assert!(c.slides[0].elements[0].accessibility().decorative);
        value["slides"][0]["elements"][0]["text"]["style"]["alignment"] = json!("right");
        let error =
            from_json_str_with_path::<super::super::PresentationContent>(&value.to_string())
                .unwrap_err();
        assert!(
            error.message.contains("alignment") && error.message.contains("right"),
            "{error:?}"
        );
        assert!(!error.message.contains("untagged enum"));
    }
    #[test]
    fn authoring_never_invents_a_revision_or_authorized_resource() {
        let append = AuthoringAction::Append { slides: vec![] };
        assert!(
            append
                .bind(
                    DocumentId::new("document:x").unwrap(),
                    None,
                    vec![],
                    &|| false
                )
                .is_err()
        );
        let create: AuthoringAction = serde_json::from_value(json!({"kind":"create","title":"T","pageSize":{"width":"100000","height":"100000"},"slides":[{"id":"slide:one","elements":[{"id":"object:photo","frame":{"x":"0","y":"0","width":"1000","height":"1000"},"picture":{"resource":"resource:selected"},"accessibility":{"title":"Photo","description":"A machine","decorative":false}}]}]})).unwrap();
        assert_eq!(create.picture_resources()[0].as_str(), "resource:selected");
        let action = create
            .bind(
                DocumentId::new("document:x").unwrap(),
                None,
                vec![],
                &|| false,
            )
            .unwrap();
        assert!(
            compute_mutation(
                &Computation {
                    request_id: &RequestId::new("request:missing").unwrap(),
                    profile_id: OperationProfile::AuthorModel,
                    action: &action
                },
                None,
                &|| false
            )
            .is_err()
        );
    }
}
