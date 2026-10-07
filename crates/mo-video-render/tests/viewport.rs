mod support;
use mo_common::{Digest, ResourceId};
use mo_time::ExactRatio;
use mo_video_compile::{MediaBinding, VisualEffect, VisualSource};
use mo_video_model::*;
use mo_video_render::*;
use std::{cell::Cell, sync::Arc};
use support::*;

fn transform(x: i32, y: i32, scale: i32, angle: i32) -> VisualEffect {
    VisualEffect::Transform {
        x_milli_pixels: x,
        y_milli_pixels: y,
        scale_x_ppm: scale,
        scale_y_ppm: scale,
        rotation_milli_degrees: angle,
        opacity_ppm: 1_000_000,
    }
}
fn group(id: &str, layers: Vec<mo_video_compile::VisualNode>) -> mo_video_compile::VisualNode {
    node(
        id,
        VisualSource::Group {
            sequence: SequenceId::new(format!("sequence-{id}")).unwrap(),
            mapping: MappedTime {
                sample: TimeSample::after(t(0)),
                rate: ExactRatio::integer(1),
            },
            layers,
        },
    )
}
fn options(width: u32, height: u32) -> FrameRenderOptions {
    FrameRenderOptions {
        raster: RasterExtent { width, height },
        sampling: Sampling::Nearest,
        limits: RenderLimits::default(),
    }
}
fn pixels(plan: &mo_video_compile::VideoFramePlan, width: u32, height: u32) -> SrgbImage {
    render_frame_with_options(plan, &mut NoMedia, options(width, height), &|| false)
        .unwrap()
        .image
        .to_srgb8(output(), &|| false)
        .unwrap()
}

#[test]
fn project_translation_group_clip_and_opacity_survive_preview_scaling() {
    // In the 120x80 project the half-size red child covers [80,140)x[20,60).
    // Its group first clips it to x<120, then moves it left by 40. Expected
    // visible range is [40,80)x[20,60), not [40,100) or a raster-space offset.
    let mut red = node(
        "red",
        VisualSource::Solid {
            rgba: [255, 0, 0, 255],
        },
    );
    red.effects.push(transform(50_000, 0, 500_000, 0));
    let mut g = group("g", vec![red]);
    g.effects.push(transform(-40_000, 0, 1_000_000, 0));
    g.effects.push(VisualEffect::Opacity {
        factor: ExactRatio::integer(1)
            .div(&ExactRatio::integer(2), 256)
            .unwrap(),
    });
    let p = plan(
        vec![
            node(
                "blue",
                VisualSource::Solid {
                    rgba: [0, 0, 255, 255],
                },
            ),
            g,
        ],
        120,
        80,
    );
    let original = serde_json::to_vec(&p).unwrap();
    for (width, height) in [(12, 8), (24, 16), (12, 16), (120, 80)] {
        let out = pixels(&p, width, height);
        assert_eq!((out.width, out.height), (width, height));
        for y in 0..height {
            for x in 0..width {
                let px = (2 * x + 1) * 120;
                let py = (2 * y + 1) * 80;
                let inside =
                    px >= 80 * width && px < 160 * width && py >= 40 * height && py < 120 * height;
                let at = ((y * width + x) * 4) as usize;
                assert_eq!(
                    &out.rgba[at..at + 4],
                    if inside {
                        &[188, 0, 188, 255]
                    } else {
                        &[0, 0, 255, 255]
                    },
                    "{width}x{height} at {x},{y}"
                );
            }
        }
    }
    assert_eq!(serde_json::to_vec(&p).unwrap(), original);
}

#[test]
fn nested_rotation_uses_project_pivot_in_an_anisotropic_grid() {
    // [60,120)x[20,60) rotated +90 about (60,40) => [40,80)x[40,100),
    // then clipped to y<80. The outer group translates by (-20,-20).
    let mut child = node(
        "red",
        VisualSource::Solid {
            rgba: [255, 0, 0, 255],
        },
    );
    child.effects.push(transform(30_000, 0, 500_000, 0));
    let mut inner = group("inner", vec![child]);
    inner.effects.push(transform(0, 0, 1_000_000, 90_000));
    let mut outer = group("outer", vec![inner]);
    outer
        .effects
        .push(transform(-20_000, -20_000, 1_000_000, 0));
    let p = plan(vec![outer], 120, 80);
    for (width, height) in [(12, 8), (12, 16), (24, 8)] {
        let out = pixels(&p, width, height);
        for y in 0..height {
            for x in 0..width {
                let px = (2 * x + 1) * 120;
                let py = (2 * y + 1) * 80;
                let inside =
                    px >= 40 * width && px < 120 * width && py >= 40 * height && py < 120 * height;
                let at = ((y * width + x) * 4) as usize;
                assert_eq!(
                    &out.rgba[at..at + 4],
                    if inside { &[255, 0, 0, 255] } else { &[0; 4] },
                    "{width}x{height} at {x},{y}"
                );
            }
        }
    }
}

