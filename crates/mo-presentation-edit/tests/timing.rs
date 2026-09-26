#[allow(dead_code)]
mod support;
use mo_common::*;
use mo_presentation_edit::*;
use mo_presentation_model::*;
use mo_timeline::*;
use support::{document, id, slide_id};
fn timeline() -> Timeline {
    Timeline {
        format: TimelineVersion::V01,
        tree: None,
        nodes: vec![TimingNode {
            id: TimingNodeId::new("anim").unwrap(),
            start: StartCondition::At {
                offset: RationalTime::new(0, 1).unwrap(),
            },
            duration: RationalTime::new(2, 1).unwrap(),
            end_conditions: vec![],
            repeat_milli: 1000.into(),
            repeat_duration: None,
            time_transform: None,
            fill: FillMode::Freeze,
            effect: Effect::Rotation {
                target: id(),
                from: 0,
                to: 21600000,
            },
        }],
    }
}
fn edit(s: &Snapshot, operations: Vec<Operation>) -> Result<PreparedTransaction, EditError> {
    prepare(
        s,
        &Transaction {
            document_id: s.document().id.clone(),
            request_id: RequestId::new("edit").unwrap(),
            base_revision: s.revision().clone(),
            operations: operations
                .into_iter()
                .enumerate()
                .map(|(i, operation)| OperationEntry {
                    operation_id: OperationId::new(format!("op{i}")).unwrap(),
                    operation,
                })
                .collect(),
        },
        ValidationLimits::default(),
    )
}
#[test]
fn empty_documents_keep_their_existing_wire_form_and_timeline_edits_are_atomic() {
    let doc = document();
    let value = serde_json::to_value(&doc).unwrap();
    assert!(value.get("timelines").is_none());
    let mut empty = value.clone();
    empty["timelines"] = serde_json::json!({});
    let restored: Document = serde_json::from_value(empty).unwrap();
    assert_eq!(
        doc.semantic_digest().unwrap(),
        restored.semantic_digest().unwrap()
    );
    let s = Snapshot::new(doc, ValidationLimits::default()).unwrap();
    let result = edit(
        &s,
        vec![Operation::SetTimeline {
            slide: slide_id(),
            timeline: Some(timeline()),
        }],
    )
    .unwrap();
    assert_eq!(result.receipt.changes.changed_timelines, vec![slide_id()]);
    assert_ne!(result.snapshot.revision(), s.revision());
    assert!(s.document().timelines.is_empty());
    let mut invalid = timeline();
    invalid.nodes[0].effect = Effect::Rotation {
        target: ObjectId::new("missing").unwrap(),
        from: 0,
        to: 1,
    };
    assert!(
        edit(
            &s,
            vec![
                Operation::SetTitle {
                    title: "unpublished".into()
                },
                Operation::SetTimeline {
                    slide: slide_id(),
                    timeline: Some(invalid)
                }
            ]
        )
        .is_err()
    );
    assert_eq!(s.document().title, "");
    let removed = edit(
        &result.snapshot,
        vec![Operation::SetTimeline {
            slide: slide_id(),
            timeline: None,
        }],
    )
    .unwrap();
    assert_eq!(removed.snapshot.document(), s.document());
}
#[test]
fn deleting_a_target_rejects_or_removes_transitive_behaviors() {
    let mut doc = document();
    let second = ObjectId::new("second").unwrap();
    let mut obj = doc.objects[&id()].clone();
    obj.id = second.clone();
    obj.content = ObjectContent::Shape {
        geometry: Geometry::Rectangle,
        text: None,
    };
    doc.objects.insert(second.clone(), obj);
    doc.slides
        .get_mut(&slide_id())
        .unwrap()
        .objects
        .push(second.clone());
    let mut timing = timeline();
    for i in 1..4 {
        let mut n = timing.nodes[0].clone();
        n.id = TimingNodeId::new(format!("n{i}")).unwrap();
        n.effect = Effect::Rotation {
            target: second.clone(),
            from: 0,
            to: 100,
        };
        n.start = StartCondition::After {
            node: timing.nodes.last().unwrap().id.clone(),
            event: NodeEvent::End,
            delay: RationalTime::new(0, 1).unwrap(),
        };
        timing.nodes.push(n);
    }
    doc.timelines.insert(slide_id(), timing);
    let s = Snapshot::new(doc, ValidationLimits::default()).unwrap();
    assert!(
        edit(
            &s,
            vec![Operation::DeleteObject {
                object: id(),
                policy: DeletePolicy::RejectDependencies
            }]
        )
        .is_err()
    );
    let out = edit(
        &s,
        vec![Operation::DeleteObject {
            object: id(),
            policy: DeletePolicy::Cascade,
        }],
    )
    .unwrap();
    assert!(out.snapshot.document().timelines.is_empty());
    assert!(out.snapshot.document().objects.contains_key(&second));
    assert_eq!(out.receipt.changes.changed_timelines, vec![slide_id()]);
    assert_eq!(s.document().timelines[&slide_id()].nodes.len(), 4);
}
#[test]
fn click_trigger_ownership_and_slide_deletion_are_validated() {
    let mut doc = document();
    let other = SlideId::new("other").unwrap();
    let mut slide = doc.slides[&slide_id()].clone();
    slide.id = other.clone();
    slide.objects.clear();
    doc.slides.insert(other.clone(), slide);
    doc.slide_order.push(other.clone());
    doc.timelines.insert(other.clone(), timeline());
    assert!(!validate(&doc, ValidationLimits::default()).is_valid());
    doc.timelines.clear();
    let mut timing = timeline();
    timing.nodes[0].start = StartCondition::Click {
        target: Some(ObjectId::new("missing").unwrap()),
        delay: RationalTime::new(0, 1).unwrap(),
    };
    doc.timelines.insert(slide_id(), timing);
    assert!(!validate(&doc, ValidationLimits::default()).is_valid());
    doc.timelines.insert(slide_id(), timeline());
    let s = Snapshot::new(doc, ValidationLimits::default()).unwrap();
    let out = edit(
        &s,
        vec![Operation::DeleteSlide {
            slide: slide_id(),
            policy: DeletePolicy::Cascade,
        }],
    )
    .unwrap();
    assert!(out.snapshot.document().timelines.is_empty());
}

