use mo_common::*;
use mo_timeline::*;

fn time(ms: i64) -> RationalTime {
    RationalTime::new(ms, 1000).unwrap()
}
fn binding() -> PlaybackBinding {
    PlaybackBinding {
        session: PlaybackSessionId::new("visibility").unwrap(),
        revision: Digest::from_sha256([7; 32]),
        generation: PlaybackGeneration::new(1),
    }
}
fn node(id: &str, start: i64, value: Visibility, fill: FillMode) -> TimingNode {
    TimingNode {
        id: TimingNodeId::new(id).unwrap(),
        restart: RestartMode::Never,
        start: TimeCondition::At {
            offset: time(start),
        }
        .into(),
        duration: time(1000),
        repeat_milli: 1000.into(),
        repeat_duration: None,
        end_conditions: vec![],
        fill,
        time_transform: None,
        effect: Effect::SetVisibility {
            target: ObjectId::new("shape").unwrap(),
            value,
        },
    }
}
fn timeline(nodes: Vec<TimingNode>) -> Timeline {
    Timeline {
        format: TimelineVersion::V01,
        tree: None,
        nodes,
    }
}
fn value(frame: &EvaluatedFrame) -> Option<Visibility> {
    frame
        .state
        .visibility
        .get(&ObjectId::new("shape").unwrap())
        .copied()
}
fn sample(t: &Timeline, ms: i64) -> EvaluatedFrame {
    TimelinePlan::compile(t, TimelineLimits::default(), &|| false)
        .unwrap()
        .evaluate(&binding(), time(ms), None, &|| false)
        .unwrap()
}
#[test]
fn fixed_value_has_exact_activation_and_remove_boundaries_without_implicit_entrance() {
    for v in [Visibility::Visible, Visibility::Hidden] {
        let t = timeline(vec![node("set", 500, v, FillMode::Remove)]);
        for (ms, expected) in [
            (0, None),
            (499, None),
            (500, Some(v)),
            (1499, Some(v)),
            (1500, None),
        ] {
            let f = sample(&t, ms);
            assert_eq!(value(&f), expected, "at {ms}");
            assert_eq!(f.state.profile, PROPERTY_FRAME_PROFILE);
            assert!(f.state.rotations.is_empty() && f.state.scales.is_empty());
        }
    }
}
#[test]
fn removing_later_assignment_reveals_underlying_held_value() {
    let t = timeline(vec![
        node("hidden", 0, Visibility::Hidden, FillMode::Hold),
        node("visible", 500, Visibility::Visible, FillMode::Remove),
    ]);
    for (ms, v) in [
        (0, Visibility::Hidden),
        (500, Visibility::Visible),
        (1499, Visibility::Visible),
        (1500, Visibility::Hidden),
        (10000, Visibility::Hidden),
    ] {
        assert_eq!(value(&sample(&t, ms)), Some(v));
    }
    let t = timeline(vec![
        node("a", 0, Visibility::Visible, FillMode::Hold),
        node("b", 0, Visibility::Hidden, FillMode::Hold),
    ]);
    assert_eq!(value(&sample(&t, 0)), Some(Visibility::Hidden));
}
#[test]
fn visibility_is_discrete_through_reverse_repeat_and_other_property_channels() {
    let mut set = node("set", 0, Visibility::Hidden, FillMode::Hold);
    set.repeat_milli = 2500.into();
    set.time_transform = Some(TimeTransform {
        speed_milli_percent: -125000,
        auto_reverse: true,
        acceleration_milli_percent: 25000,
        deceleration_milli_percent: 25000,
    });
    let mut rotate = set.clone();
    rotate.id = TimingNodeId::new("rotate").unwrap();
    rotate.effect = Effect::Rotation {
        composition: Default::default(),
        target: set.target().clone(),
        from: 0,
        to: 60000,
    };
    let t = timeline(vec![set, rotate]);
    for ms in [0, 125, 333, 1000, 4000, 10000] {
        let f = sample(&t, ms);
        assert_eq!(value(&f), Some(Visibility::Hidden));
        assert_eq!(f.state.rotations.len(), 1);
    }
}
#[test]
fn seeking_and_restarting_use_explicit_event_history_and_restore_base() {
    let mut n = node("set", 0, Visibility::Hidden, FillMode::Remove);
    n.restart = RestartMode::Always;
    n.start = TimeCondition::Click {
        target: None,
        delay: time(0),
    }
    .into();
    let t = timeline(vec![n]);
    let plan = TimelinePlan::compile(&t, TimelineLimits::default(), &|| false).unwrap();
    assert!(matches!(
        plan.evaluate(&binding(), time(0), None, &|| false),
        Err(TimelineError::MissingEventHistory)
    ));
    let history = EventHistory {
        binding: binding(),
        through: time(5000),
        events: [500, 750, 2500]
            .into_iter()
            .enumerate()
            .map(|(i, ms)| PlaybackEvent {
                generation: binding().generation,
                sequence: i as u32 + 1,
                at: time(ms),
                event: InputEvent::Click { target: None },
            })
            .collect(),
    };
    let mut retained = TimelineSampler::new(plan.clone());
    for (ms, expected) in [
        (0, None),
        (500, Some(Visibility::Hidden)),
        (1600, Some(Visibility::Hidden)),
        (1750, None),
        (2500, Some(Visibility::Hidden)),
        (5000, None),
        (750, Some(Visibility::Hidden)),
        (0, None),
    ] {
        let f = retained
            .evaluate(&binding(), time(ms), Some(&history), &|| false)
            .unwrap();
        assert_eq!(value(&f), expected, "at {ms}");
        assert_eq!(
            f,
            plan.evaluate(&binding(), time(ms), Some(&history), &|| false)
                .unwrap()
        );
    }
}
#[test]
fn removing_parent_suppresses_held_child_and_no_visibility_is_invented_before_parent() {
    let child = node("child", 0, Visibility::Hidden, FillMode::Hold);
    let parent = TimingContainer {
        time_transform: None,
        presentation: None,
        id: TimingNodeId::new("parent").unwrap(),
        restart: RestartMode::Never,
        kind: ContainerKind::Parallel,
        navigation: None,
        start: TimeCondition::At { offset: time(500) }.into(),
        end_conditions: vec![],
        duration: ContainerDuration::Fixed {
            duration: time(2000),
        },
        fill: FillMode::Remove,
        children: vec![child.id.clone()],
    };
    let t = Timeline {
        format: TimelineVersion::V02,
        nodes: vec![child],
        tree: Some(TimingTree {
            roots: vec![parent.id.clone()],
            containers: vec![parent],
        }),
    };
    for (ms, expected) in [
        (0, None),
        (500, Some(Visibility::Hidden)),
        (2400, Some(Visibility::Hidden)),
        (2500, None),
    ] {
        assert_eq!(value(&sample(&t, ms)), expected);
    }
}
#[test]
fn invalid_discrete_values_are_not_coerced_and_empty_channel_is_wire_compatible() {
    let t = timeline(vec![node(
        "set",
        500,
        Visibility::Visible,
        FillMode::Remove,
    )]);
    let mut wire = serde_json::to_value(&t).unwrap();
    wire["nodes"][0]["effect"]["value"] = "collapse".into();
    assert!(serde_json::from_value::<Timeline>(wire).is_err());
    assert!(
        serde_json::to_value(sample(&t, 0)).unwrap()["state"]
            .get("visibility")
            .is_none()
    );
    let plan = TimelinePlan::compile(&t, TimelineLimits::default(), &|| false).unwrap();
    assert_eq!(plan.targets().len(), 1);
    assert!(matches!(
        plan.evaluate(&binding(), time(0), None, &|| true),
        Err(TimelineError::Cancelled)
    ));
}

