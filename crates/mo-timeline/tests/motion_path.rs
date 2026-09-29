use mo_common::*;
use mo_timeline::*;
use std::cell::Cell;

fn p(x: &str, y: &str) -> MotionPoint {
    MotionPoint {
        x: x.to_owned().try_into().unwrap(),
        y: y.to_owned().try_into().unwrap(),
    }
}
fn t(n: i64) -> RationalTime {
    RationalTime::new(n, 1000).unwrap()
}
fn timeline(segments: Vec<MotionSegment>) -> Timeline {
    Timeline {
        format: TimelineVersion::V01,
        tree: None,
        nodes: vec![TimingNode {
            id: TimingNodeId::new("paced").unwrap(),
            restart: RestartMode::Never,
            start: TimeCondition::At { offset: t(0) }.into(),
            end_conditions: vec![],
            duration: t(1000),
            repeat_milli: 1000.into(),
            repeat_duration: None,
            fill: FillMode::Hold,
            time_transform: None,
            effect: Effect::MotionPath {
                target: ObjectId::new("shape").unwrap(),
                path: MotionPath {
                    from: p("0", "0"),
                    segments,
                },
            },
        }],
    }
}
fn compile(t: &Timeline) -> TimelinePlan {
    TimelinePlan::compile(t, TimelineLimits::default(), &|| false).unwrap()
}
fn sample(plan: &TimelinePlan, ms: i64) -> FrameState {
    plan.evaluate(
        &PlaybackBinding {
            session: PlaybackSessionId::new("motion-path").unwrap(),
            revision: Digest::from_sha256([4; 32]),
            generation: PlaybackGeneration::new(1),
        },
        t(ms),
        None,
        &|| false,
    )
    .unwrap()
    .state
}
fn coordinates(plan: &TimelinePlan, ms: i64) -> [f64; 2] {
    let state = sample(plan, ms);
    assert_eq!(state.profile, PACED_MOTION_FRAME_PROFILE);
    let motion = state.motion.values().next().unwrap();
    let value = |v: &ExactValue| {
        v.numerator.parse::<f64>().unwrap() / v.denominator.parse::<f64>().unwrap()
    };
    [value(&motion.x), value(&motion.y)]
}
fn close(a: [f64; 2], b: [f64; 2], tolerance: f64) {
    assert!(
        (a[0] - b[0]).hypot(a[1] - b[1]) <= tolerance,
        "{a:?} != {b:?}"
    );
}
#[test]
fn nonuniform_collinear_cubic_and_unequal_segments_are_distance_paced() {
    let curve = compile(&timeline(vec![MotionSegment::Cubic {
        control1: p("0", "0"),
        control2: p("0", "0"),
        to: p("0.3", "0"),
    }]));
    let lines = compile(&timeline(vec![
        MotionSegment::Line { to: p("0.03", "0") },
        MotionSegment::Line { to: p("0.3", "0") },
    ]));
    for ms in [0, 1, 125, 333, 500, 999, 1000, 2000] {
        let expected = [0.3 * ms.min(1000) as f64 / 1000.0, 0.0];
        close(coordinates(&curve, ms), expected, 1e-15);
        close(coordinates(&lines, ms), expected, 1e-15);
    }
}
#[test]
fn normalized_axes_corners_close_zero_segments_and_exact_endpoints() {
    let mut q = timeline(vec![
        MotionSegment::Line { to: p("0", "0") },
        MotionSegment::Line { to: p("0.3", "0") },
        MotionSegment::Line {
            to: p("0.3", "0.3"),
        },
    ]);
    let plan = compile(&q);
    close(coordinates(&plan, 250), [0.15, 0.0], 1e-15);
    close(coordinates(&plan, 500), [0.3, 0.0], 1e-15);
    close(coordinates(&plan, 750), [0.3, 0.15], 1e-15);
    let Effect::MotionPath { path, .. } = &mut q.nodes[0].effect else {
        unreachable!()
    };
    path.segments = vec![
        MotionSegment::Line {
            to: p("0.300000000000000001", "0"),
        },
        MotionSegment::Close,
    ];
    let plan = compile(&q);
    close(coordinates(&plan, 250), [0.15, 0.0], 1e-15);
    close(coordinates(&plan, 750), [0.15, 0.0], 1e-15);
    assert_eq!(
        sample(&plan, 1000)
            .motion
            .values()
            .next()
            .unwrap()
            .x
            .numerator,
        "0"
    );
    let Effect::MotionPath { path, .. } = &mut q.nodes[0].effect else {
        unreachable!()
    };
    path.segments.pop();
    let frame = sample(&compile(&q), 1000);
    assert_eq!(
        frame.motion.values().next().unwrap().x,
        ExactValue {
            numerator: "300000000000000001".into(),
            denominator: "1000000000000000000".into()
        }
    );
}

