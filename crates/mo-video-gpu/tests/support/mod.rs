use mo_common::{Digest, RationalTime, ResourceId, Ticks};
use mo_time::ExactRatio;
use mo_video_compile::*;
use mo_video_model::*;
use mo_video_render::*;
use std::sync::Arc;
pub struct Resources {
    pub binding: MediaBinding,
    pub image: Arc<LinearImage>,
}
impl VisualResources for Resources {
    fn media(
        &mut self,
        _: &MediaBinding,
        _: &TimeSample,
        _: &dyn Fn() -> bool,
    ) -> mo_video_render::Result<ResolvedMedia> {
        Ok(ResolvedMedia {
            binding: self.binding.clone(),
            visual: ResolvedVisual::Image {
                frame: VisualFrame {
                    image: self.image.clone(),
                    geometry: SourceGeometry::square_pixels(),
                },
            },
        })
    }
}
pub fn sample() -> TimeSample {
    TimeSample::after(RationalTime::new(0, 1).unwrap())
}
pub fn node(id: &str, source: VisualSource) -> VisualNode {
    VisualNode {
        track: TrackId::new("v1").unwrap(),
        clip: ClipId::new(id).unwrap(),
        local: sample(),
        effects: vec![],
        source,
    }
}
pub fn solid(id: &str, rgba: [u8; 4]) -> VisualNode {
    node(id, VisualSource::Solid { rgba })
}
pub fn transform(rotation: i32, scale: i32) -> VisualEffect {
    VisualEffect::Transform {
        x_milli_pixels: 1130,
        y_milli_pixels: -2270,
        scale_x_ppm: scale,
        scale_y_ppm: 730_000,
        rotation_milli_degrees: rotation,
        opacity_ppm: 850_000,
    }
}
pub fn opacity() -> VisualEffect {
    VisualEffect::Opacity {
        factor: ExactRatio::integer(1)
            .div(&ExactRatio::integer(2), 128)
            .unwrap(),
    }
}
pub fn group(layers: Vec<VisualNode>) -> VisualNode {
    let mut group = node(
        "group",
        VisualSource::Group {
            sequence: SequenceId::new("child").unwrap(),
            mapping: MappedTime {
                sample: sample(),
                rate: ExactRatio::integer(1),
            },
            layers,
        },
    );
    group.effects.push(opacity());
    group
}
pub fn plan(layers: Vec<VisualNode>, width: u32, height: u32) -> VideoFramePlan {
    VideoFramePlan {
        document_digest: Digest::from_sha256([1; 32]),
        frame_index: Ticks::new(0),
        sample: sample(),
        canvas: Canvas { width, height },
        layers,
    }
}
pub fn source(image: LinearImage) -> (Resources, VisualNode) {
    let binding = MediaBinding {
        resource: ResourceId::new("image").unwrap(),
        digest: Digest::from_sha256([2; 32]),
        stream: StreamId::new("v").unwrap(),
        descriptor: MediaStream::Image {
            dimensions: Canvas {
                width: image.width(),
                height: image.height(),
            },
        },
    };
    let n = node(
        "source",
        VisualSource::Media {
            binding: binding.clone(),
            mapping: MappedTime {
                sample: sample(),
                rate: ExactRatio::integer(1),
            },
        },
    );
    (
        Resources {
            binding,
            image: Arc::new(image),
        },
        n,
    )
}
pub fn image() -> LinearImage {
    let pixels = (0..17)
        .flat_map(|y| {
            (0..33).map(move |x| {
                let a = ((x * 7 + y * 3) % 17) as f32 / 16.;
                [
                    ((x * 17 + y * 13) % 257) as f32 / 256. * a,
                    ((x * 11 + y * 7) % 97) as f32 / 96. * a,
                    ((x * 23 + y * 5) % 131) as f32 / 130. * a,
                    a,
                ]
            })
        })
        .collect();
    LinearImage::from_pixels(33, 17, pixels, false, PixelLimits::default(), &|| false).unwrap()
}
pub fn cases() -> Vec<(String, VideoFramePlan)> {
    let (_, media) = source(image());
    let mut out = vec![
        ("empty".into(), plan(vec![], 31, 17)),
        (
            "group-opacity".into(),
            plan(
                vec![
                    solid("white", [255; 4]),
                    group(vec![
                        solid("red", [255, 0, 0, 255]),
                        solid("blue", [0, 0, 255, 255]),
                    ]),
                ],
                16,
                8,
            ),
        ),
        ("alpha-image".into(), plan(vec![media.clone()], 96, 64)),
    ];
    for (i, angle) in [0, 15000, 23000, 90000, 180000, 270000, -47000]
        .into_iter()
        .enumerate()
    {
        let mut image = media.clone();
        image.effects.push(transform(
            angle,
            if i % 2 == 0 { 750_000 } else { -850_000 },
        ));
        let mut nested = group(vec![solid("solid", [255, 30, 90, 128]), image]);
        nested.effects.push(transform(angle / 3, 910_000));
        out.push((
            format!("transform-{angle}"),
            plan(vec![solid("background", [15, 23, 42, 255]), nested], 96, 64),
        ));
    }
    out.push(("maximum-width".into(), plan(vec![media.clone()], 16384, 7)));
    let mut hd = media;
    hd.effects.push(transform(23000, 850_000));
    out.push((
        "hd-composition".into(),
        plan(
            vec![solid("background", [15, 23, 42, 255]), group(vec![hd])],
            1920,
            1080,
        ),
    ));
    out
}
