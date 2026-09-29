use mo_common::*;
use mo_timeline::*;
use serde_json::json;

fn time(ms: i64) -> RationalTime {
    RationalTime::new(ms, 1000).unwrap()
}
fn binding() -> PlaybackBinding {
    PlaybackBinding {
        session: PlaybackSessionId::new("rotation-stack").unwrap(),
        revision: Digest::from_sha256([71; 32]),
        generation: PlaybackGeneration::new(1),
    }
}
fn node(
    id: &str,
    mode: &str,
    from: i32,
    to: i32,
    start: i64,
    duration: i64,
    fill: &str,
) -> TimingNode {
    serde_json::from_value(json!({"id":id,"restart":"never","start":{"kind":"at","offset":{"ticks":start.to_string(),"timescale":1000}},"duration":{"ticks":duration.to_string(),"timescale":1000},"repeatMilli":1000,"fill":fill,"effect":{"kind":"rotation","target":"a","from":from,"to":to,"composition":mode}})).unwrap()
}
fn plan(nodes: Vec<TimingNode>) -> TimelinePlan {
    TimelinePlan::compile(
        &Timeline {
            format: TimelineVersion::V01,
            tree: None,
            nodes,
        },
        TimelineLimits::default(),
        &|| false,
    )
    .unwrap()
}
fn rotation(frame: &EvaluatedFrame, n: &str, d: &str, basis: RotationBasis) {
    assert_eq!(
        frame.state.rotations[&ObjectId::new("a").unwrap()],
        ExactRotation {
            numerator: n.into(),
            denominator: d.into(),
            basis
        }
    );
}
#[test]
fn relative_stack_uses_visible_values_and_reveals_lower_values_on_removal() {
    let p = plan(vec![
        node("first", "add", 0, 30, 0, 1000, "hold"),
        node("second", "add", 0, 70, 500, 1000, "remove"),
    ]);
    let mut retained = TimelineSampler::new(p.clone());
    for (at, n, d) in [
        (250, "15", "2"),
        (500, "15", "1"),
        (750, "40", "1"),
        (1000, "65", "1"),
        (1499, "9993", "100"),
        (1500, "30", "1"),
        (2500, "30", "1"),
        (500, "15", "1"),
    ] {
        let f = p.evaluate(&binding(), time(at), None, &|| false).unwrap();
        rotation(&f, n, d, RotationBasis::Layout);
        assert_eq!(
            retained
                .evaluate(&binding(), time(at), None, &|| false)
                .unwrap(),
            f
        );
    }
    let p = plan(vec![
        node("first", "add", 0, 30, 0, 1000, "remove"),
        node("second", "add", 0, 70, 500, 1000, "hold"),
    ]);
    rotation(
        &p.evaluate(&binding(), time(1500), None, &|| false).unwrap(),
        "70",
        "1",
        RotationBasis::Layout,
    );
}
#[test]
fn replacement_selects_one_basis_and_only_higher_priority_additions_survive() {
    // Traversal order intentionally differs from activation order. At 750,
    // absolute 100 plus the new +10 masks the older additive +30 and layout +80.
    let p = plan(vec![
        node("new-add", "add", 0, 40, 500, 1000, "hold"),
        node("older-add", "add", 0, 30, 0, 100, "hold"),
        node("layout", "layout", 80, 80, 200, 1000, "hold"),
        node("absolute", "absolute", 100, 100, 500, 500, "remove"),
    ]);
    let f = p.evaluate(&binding(), time(750), None, &|| false).unwrap();
    // Equal start: the later declared absolute behavior also masks new-add.
    rotation(&f, "100", "1", RotationBasis::Absolute);
    rotation(
        &p.evaluate(&binding(), time(1000), None, &|| false).unwrap(),
        "100",
        "1",
        RotationBasis::Layout,
    );
    let p = plan(vec![
        node("absolute", "absolute", 100, 100, 500, 500, "remove"),
        node("new-add", "add", 0, 40, 500, 1000, "hold"),
    ]);
    rotation(
        &p.evaluate(&binding(), time(750), None, &|| false).unwrap(),
        "110",
        "1",
        RotationBasis::Absolute,
    );
    rotation(
        &p.evaluate(&binding(), time(1000), None, &|| false).unwrap(),
        "20",
        "1",
        RotationBasis::Layout,
    );
}
#[test]
fn legacy_absolute_wire_shape_is_preserved_and_basis_is_not_lost() {
    let mut n = node("a", "absolute", -1, 1, 0, 3000, "hold");
    let v = serde_json::to_value(&n).unwrap();
    assert!(v["effect"].get("composition").is_none());
    let f = plan(vec![n.clone()])
        .evaluate(&binding(), time(1000), None, &|| false)
        .unwrap();
    rotation(&f, "-1", "3", RotationBasis::Absolute);
    let v = serde_json::to_value(&f).unwrap();
    assert_eq!(
        v["state"]["rotations"]["a"],
        json!({"numerator":"-1","denominator":"3"})
    );
    if let Effect::Rotation { composition, .. } = &mut n.effect {
        *composition = RotationComposition::Layout;
    }
    let other = plan(vec![n])
        .evaluate(&binding(), time(1000), None, &|| false)
        .unwrap();
    rotation(&other, "-1", "3", RotationBasis::Layout);
    assert_ne!(f.sha256, other.sha256);
    assert_eq!(
        serde_json::from_value::<EvaluatedFrame>(serde_json::to_value(&other).unwrap()).unwrap(),
        other
    );
}