fn bezier(p: [[f64; 2]; 4], t: f64) -> [f64; 2] {
    let u = 1.0 - t;
    std::array::from_fn(|i| {
        u * u * u * p[0][i]
            + 3.0 * u * u * t * p[1][i]
            + 3.0 * u * t * t * p[2][i]
            + t * t * t * p[3][i]
    })
}
// Independent reference: dense parameter integration + inverse arc lookup,
// with a second resolution checking convergence. No production table reused.
fn reference(p: [[f64; 2]; 4], samples: usize, progress: &[f64]) -> Vec<[f64; 2]> {
    let mut distances = vec![0.0];
    let mut previous = p[0];
    for i in 1..=samples {
        let point = bezier(p, i as f64 / samples as f64);
        distances.push(distances[i - 1] + (point[0] - previous[0]).hypot(point[1] - previous[1]));
        previous = point;
    }
    progress
        .iter()
        .map(|v| {
            let at = v * distances[samples];
            let i = distances.partition_point(|d| *d < at).max(1).min(samples);
            let ratio = (at - distances[i - 1]) / (distances[i] - distances[i - 1]);
            bezier(p, ((i - 1) as f64 + ratio) / samples as f64)
        })
        .collect()
}
#[test]
fn curved_loop_cusp_and_backtracking_paths_match_an_independent_converged_reference() {
    let cases = [
        [[0., 0.], [0., 1.], [1., 1.], [1., 0.]],
        [[0., 0.], [1., 1.], [-1., 1.], [0., 0.]],
        [[0., 0.], [2., 0.], [-1., 0.], [1., 0.]],
        [[0., 0.], [2., 3.], [-3., -2.], [1., 1.]],
    ];
    let times: Vec<_> = (1..40).map(|i| i * 25).collect();
    let progress: Vec<_> = times.iter().map(|t| *t as f64 / 1000.).collect();
    for points in cases {
        let point = |i: usize| p(&format!("{}", points[i][0]), &format!("{}", points[i][1]));
        let plan = compile(&timeline(vec![MotionSegment::Cubic {
            control1: point(1),
            control2: point(2),
            to: point(3),
        }]));
        let coarse = reference(points, 32768, &progress);
        let fine = reference(points, 65536, &progress);
        for ((ms, a), b) in times.iter().zip(coarse).zip(fine) {
            close(a, b, 1e-8);
            close(coordinates(&plan, *ms), b, 3e-6);
        }
    }
}
#[test]
fn immutable_path_sampling_obeys_repeat_reverse_easing_and_cancel() {
    let mut q = timeline(vec![MotionSegment::Cubic {
        control1: p("0", "1"),
        control2: p("1", "1"),
        to: p("1", "0"),
    }]);
    q.nodes[0].time_transform = Some(TimeTransform {
        auto_reverse: true,
        acceleration_milli_percent: 25000,
        deceleration_milli_percent: 25000,
        ..Default::default()
    });
    q.nodes[0].repeat_milli = 2000.into();
    let plan = compile(&q);
    for ms in [125, 250, 500, 750] {
        assert_eq!(sample(&plan, ms).motion, sample(&plan, 2000 - ms).motion);
        assert_eq!(sample(&plan, ms).motion, sample(&plan, 2000 + ms).motion);
    }
    let calls = Cell::new(0);
    let error = TimelinePlan::compile(&q, TimelineLimits::default(), &|| {
        calls.set(calls.get() + 1);
        calls.get() > 100
    })
    .unwrap_err();
    assert!(matches!(error, TimelineError::Cancelled));
}
#[test]
fn resource_limits_are_shared_and_never_coarsen_the_curve() {
    let mut q = timeline(vec![
        MotionSegment::Line { to: p("1", "0") },
        MotionSegment::Line { to: p("1", "1") },
    ]);
    let mut second = q.nodes[0].clone();
    second.id = TimingNodeId::new("second").unwrap();
    q.nodes.push(second);
    for limits in [
        TimelineLimits {
            max_motion_segments: 3,
            ..Default::default()
        },
        TimelineLimits {
            max_motion_vertices: 3,
            ..Default::default()
        },
        TimelineLimits {
            max_motion_steps: 3,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            TimelinePlan::compile(&q, limits, &|| false),
            Err(TimelineError::Limit(_))
        ));
    }
    let curve = timeline(vec![MotionSegment::Cubic {
        control1: p("0", "1"),
        control2: p("1", "1"),
        to: p("1", "0"),
    }]);
    assert!(matches!(
        TimelinePlan::compile(
            &curve,
            TimelineLimits {
                max_motion_vertices: 1,
                ..Default::default()
            },
            &|| false
        ),
        Err(TimelineError::Limit("motion path vertices"))
    ));
    assert!(
        TimelinePlan::compile(&timeline(vec![]), TimelineLimits::default(), &|| false).is_err()
    );
}
#[test]
fn path_contract_retains_controls_and_accepts_only_bounded_decimals() {
    let q = timeline(vec![
        MotionSegment::Cubic {
            control1: p("-0.1", "0.000000000000000001"),
            control2: p("0.2", "1"),
            to: p("0.3", "0"),
        },
        MotionSegment::Close,
    ]);
    let json = serde_json::to_string(&q).unwrap();
    assert_eq!(serde_json::from_str::<Timeline>(&json).unwrap(), q);
    assert!(
        serde_json::from_str::<Timeline>(&json.replace("0.000000000000000001", "1e-18")).is_err()
    );
}
