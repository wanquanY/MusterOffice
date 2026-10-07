mod support;
use mo_common::{Digest, ResourceId};
use mo_time::{ExactInterval, ExactRatio};
use mo_video_compile::{MediaBinding, VisualEffect, VisualSource};
use mo_video_model::*;
use mo_video_render::*;
use std::sync::Arc;
use support::*;
struct Source(ResolvedMedia);
impl VisualResources for Source {
    fn media(
        &mut self,
        _: &MediaBinding,
        _: &TimeSample,
        _: &dyn Fn() -> bool,
    ) -> Result<ResolvedMedia> {
        Ok(self.0.clone())
    }
}
fn binding() -> MediaBinding {
    MediaBinding {
        resource: ResourceId::new("video").unwrap(),
        digest: Digest::from_sha256([1; 32]),
        stream: StreamId::new("v").unwrap(),
        descriptor: MediaStream::Video {
            available: TimeRange {
                start: t(0),
                end: t(2),
            },
            dimensions: Canvas {
                width: 1,
                height: 1,
            },
            frame_rate: None,
        },
    }
}
fn visual(binding: MediaBinding, sample: TimeSample) -> VisualSource {
    VisualSource::Media {
        binding,
        mapping: MappedTime {
            sample,
            rate: ExactRatio::integer(1),
        },
    }
}
fn frame() -> VisualFrame {
    VisualFrame {
        image: Arc::new(
            normalize_frame(
                &rgba(&[[255, 0, 0, 255]]),
                srgb(),
                Default::default(),
                &|| false,
            )
            .unwrap()
            .image,
        ),
        geometry: SourceGeometry::square_pixels(),
    }
}
fn render(source: ResolvedMedia, sample: TimeSample) -> Result<RenderedVideo> {
    render_frame(
        &plan(vec![node("video", visual(binding(), sample))], 1, 1),
        &mut Source(source),
        Sampling::Nearest,
        Default::default(),
        &|| false,
    )
}
#[test]
fn exact_frame_and_gap_endpoints_preserve_before_after_without_quantizing() {
    let boundary = ExactRatio::new(1.into(), 9_999_999_967u64.into(), 128).unwrap();
    let picture = ResolvedMedia {
        binding: binding(),
        visual: ResolvedVisual::VideoFrame {
            interval: ExactInterval {
                start: ExactRatio::integer(0),
                end: boundary.clone(),
            },
            frame: frame(),
        },
    };
    let gap = ResolvedMedia {
        binding: binding(),
        visual: ResolvedVisual::VideoGap {
            interval: ExactInterval {
                start: boundary.clone(),
                end: ExactRatio::integer(2),
            },
        },
    };
    let before = TimeSample {
        time: boundary.clone(),
        side: SampleSide::Before,
    };
    let after = TimeSample {
        time: boundary,
        side: SampleSide::After,
    };
    assert_eq!(
        render(picture.clone(), before.clone())
            .unwrap()
            .image
            .to_srgb8(output(), &|| false)
            .unwrap()
            .rgba,
        [255, 0, 0, 255]
    );
    assert!(render(picture, after.clone()).is_err());
    assert!(render(gap.clone(), before).is_err());
    assert_eq!(render(gap, after).unwrap().image.pixels(), &[[0.; 4]]);
}
#[test]
fn a_gap_inside_an_opacity_group_reveals_lower_layers_without_paint_or_hdr() {
    let mut group = node(
        "group",
        VisualSource::Group {
            sequence: SequenceId::new("s").unwrap(),
            mapping: MappedTime {
                sample: TimeSample::after(t(0)),
                rate: ExactRatio::integer(1),
            },
            layers: vec![node("gap", visual(binding(), TimeSample::after(t(1))))],
        },
    );
    group.effects.push(VisualEffect::Opacity {
        factor: ExactRatio::new(1.into(), 2.into(), 128).unwrap(),
    });
    let mut source = Source(ResolvedMedia {
        binding: binding(),
        visual: ResolvedVisual::VideoGap {
            interval: ExactInterval {
                start: ExactRatio::integer(0),
                end: ExactRatio::integer(2),
            },
        },
    });
    let result = render_frame(
        &plan(
            vec![
                node(
                    "blue",
                    VisualSource::Solid {
                        rgba: [0, 0, 255, 255],
                    },
                ),
                group,
            ],
            1,
            1,
        ),
        &mut source,
        Sampling::Nearest,
        Default::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(
        result.image.to_srgb8(output(), &|| false).unwrap().rgba,
        [0, 0, 255, 255]
    );
    assert!(!result.image.has_hdr());
    assert_eq!(result.stats.pixel_visits, 2); // background + completed group, no gap paint
}
#[test]
fn gaps_cannot_bypass_resource_kind_binding_interval_or_availability_validation() {
    let good = ResolvedMedia {
        binding: binding(),
        visual: ResolvedVisual::VideoGap {
            interval: ExactInterval {
                start: ExactRatio::integer(0),
                end: ExactRatio::integer(2),
            },
        },
    };
    let sample = TimeSample::after(t(1));
    let mut changed = good.clone();
    changed.binding.digest = Digest::from_sha256([2; 32]);
    assert!(render(changed, sample.clone()).is_err());
    for (start, end) in [(1, 1), (2, 1), (-1, 2), (0, 3), (0, 1)] {
        let mut bad = good.clone();
        bad.visual = ResolvedVisual::VideoGap {
            interval: ExactInterval {
                start: ExactRatio::integer(start),
                end: ExactRatio::integer(end),
            },
        };
        assert!(render(bad, sample.clone()).is_err());
    }
    assert!(render(good.clone(), TimeSample::after(t(2))).is_err());
    let mut bad = good;
    bad.visual = ResolvedVisual::Image { frame: frame() };
    assert!(render(bad, sample).is_err());
}
