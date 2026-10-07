mod support;
use mo_common::{Digest, ResourceId};
use mo_time::ExactRatio;
use mo_video_compile::{MediaBinding, VisualEffect, VisualSource};
use mo_video_model::*;
use mo_video_render::*;
use std::{cell::Cell, sync::Arc};
use support::*;
fn opacity(n: i64, d: i64) -> VisualEffect {
    VisualEffect::Opacity {
        factor: ExactRatio::integer(n)
            .div(&ExactRatio::integer(d), 256)
            .unwrap(),
    }
}
#[test]
fn nested_group_opacity_is_applied_after_children_in_linear_light() {
    let red = node(
        "red",
        VisualSource::Solid {
            rgba: [255, 0, 0, 255],
        },
    );
    let blue = node(
        "blue",
        VisualSource::Solid {
            rgba: [0, 0, 255, 255],
        },
    );
    let mut group = node(
        "group",
        VisualSource::Group {
            sequence: SequenceId::new("nested").unwrap(),
            mapping: MappedTime {
                sample: TimeSample::after(t(0)),
                rate: ExactRatio::integer(1),
            },
            layers: vec![red, blue],
        },
    );
    group.effects.push(opacity(1, 2));
    let p = plan(
        vec![
            node("background", VisualSource::Solid { rgba: [255; 4] }),
            group,
        ],
        1,
        1,
    );
    let rendered = render_frame(
        &p,
        &mut NoMedia,
        Sampling::Bilinear,
        RenderLimits::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(
        rendered.image.to_srgb8(output(), &|| false).unwrap().rgba,
        vec![188, 188, 255, 255]
    );
    assert_eq!(rendered.stats.peak_temporary_bytes, 32);
    assert_eq!(rendered.stats.pixel_visits, 4);
}
struct ImageSource {
    image: Arc<LinearImage>,
    geometry: SourceGeometry,
    binding: MediaBinding,
}
impl VisualResources for ImageSource {
    fn media(
        &mut self,
        _: &MediaBinding,
        _: &TimeSample,
        _: &dyn Fn() -> bool,
    ) -> Result<ResolvedMedia> {
        Ok(ResolvedMedia {
            binding: self.binding.clone(),
            visual: ResolvedVisual::Image {
                frame: VisualFrame {
                    image: self.image.clone(),
                    geometry: self.geometry,
                },
            },
        })
    }
}
fn source() -> (ImageSource, VisualSource) {
    let img = normalize_frame(
        &rgba(&[[255, 0, 0, 255], [0, 255, 0, 255]]),
        srgb(),
        PixelLimits::default(),
        &|| false,
    )
    .unwrap()
    .image;
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
    let visual = VisualSource::Media {
        binding: binding.clone(),
        mapping: MappedTime {
            sample: TimeSample::after(t(0)),
            rate: ExactRatio::integer(1),
        },
    };
    (
        ImageSource {
            image: Arc::new(img),
            geometry: SourceGeometry::square_pixels(),
            binding,
        },
        visual,
    )
}
#[test]
fn display_rotation_and_fit_preserve_source_pixels_and_alpha() {
    let (mut resources, visual) = source();
    resources.geometry.pixel_to_display = Affine::new([0., 1., -1., 0., 0., 0.]).unwrap();
    let p = plan(vec![node("image", visual)], 1, 2);
    let result = render_frame(
        &p,
        &mut resources,
        Sampling::Nearest,
        RenderLimits::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(
        result.image.to_srgb8(output(), &|| false).unwrap().rgba,
        vec![255, 0, 0, 255, 0, 255, 0, 255]
    );
    let placement = SourceGeometry {
        pixel_to_display: Affine::new([2., 0., 0., 1., 0., 0.]).unwrap(),
    }
    .contain((2, 1), (4, 4))
    .unwrap();
    assert_eq!(placement.map(0., 0.), (0., 1.5));
    assert_eq!(placement.map(2., 1.), (4., 2.5));
}
#[test]
fn bilinear_interpolation_uses_linear_premultiplied_values_and_clamped_edges() {
    let raw = rgba(&[[255, 0, 0, 255], [0, 255, 0, 0]]);
    let img = normalize_frame(&raw, srgb(), PixelLimits::default(), &|| false)
        .unwrap()
        .image;
    let (mut resources, visual) = source();
    resources.image = Arc::new(img);
    let p = plan(vec![node("image", visual)], 4, 2);
    let result = render_frame(
        &p,
        &mut resources,
        Sampling::Bilinear,
        RenderLimits::default(),
        &|| false,
    )
    .unwrap();
    let row = &result.image.to_srgb8(output(), &|| false).unwrap().rgba[..16];
    assert_eq!(
        row,
        [255, 0, 0, 255, 255, 0, 0, 191, 255, 0, 0, 64, 0, 0, 0, 0]
    );
}
#[test]
fn changed_binding_unknown_capability_and_budgets_fail_explicitly() {
    let (mut resources, visual) = source();
    resources.binding.digest = Digest::from_sha256([2; 32]);
    assert!(matches!(
        render_frame(
            &plan(vec![node("a", visual)], 2, 1),
            &mut resources,
            Sampling::Bilinear,
            RenderLimits::default(),
            &|| false
        ),
        Err(RenderError::Invalid(_))
    ));
    let p = plan(
        vec![node("a", VisualSource::Solid { rgba: [255; 4] })],
        4,
        4,
    );
    let l = RenderLimits {
        max_pixel_visits: 15,
        ..Default::default()
    };
    assert!(matches!(
        render_frame(&p, &mut NoMedia, Sampling::Bilinear, l, &|| false),
        Err(RenderError::Budget)
    ));
    let l = RenderLimits {
        max_temporary_bytes: 255,
        ..Default::default()
    };
    assert!(matches!(
        render_frame(&p, &mut NoMedia, Sampling::Bilinear, l, &|| false),
        Err(RenderError::Budget)
    ));
}
#[test]
fn render_cancellation_at_every_checkpoint_never_returns_partial_pixels() {
    let count = Cell::new(0);
    let p = plan(
        vec![node("a", VisualSource::Solid { rgba: [255; 4] })],
        4,
        4,
    );
    render_frame(
        &p,
        &mut NoMedia,
        Sampling::Bilinear,
        RenderLimits::default(),
        &|| {
            count.set(count.get() + 1);
            false
        },
    )
    .unwrap();
    for at in 1..=count.get() {
        let seen = Cell::new(0);
        assert!(matches!(
            render_frame(
                &p,
                &mut NoMedia,
                Sampling::Bilinear,
                RenderLimits::default(),
                &|| {
                    seen.set(seen.get() + 1);
                    seen.get() == at
                }
            ),
            Err(RenderError::Cancelled)
        ));
    }
}
#[test]
fn source_metadata_does_not_silently_drop_rotation_or_pixel_aspect() {
    let f = rgba(&[[0; 4]]);
    let stream = mo_media::StreamInfo {
        index: 0,
        container_track_id: None,
        kind: mo_media::MediaKind::Video,
        codec: "rawvideo".into(),
        time_base: None,
        start_ticks: None,
        duration_ticks: None,
        width: Some(1),
        height: Some(1),
        sample_rate: None,
        channels: None,
        nominal_frame_rate: None,
        sample_aspect_ratio: None,
        colors: f.info.colors,
        channel_layout: String::new(),
        display_matrix: Some([0, 65536, 0, -65536, 0, 0, 0, 0, 1073741824]),
    };
    let geo = SourceGeometry::from_media(&f.info, &stream).unwrap();
    assert_eq!(geo.pixel_to_display.map(1., 0.), (0., 1.));
    let mut bad = stream;
    bad.display_matrix.as_mut().unwrap()[2] = 1;
    assert!(SourceGeometry::from_media(&f.info, &bad).is_err());
}
