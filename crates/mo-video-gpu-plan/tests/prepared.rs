use mo_common::{Digest, RationalTime, ResourceId, Ticks};
use mo_time::{ExactInterval, ExactRatio, SampleSide};
use mo_video_compile::*;
use mo_video_gpu_plan::*;
use mo_video_model::*;
use mo_video_render::*;
use std::cell::Cell;
fn sample(side: SampleSide) -> TimeSample {
    TimeSample {
        time: ExactRatio::new(1.into(), 9999999967u64.into(), 128).unwrap(),
        side,
    }
}
fn binding() -> MediaBinding {
    MediaBinding {
        resource: ResourceId::new("movie").unwrap(),
        digest: Digest::from_sha256([3; 32]),
        stream: StreamId::new("v").unwrap(),
        descriptor: MediaStream::Video {
            available: TimeRange {
                start: RationalTime::new(0, 1).unwrap(),
                end: RationalTime::new(1, 1).unwrap(),
            },
            dimensions: Canvas {
                width: 2,
                height: 3,
            },
            frame_rate: None,
        },
    }
}
fn media(id: &str, side: SampleSide) -> VisualNode {
    VisualNode {
        track: TrackId::new("v").unwrap(),
        clip: ClipId::new(id).unwrap(),
        local: sample(side),
        effects: vec![],
        source: VisualSource::Media {
            binding: binding(),
            mapping: MappedTime {
                sample: sample(side),
                rate: ExactRatio::integer(-1),
            },
        },
    }
}
fn plan() -> VideoFramePlan {
    let group = VisualNode {
        track: TrackId::new("group").unwrap(),
        clip: ClipId::new("group").unwrap(),
        local: sample(SampleSide::After),
        effects: vec![VisualEffect::Opacity {
            factor: ExactRatio::new(1.into(), 2.into(), 128).unwrap(),
        }],
        source: VisualSource::Group {
            sequence: SequenceId::new("child").unwrap(),
            mapping: MappedTime {
                sample: sample(SampleSide::After),
                rate: ExactRatio::integer(1),
            },
            layers: vec![
                media("a", SampleSide::Before),
                media("b", SampleSide::After),
                media("again", SampleSide::Before),
            ],
        },
    };
    VideoFramePlan {
        document_digest: Digest::from_sha256([4; 32]),
        frame_index: Ticks::new(0),
        sample: sample(SampleSide::After),
        canvas: Canvas {
            width: 120,
            height: 80,
        },
        layers: vec![group],
    }
}
fn options() -> FrameRenderOptions {
    FrameRenderOptions {
        raster: RasterExtent {
            width: 12,
            height: 8,
        },
        sampling: Sampling::Bilinear,
        limits: RenderLimits::default(),
    }
}
fn prepared(limits: ProgramLimits) -> PreparedGpuFrame {
    PreparedGpuFrame::new(plan(), options(), limits, &|| false).unwrap()
}
fn replies(p: &PreparedGpuFrame) -> Vec<VisualReply> {
    p.requests()
        .iter()
        .map(|r| VisualReply {
            request: r.key.clone(),
            binding: r.binding.clone(),
            visual: match r.sample.side {
                SampleSide::Before => VisualDescription::VideoFrame {
                    interval: ExactInterval {
                        start: ExactRatio::integer(0),
                        end: r.sample.time.clone(),
                    },
                    image: ImageDescription {
                        width: 2,
                        height: 3,
                        has_hdr: true,
                        display: DisplayGeometry {
                            sample_aspect_ratio: [2, 1],
                            matrix: Some([0., 1., 0., -1., 0., 0., 7., -9., 1.]),
                        },
                    },
                },
                SampleSide::After => VisualDescription::VideoGap {
                    interval: ExactInterval {
                        start: r.sample.time.clone(),
                        end: ExactRatio::integer(1),
                    },
                },
            },
        })
        .collect()
}
#[test]
fn exact_requests_deduplicate_without_collapsing_sides_and_gap_does_not_paint() {
    let p = prepared(ProgramLimits::default());
    assert_eq!(p.requests().len(), 2);
    assert_ne!(p.requests()[0].key, p.requests()[1].key);
    let result = p.compile(replies(&p), &|| false).unwrap();
    assert_eq!(result.images.len(), 1);
    assert!(result.has_hdr);
    assert_eq!((result.width, result.height, result.output), (12, 8, 0));
    assert_eq!(result.commands.len(), 5); // two surfaces, image twice, then whole group
    assert_eq!(result.pixel_visits, 12 * 8 * 3);
    assert_eq!(
        result.allocated_bytes,
        16 + 2 * 12 * 8 * 16 + 2 * 3 * 16 + 3 * 112
    );
    assert!(matches!(
        result.commands.last(),
        Some(GpuCommand::Paint {
            target: 0,
            source: GpuSource::Surface { id: 1 },
            ..
        })
    ));
    let mut reversed = replies(&p);
    reversed.reverse();
    assert_eq!(
        serde_json::to_string(&result).unwrap(),
        serde_json::to_string(&p.compile(reversed, &|| false).unwrap()).unwrap()
    );
}
#[test]
fn stale_wrong_kind_dimensions_geometry_and_intervals_never_emit_partial_programs() {
    let p = prepared(ProgramLimits::default());
    for fault in 0..9 {
        let mut r = replies(&p);
        match fault {
            0 => {
                r.pop();
            }
            1 => r[1] = r[0].clone(),
            2 => r[0].request = Digest::from_sha256([99; 32]),
            3 => r[0].binding.digest = Digest::from_sha256([99; 32]),
            4 => {
                if let VisualDescription::VideoFrame { image, .. } = &mut r[0].visual {
                    image.width = 3
                }
            }
            5 => {
                if let VisualDescription::VideoFrame { interval, .. } = &mut r[0].visual {
                    interval.end = ExactRatio::integer(0)
                }
            }
            6 => {
                if let VisualDescription::VideoFrame { image, .. } = &mut r[0].visual {
                    image.display.sample_aspect_ratio = [0, 1]
                }
            }
            7 => {
                if let VisualDescription::VideoFrame { image, .. } = &mut r[0].visual {
                    image.display.matrix = Some([0.; 9])
                }
            }
            8 => {
                r[1].visual = VisualDescription::VideoGap {
                    interval: ExactInterval {
                        start: ExactRatio::integer(0),
                        end: p.requests()[1].sample.time.clone(),
                    },
                }
            }
            _ => unreachable!(),
        }
        assert!(p.compile(r, &|| false).is_err(), "fault {fault}");
        assert_eq!(p.compile(replies(&p), &|| false).unwrap().commands.len(), 5);
    }
}
#[test]
fn every_observed_cancel_and_exact_budget_boundary_is_atomic() {
    let p = prepared(ProgramLimits::default());
    let full = p.compile(replies(&p), &|| false).unwrap();
    for limits in [
        ProgramLimits {
            max_allocated_bytes: full.allocated_bytes - 1,
            ..Default::default()
        },
        ProgramLimits {
            max_commands: 4,
            ..Default::default()
        },
    ] {
        let p = prepared(limits);
        assert!(matches!(
            p.compile(replies(&p), &|| false),
            Err(RenderError::Budget)
        ));
    }
    let p = prepared(ProgramLimits {
        max_allocated_bytes: full.allocated_bytes,
        max_commands: 5,
        ..Default::default()
    });
    p.compile(replies(&p), &|| false).unwrap();
    assert!(matches!(
        PreparedGpuFrame::new(
            plan(),
            options(),
            ProgramLimits {
                max_images: 1,
                ..Default::default()
            },
            &|| false
        ),
        Err(RenderError::Budget)
    ));
    let calls = Cell::new(0);
    p.compile(replies(&p), &|| {
        calls.set(calls.get() + 1);
        false
    })
    .unwrap();
    for at in 1..=calls.get() {
        let n = Cell::new(0);
        assert!(matches!(
            p.compile(replies(&p), &|| {
                n.set(n.get() + 1);
                n.get() == at
            }),
            Err(RenderError::Cancelled)
        ));
    }
}
#[test]
fn unsupported_text_is_rejected_before_requesting_any_media() {
    let mut value = plan();
    value.layers.push(VisualNode {
        track: TrackId::new("v").unwrap(),
        clip: ClipId::new("title").unwrap(),
        local: sample(SampleSide::After),
        effects: vec![],
        source: VisualSource::Text {
            text: "Title".into(),
            font: ResourceId::new("font").unwrap(),
            font_digest: Digest::from_sha256([1; 32]),
            face_index: 0,
            font_size_milli_pixels: 24000,
            rgba: [255; 4],
        },
    });
    assert!(matches!(
        PreparedGpuFrame::new(value, options(), ProgramLimits::default(), &|| false),
        Err(RenderError::Unsupported(_))
    ));
}
#[test]
fn normalized_container_matrix_keeps_sar_rotation_flip_translation_and_projective_rejection() {
    let g = SourceGeometry::from_display_metadata(
        (2, 1),
        Some([0., 2., 0., -2., 0., 0., 14., -18., 2.]),
    )
    .unwrap();
    assert_eq!(g.pixel_to_display.map(3., 4.), (3., -3.));
    let fitted = g.contain((2, 3), (120, 80)).unwrap();
    assert_eq!(fitted.map(0., 0.), (90., 0.));
    assert_eq!(fitted.map(2., 3.), (30., 80.));
    assert!(
        SourceGeometry::from_display_metadata((1, 1), Some([1., 0., 0.1, 0., 1., 0., 0., 0., 1.]))
            .is_err()
    );
}
