use mo_common::*;
use mo_presentation_compile::{playback::*, *};
use mo_raster::{BackendReply, RasterBackend, RasterError};
use mo_timeline::*;
use std::cell::Cell;
fn request() -> PageRenderRequest {
    serde_json::from_str(include_str!(
        "../../../fixtures/presentations/playback/page.json"
    ))
    .unwrap()
}
fn binding() -> PlaybackBinding {
    PlaybackBinding {
        session: PlaybackSessionId::new("playback").unwrap(),
        revision: Digest::from_sha256([1; 32]),
        generation: PlaybackGeneration::new(1),
    }
}
fn time(n: i64, d: u32) -> RationalTime {
    RationalTime::new(n, d).unwrap()
}
#[test]
fn integer_samples_share_static_geometry_without_mutating_the_plan() {
    let q = request();
    let original = q.page.document.semantic_digest().unwrap();
    let mut plan =
        PlaybackPagePlan::new(q.clone(), binding(), TimelineLimits::default(), &|| false).unwrap();
    for at in [time(0, 1), time(1, 1), time(2, 1), time(1, 1), time(0, 1)] {
        let frame = plan.compile_frame(at, None, &|| false).unwrap();
        let mut static_q = q.clone();
        for (id, r) in &frame.frame.state.rotations {
            assert_eq!(r.denominator, "1");
            static_q
                .page
                .document
                .objects
                .get_mut(id)
                .unwrap()
                .transform
                .rotation = r.numerator.parse().unwrap();
        }
        let expected = compile_page(&static_q, &|| false).unwrap();
        assert_eq!(
            serde_json::to_value(&frame.page.raster).unwrap(),
            serde_json::to_value(&expected.raster).unwrap()
        );
        assert_eq!(
            frame.page.combined_coordinate_error_bound,
            expected.combined_coordinate_error_bound
        );
        assert_eq!(&frame.page.info.document_sha256, &original);
    }
    assert_eq!(plan.document_sha256(), &original);
    assert_eq!(q.page.document.semantic_digest().unwrap(), original);
}
#[test]
fn fractional_rotation_survives_sampling_and_is_not_rounded_to_native_angle_units() {
    let q = request();
    let mut plan =
        PlaybackPagePlan::new(q.clone(), binding(), TimelineLimits::default(), &|| false).unwrap();
    let f = plan.compile_frame(time(1, 3), None, &|| false).unwrap();
    let id = ObjectId::new("shape:1").unwrap();
    assert_eq!(f.frame.state.rotations[&id].denominator, "3");
    let mut rounded = q.clone();
    for (id, r) in &f.frame.state.rotations {
        let value = r.numerator.parse::<i64>().unwrap() / r.denominator.parse::<i64>().unwrap();
        rounded
            .page
            .document
            .objects
            .get_mut(id)
            .unwrap()
            .transform
            .rotation = value as i32;
    }
    let different = page_placements(&rounded.page, &|| false).unwrap();
    assert_ne!(
        serde_json::to_value(&f.placements.surfaces).unwrap(),
        serde_json::to_value(different.surfaces).unwrap()
    );
    assert!(f.page.combined_coordinate_error_bound <= q.viewport.coordinate_tolerance);
}
struct Never;
impl RasterBackend for Never {
    fn raster(&mut self, _: &[u32]) -> Result<BackendReply, RasterError> {
        panic!("invalid frame reached backend")
    }
    fn invalidate(&mut self) {
        panic!("unstarted backend invalidated")
    }
}
#[test]
fn event_gaps_and_unimplemented_paint_never_reach_the_backend() {
    let mut q = request();
    q.page.document.timelines.values_mut().next().unwrap().nodes[0].start = StartCondition::Click {
        target: None,
        delay: time(0, 1),
    };
    let mut plan =
        PlaybackPagePlan::new(q, binding(), TimelineLimits::default(), &|| false).unwrap();
    assert!(matches!(
        plan.render_frame(time(1, 1), None, &mut Never, &|| false),
        Err(PlaybackError::Time(TimelineError::MissingEventHistory))
    ));
    let mut q = request();
    q.page
        .document
        .objects
        .get_mut(&ObjectId::new("shape:1").unwrap())
        .unwrap()
        .appearance
        .fill = mo_presentation_model::Inherited::Inherit;
    let mut plan =
        PlaybackPagePlan::new(q, binding(), TimelineLimits::default(), &|| false).unwrap();
    assert!(matches!(
        plan.render_frame(time(1, 1), None, &mut Never, &|| false),
        Err(PlaybackError::Page(PageError::Unsupported { .. }))
    ));
}
#[test]
fn cancellation_does_not_poison_a_reusable_plan() {
    let create = || {
        PlaybackPagePlan::new(request(), binding(), TimelineLimits::default(), &|| false).unwrap()
    };
    for warm in [false, true] {
        let mut probe = create();
        if warm {
            probe.compile_frame(time(0, 1), None, &|| false).unwrap();
        }
        let steps = Cell::new(0);
        let expected = probe
            .compile_frame(time(1, 3), None, &|| {
                steps.set(steps.get() + 1);
                false
            })
            .unwrap();
        for stop in 0..steps.get() {
            let mut plan = create();
            if warm {
                plan.compile_frame(time(0, 1), None, &|| false).unwrap();
            }
            let current = Cell::new(0);
            assert!(
                plan.compile_frame(time(1, 3), None, &|| {
                    let cancel = current.get() == stop;
                    current.set(current.get() + 1);
                    cancel
                })
                .is_err()
            );
            assert_eq!(
                serde_json::to_value(plan.compile_frame(time(1, 3), None, &|| false).unwrap())
                    .unwrap(),
                serde_json::to_value(&expected).unwrap()
            );
        }
    }
}