struct ImageSource {
    binding: MediaBinding,
    frame: VisualFrame,
    expected_time: TimeSample,
    calls: usize,
}
impl VisualResources for ImageSource {
    fn media(
        &mut self,
        binding: &MediaBinding,
        time: &TimeSample,
        _: &dyn Fn() -> bool,
    ) -> Result<ResolvedMedia> {
        assert_eq!(binding, &self.binding);
        assert_eq!(time, &self.expected_time);
        self.calls += 1;
        Ok(ResolvedMedia {
            binding: binding.clone(),
            visual: ResolvedVisual::Image {
                frame: self.frame.clone(),
            },
        })
    }
}
#[test]
fn media_rotation_fit_and_exact_time_are_independent_of_raster_extent() {
    let binding = MediaBinding {
        resource: ResourceId::new("image").unwrap(),
        digest: Digest::from_sha256([1; 32]),
        stream: StreamId::new("image").unwrap(),
        descriptor: MediaStream::Image {
            dimensions: Canvas {
                width: 2,
                height: 1,
            },
        },
    };
    let sample = TimeSample {
        time: ExactRatio::from_time(mo_common::RationalTime::new(1001, 30000).unwrap()),
        side: mo_time::SampleSide::Before,
    };
    let image = normalize_frame(
        &rgba(&[[255, 0, 0, 255], [0, 255, 0, 255]]),
        srgb(),
        PixelLimits::default(),
        &|| false,
    )
    .unwrap()
    .image;
    let mut resources = ImageSource {
        binding: binding.clone(),
        frame: VisualFrame {
            image: Arc::new(image),
            geometry: SourceGeometry {
                pixel_to_display: Affine::new([0., 1., -1., 0., 0., 0.]).unwrap(),
            },
        },
        expected_time: sample.clone(),
        calls: 0,
    };
    let mut visual = node(
        "source",
        VisualSource::Media {
            binding,
            mapping: MappedTime {
                sample,
                rate: ExactRatio::integer(-1),
            },
        },
    );
    visual.effects.push(transform(20_000, 0, 1_000_000, 0));
    let p = plan(vec![visual], 120, 80);
    let rendered = render_frame_with_options(&p, &mut resources, options(6, 4), &|| false).unwrap();
    let out = rendered.image.to_srgb8(output(), &|| false).unwrap();
    for y in 0..4 {
        for x in 0..6 {
            let expected = if (3..5).contains(&x) {
                if y < 2 {
                    [255, 0, 0, 255]
                } else {
                    [0, 255, 0, 255]
                }
            } else {
                [0; 4]
            };
            let at = (y * 6 + x) * 4;
            assert_eq!(&out.rgba[at..at + 4], &expected);
        }
    }
    assert_eq!(resources.calls, 1);
}

#[test]
fn preview_allocations_and_work_are_charged_to_raster_not_project_extent() {
    let p = plan(
        vec![group(
            "g",
            vec![node("white", VisualSource::Solid { rgba: [255; 4] })],
        )],
        16384,
        16384,
    );
    let mut o = options(16, 16);
    o.limits.pixels = PixelLimits {
        max_pixels: 256,
        max_bytes: 4096,
    };
    o.limits.max_pixel_visits = 512;
    o.limits.max_temporary_bytes = 8192;
    let result = render_frame_with_options(&p, &mut NoMedia, o, &|| false).unwrap();
    assert_eq!(result.stats.pixel_visits, 512);
    assert_eq!(result.stats.peak_temporary_bytes, 8192);
    assert!(
        result
            .image
            .to_srgb8(output(), &|| false)
            .unwrap()
            .rgba
            .iter()
            .all(|b| *b == 255)
    );
    assert!(matches!(
        render_frame(&p, &mut NoMedia, o.sampling, o.limits, &|| false),
        Err(RenderError::Budget)
    ));
    o.limits.max_temporary_bytes -= 1;
    assert!(matches!(
        render_frame_with_options(&p, &mut NoMedia, o, &|| false),
        Err(RenderError::Budget)
    ));
    o.limits.max_temporary_bytes += 1;
    o.limits.max_pixel_visits -= 1;
    assert!(matches!(
        render_frame_with_options(&p, &mut NoMedia, o, &|| false),
        Err(RenderError::Budget)
    ));
}

#[test]
fn invalid_extent_is_rejected_and_each_cancellation_discards_candidate_pixels() {
    let p = plan(
        vec![node("white", VisualSource::Solid { rgba: [255; 4] })],
        120,
        80,
    );
    for (w, h) in [(0, 8), (12, 0), (16385, 8), (12, u32::MAX)] {
        assert!(matches!(
            render_frame_with_options(&p, &mut NoMedia, options(w, h), &|| false),
            Err(RenderError::Invalid(_))
        ));
    }
    let mut invalid = p.clone();
    invalid.canvas.width = 0;
    assert!(matches!(
        render_frame_with_options(&invalid, &mut NoMedia, options(12, 8), &|| false),
        Err(RenderError::Invalid(_))
    ));
    let calls = Cell::new(0);
    let count = || {
        calls.set(calls.get() + 1);
        false
    };
    render_frame_with_options(&p, &mut NoMedia, options(12, 8), &count).unwrap();
    for point in 1..=calls.get() {
        let seen = Cell::new(0);
        assert!(matches!(
            render_frame_with_options(&p, &mut NoMedia, options(12, 8), &|| {
                seen.set(seen.get() + 1);
                seen.get() == point
            }),
            Err(RenderError::Cancelled)
        ));
    }
}