fn hierarchy(doc: &mut Document) -> (ObjectId, TimingNodeId) {
    let surviving = ObjectId::new("surviving").unwrap();
    let mut object = doc.objects[&id()].clone();
    object.id = surviving.clone();
    object.content = ObjectContent::Shape {
        geometry: Geometry::Rectangle,
        text: None,
    };
    doc.objects.insert(surviving.clone(), object);
    doc.slides
        .get_mut(&slide_id())
        .unwrap()
        .objects
        .push(surviving.clone());
    let mut t = timeline();
    t.format = TimelineVersion::V02;
    let container = TimingNodeId::new("scope").unwrap();
    t.tree = Some(TimingTree {
        roots: vec![container.clone()],
        containers: vec![TimingContainer {
            id: container.clone(),
            kind: ContainerKind::Sequence,
            end_conditions: vec![],
            start: StartCondition::At {
                offset: RationalTime::new(0, 1).unwrap(),
            },
            duration: ContainerDuration::Fixed {
                duration: RationalTime::new(9, 1).unwrap(),
            },
            fill: FillMode::Hold,
            children: vec![t.nodes[0].id.clone()],
        }],
    });
    doc.timelines.insert(slide_id(), t);
    (surviving, container)
}
#[test]
fn deleting_container_trigger_removes_descendants_and_cross_scope_dependents_atomically() {
    let mut doc = document();
    let (surviving, container) = hierarchy(&mut doc);
    let t = doc.timelines.get_mut(&slide_id()).unwrap();
    t.nodes[0].effect = Effect::Rotation {
        target: surviving,
        from: 0,
        to: 120,
    };
    let mut dependent = t.nodes[0].clone();
    dependent.id = TimingNodeId::new("dependent").unwrap();
    dependent.start = StartCondition::After {
        node: dependent.id.clone(),
        event: NodeEvent::End,
        delay: RationalTime::new(0, 1).unwrap(),
    };
    // The dependency crosses out of the removed subtree.
    if let StartCondition::After { node, .. } = &mut dependent.start {
        *node = t.nodes[0].id.clone();
    }
    let tree = t.tree.as_mut().unwrap();
    tree.roots.push(dependent.id.clone());
    tree.containers[0].start = StartCondition::Click {
        target: Some(id()),
        delay: RationalTime::new(0, 1).unwrap(),
    };
    t.nodes.push(dependent);
    let snapshot = Snapshot::new(doc, ValidationLimits::default()).unwrap();
    assert!(
        edit(
            &snapshot,
            vec![Operation::DeleteObject {
                object: id(),
                policy: DeletePolicy::RejectDependencies
            }]
        )
        .is_err()
    );
    let result = edit(
        &snapshot,
        vec![Operation::DeleteObject {
            object: id(),
            policy: DeletePolicy::Cascade,
        }],
    )
    .unwrap();
    assert!(result.snapshot.document().timelines.is_empty());
    assert_eq!(
        snapshot.document().timelines[&slide_id()]
            .tree
            .as_ref()
            .unwrap()
            .roots[0],
        container
    );
    assert_eq!(snapshot.document().timelines[&slide_id()].node_count(), 3);
}
#[test]
fn removing_a_leaf_preserves_an_empty_timer_and_its_event_dependents() {
    let mut doc = document();
    let (surviving, container) = hierarchy(&mut doc);
    let t = doc.timelines.get_mut(&slide_id()).unwrap();
    let mut dependent = t.nodes[0].clone();
    dependent.id = TimingNodeId::new("dependent").unwrap();
    dependent.effect = Effect::Rotation {
        target: surviving,
        from: 0,
        to: 120,
    };
    dependent.start = StartCondition::After {
        node: container,
        event: NodeEvent::End,
        delay: RationalTime::new(0, 1).unwrap(),
    };
    t.tree.as_mut().unwrap().roots.push(dependent.id.clone());
    t.nodes.push(dependent);
    let snapshot = Snapshot::new(doc, ValidationLimits::default()).unwrap();
    let result = edit(
        &snapshot,
        vec![Operation::DeleteObject {
            object: id(),
            policy: DeletePolicy::Cascade,
        }],
    )
    .unwrap();
    let t = &result.snapshot.document().timelines[&slide_id()];
    assert_eq!(t.nodes.len(), 1);
    let tree = t.tree.as_ref().unwrap();
    assert_eq!(tree.containers.len(), 1);
    assert!(tree.containers[0].children.is_empty());
    let binding = PlaybackBinding {
        session: PlaybackSessionId::new("edit").unwrap(),
        revision: result.snapshot.revision().clone(),
        generation: PlaybackGeneration::new(1),
    };
    let frame = TimelinePlan::compile(t, TimelineLimits::default(), &|| false)
        .unwrap()
        .evaluate(&binding, RationalTime::new(10, 1).unwrap(), None, &|| false)
        .unwrap();
    assert_eq!(
        frame.state.nodes[0].start,
        Some(ExactValue {
            numerator: "9".into(),
            denominator: "1".into()
        })
    );
}
#[test]
fn end_trigger_ownership_and_transitive_end_dependencies_are_atomic() {
    let mut doc = document();
    let (surviving, _) = hierarchy(&mut doc);
    let timing = doc.timelines.get_mut(&slide_id()).unwrap();
    timing.nodes[0].effect = Effect::Rotation {
        target: surviving.clone(),
        from: 0,
        to: 120,
    };
    timing.nodes[0].end_conditions = vec![TimeCondition::Click {
        target: Some(id()),
        delay: RationalTime::new(0, 1).unwrap(),
    }];
    let mut dependent = timing.nodes[0].clone();
    dependent.id = TimingNodeId::new("end-dependent").unwrap();
    dependent.end_conditions = vec![TimeCondition::After {
        node: timing.nodes[0].id.clone(),
        event: NodeEvent::End,
        delay: RationalTime::new(0, 1).unwrap(),
    }];
    timing
        .tree
        .as_mut()
        .unwrap()
        .roots
        .push(dependent.id.clone());
    timing.nodes.push(dependent);
    let snapshot = Snapshot::new(doc, ValidationLimits::default()).unwrap();
    assert!(
        edit(
            &snapshot,
            vec![Operation::DeleteObject {
                object: id(),
                policy: DeletePolicy::RejectDependencies
            }]
        )
        .is_err()
    );
    let result = edit(
        &snapshot,
        vec![Operation::DeleteObject {
            object: id(),
            policy: DeletePolicy::Cascade,
        }],
    )
    .unwrap();
    let timing = &result.snapshot.document().timelines[&slide_id()];
    assert!(timing.nodes.is_empty());
    assert_eq!(timing.node_count(), 1);
    assert!(result.snapshot.document().objects.contains_key(&surviving));
    assert_eq!(snapshot.document().timelines[&slide_id()].nodes.len(), 2);
    assert_eq!(result.receipt.changes.changed_timelines, vec![slide_id()]);
    let mut invalid = snapshot.document().clone();
    invalid.timelines.get_mut(&slide_id()).unwrap().nodes[0].end_conditions =
        vec![TimeCondition::Click {
            target: Some(ObjectId::new("missing-stop-target").unwrap()),
            delay: RationalTime::new(0, 1).unwrap(),
        }];
    assert!(!validate(&invalid, ValidationLimits::default()).is_valid());
}
#[test]
fn removing_container_end_target_atomically_removes_its_scope_and_external_dependents() {
    let mut doc = document();
    let (surviving, scope) = hierarchy(&mut doc);
    let timing = doc.timelines.get_mut(&slide_id()).unwrap();
    timing.nodes[0].effect = Effect::Rotation {
        target: surviving.clone(),
        from: 0,
        to: 120,
    };
    timing.tree.as_mut().unwrap().containers[0].end_conditions = vec![TimeCondition::Click {
        target: Some(id()),
        delay: RationalTime::new(0, 1).unwrap(),
    }];
    let mut outside = timing.nodes[0].clone();
    outside.id = TimingNodeId::new("outside-scope").unwrap();
    outside.start = TimeCondition::After {
        node: scope,
        event: NodeEvent::End,
        delay: RationalTime::new(0, 1).unwrap(),
    };
    timing.tree.as_mut().unwrap().roots.push(outside.id.clone());
    timing.nodes.push(outside);
    let snapshot = Snapshot::new(doc, ValidationLimits::default()).unwrap();
    assert!(
        edit(
            &snapshot,
            vec![Operation::DeleteObject {
                object: id(),
                policy: DeletePolicy::RejectDependencies
            }]
        )
        .is_err()
    );
    let result = edit(
        &snapshot,
        vec![Operation::DeleteObject {
            object: id(),
            policy: DeletePolicy::Cascade,
        }],
    )
    .unwrap();
    assert!(result.snapshot.document().timelines.is_empty());
    assert!(result.snapshot.document().objects.contains_key(&surviving));
    assert_eq!(snapshot.document().timelines[&slide_id()].node_count(), 3);
    assert_eq!(result.receipt.changes.changed_timelines, vec![slide_id()]);
}