fn presentation_timeline(presets: &[PresentationPreset]) -> Timeline {
    let nodes: Vec<_> = presets
        .iter()
        .enumerate()
        .map(|(i, preset)| {
            let mut n = node(
                &format!("n{i}"),
                500 + i as i64 * 2000,
                if *preset == PresentationPreset::Disappear {
                    Visibility::Hidden
                } else {
                    Visibility::Visible
                },
                FillMode::Remove,
            );
            if *preset == PresentationPreset::Spin {
                n.effect = Effect::Rotation {
                    composition: Default::default(),
                    target: n.target().clone(),
                    from: 0,
                    to: 60000,
                };
            }
            n
        })
        .collect();
    let containers: Vec<_> = nodes
        .iter()
        .zip(presets)
        .enumerate()
        .map(|(i, (n, preset))| TimingContainer {
            time_transform: None,
            id: TimingNodeId::new(format!("p{i}")).unwrap(),
            presentation: Some(PresentationRole::Effect {
                preset: *preset,
                trigger: PresentationTrigger::Click,
            }),
            navigation: None,
            restart: RestartMode::Never,
            kind: ContainerKind::Parallel,
            start: TimeCondition::At { offset: time(0) }.into(),
            end_conditions: vec![],
            duration: ContainerDuration::Indefinite,
            fill: FillMode::Hold,
            children: vec![n.id.clone()],
        })
        .collect();
    Timeline {
        format: TimelineVersion::V02,
        nodes,
        tree: Some(TimingTree {
            roots: containers.iter().map(|c| c.id.clone()).collect(),
            containers,
        }),
    }
}
#[test]
fn presentation_baseline_uses_structural_first_effect_and_survives_removal() {
    let t = presentation_timeline(&[PresentationPreset::Appear, PresentationPreset::Disappear]);
    assert_eq!(value(&sample(&t, 0)), Some(Visibility::Hidden));
    assert_eq!(value(&sample(&t, 500)), Some(Visibility::Visible));
    assert_eq!(value(&sample(&t, 1500)), Some(Visibility::Hidden));
    let mut reverse = t.clone();
    reverse.tree.as_mut().unwrap().roots.reverse();
    assert_eq!(value(&sample(&reverse, 0)), None);
    assert_eq!(value(&sample(&reverse, 1500)), None);
    let t = presentation_timeline(&[PresentationPreset::Spin, PresentationPreset::Appear]);
    assert_eq!(value(&sample(&t, 0)), None);
    assert_eq!(value(&sample(&t, 2500)), Some(Visibility::Visible));
    assert_eq!(value(&sample(&t, 3500)), None);
}
#[test]
fn presentation_identity_is_checked_against_structure_payload_and_wire_contract() {
    let t = presentation_timeline(&[PresentationPreset::Appear]);
    for variant in 0..4 {
        let mut t = t.clone();
        let c = &mut t.tree.as_mut().unwrap().containers[0];
        match variant {
            0 => c.presentation = Some(PresentationRole::MainSequence),
            1 => c.children.clear(),
            2 => c.kind = ContainerKind::Sequence,
            _ => {
                c.presentation = Some(PresentationRole::Effect {
                    preset: PresentationPreset::Disappear,
                    trigger: PresentationTrigger::Click,
                })
            }
        }
        assert!(TimelinePlan::compile(&t, TimelineLimits::default(), &|| false).is_err());
    }
    let mut wire = serde_json::to_value(&t).unwrap();
    wire["tree"]["containers"][0]["presentation"]["preset"] = "unimplemented".into();
    assert!(serde_json::from_value::<Timeline>(wire).is_err());
}
