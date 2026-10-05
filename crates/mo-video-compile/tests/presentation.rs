mod support;
use mo_common::{Digest, RationalTime, ResourceId};
use mo_time::ExactRatio;
use mo_video_compile::*;
use mo_video_model::*;
use support::{range, t};

fn binding() -> MediaBinding {
    MediaBinding {
        resource: ResourceId::new("movie").unwrap(),
        digest: Digest::from_sha256([1; 32]),
        stream: StreamId::new("video").unwrap(),
        descriptor: MediaStream::Video {
            available: range(-1, 2),
            dimensions: Canvas {
                width: 100,
                height: 100,
            },
            frame_rate: None,
        },
    }
}
fn window() -> PresentationWindow {
    let r = |a, b| TimeRange {
        start: RationalTime::new(a, 1000).unwrap(),
        end: RationalTime::new(b, 1000).unwrap(),
    };
    PresentationWindow {
        resource_digest: binding().digest,
        stream: binding().stream,
        coverage: r(-50, 160),
        samples: vec![
            PresentationSample {
                id: 42,
                interval: r(-50, -10),
            },
            PresentationSample {
                id: 9,
                interval: r(-10, 23),
            },
            PresentationSample {
                id: 88,
                interval: r(23, 65),
            },
            PresentationSample {
                id: 6,
                interval: r(100, 160),
            },
        ],
    }
}
fn sample(ms: i64, side: SampleSide) -> TimeSample {
    TimeSample {
        time: ExactRatio::from_time(RationalTime::new(ms, 1000).unwrap()),
        side,
    }
}

#[test]
fn vfr_negative_pts_and_reverse_boundaries_select_presentation_intervals() {
    let index = PresentationIndex::new(window(), PresentationLimits::default(), &|| false).unwrap();
    for (ms, side, id) in [
        (-50, SampleSide::After, 42),
        (-10, SampleSide::Before, 42),
        (-10, SampleSide::After, 9),
        (23, SampleSide::Before, 9),
        (23, SampleSide::After, 88),
        (64, SampleSide::After, 88),
        (160, SampleSide::Before, 6),
    ] {
        let selected = index
            .select(&binding(), &sample(ms, side), &|| false)
            .unwrap();
        let PresentationSelection::Sample { sample } = selected else {
            panic!("{selected:?}")
        };
        assert_eq!(sample.id, id);
    }
}

#[test]
fn gaps_and_unindexed_windows_are_distinct_and_cannot_be_substituted_by_nearest_frame() {
    let index = PresentationIndex::new(window(), PresentationLimits::default(), &|| false).unwrap();
    for ms in [65, 90] {
        assert_eq!(
            index
                .select(&binding(), &sample(ms, SampleSide::After), &|| false)
                .unwrap(),
            PresentationSelection::Gap
        );
    }
    for (ms, side) in [
        (-51, SampleSide::After),
        (-50, SampleSide::Before),
        (160, SampleSide::After),
    ] {
        assert_eq!(
            index
                .select(&binding(), &sample(ms, side), &|| false)
                .unwrap(),
            PresentationSelection::OutsideWindow
        );
    }
    let mut empty = window();
    empty.samples.clear();
    let index = PresentationIndex::new(empty, PresentationLimits::default(), &|| false).unwrap();
    assert_eq!(
        index
            .select(&binding(), &sample(0, SampleSide::After), &|| false)
            .unwrap(),
        PresentationSelection::Gap
    );
}

#[test]
fn changed_resource_stream_and_wrong_media_kind_do_not_reuse_stale_indexes() {
    let index = PresentationIndex::new(window(), PresentationLimits::default(), &|| false).unwrap();
    let original = binding();
    let mut changed = original.clone();
    changed.digest = Digest::from_sha256([2; 32]);
    let mut stream = original.clone();
    stream.stream = StreamId::new("other").unwrap();
    let mut image = original.clone();
    image.descriptor = MediaStream::Image {
        dimensions: Canvas {
            width: 100,
            height: 100,
        },
    };
    let mut availability = original;
    availability.descriptor = MediaStream::Video {
        available: TimeRange {
            start: t(0),
            end: t(1),
        },
        dimensions: Canvas {
            width: 100,
            height: 100,
        },
        frame_rate: None,
    };
    for binding in [changed, stream, image, availability] {
        assert!(matches!(
            index.select(&binding, &sample(0, SampleSide::After), &|| false),
            Err(CompileError::Presentation(_))
        ));
    }
}

#[test]
fn malformed_windows_and_excessive_admission_are_rejected_without_sorting_away_evidence() {
    let mut duplicate = window();
    duplicate.samples[1].id = 42;
    let mut reordered = window();
    reordered.samples.swap(0, 1);
    let mut overlap = window();
    overlap.samples[1].interval.start = RationalTime::new(-20, 1000).unwrap();
    let mut empty = window();
    empty.samples[1].interval.end = empty.samples[1].interval.start;
    let mut outside = window();
    outside.coverage.start = t(0);
    for window in [duplicate, reordered, overlap, empty, outside] {
        assert!(matches!(
            PresentationIndex::new(window, PresentationLimits::default(), &|| false),
            Err(CompileError::Presentation(_))
        ));
    }
    assert!(matches!(
        PresentationIndex::new(
            window(),
            PresentationLimits {
                max_samples: 3,
                ..Default::default()
            },
            &|| false
        ),
        Err(CompileError::Budget)
    ));
    assert!(matches!(
        PresentationIndex::new(window(), PresentationLimits::default(), &|| true),
        Err(CompileError::Cancelled)
    ));
}
