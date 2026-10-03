//! Decode demands are resolved for every use before allocating any page pixels.
#[allow(dead_code)]
#[path = "../../../tools/test-support/source_playback.rs"]
mod support;
use mo_image::{DecodeSize, DecoderReply, ImageDecoder, ImageError};
use mo_opc::{Package, PackageLimits};
use mo_presentation_compile::{source_page::SourcePageError, source_resource_page::*};
use support::*;
struct Decoder {
    demands: Vec<Option<DecodeSize>>,
    original: u32,
}
impl Default for Decoder {
    fn default() -> Self {
        Self {
            demands: vec![],
            original: 4096,
        }
    }
}
impl Decoder {
    fn reply(&mut self, size: Option<DecodeSize>) -> DecoderReply {
        self.demands.push(size);
        let s = size.unwrap_or(DecodeSize {
            width: self.original,
            height: self.original,
        });
        let length = s.width * s.height * 4;
        DecoderReply {
            status: 0,
            words: [
                s.width,
                s.height,
                self.original,
                self.original,
                1,
                1,
                8,
                0,
                length,
            ],
            pixels: vec![0; length as usize],
        }
    }
}
impl ImageDecoder for Decoder {
    fn decode(&mut self, _: &[u8]) -> Result<DecoderReply, ImageError> {
        Ok(self.reply(None))
    }
    fn decode_sized(&mut self, _: &[u8], size: DecodeSize) -> Result<DecoderReply, ImageError> {
        Ok(self.reply(Some(size)))
    }
    fn invalidate(&mut self) {}
}
fn plan(
    bytes: &[u8],
    d: &mut Decoder,
    nearest: bool,
) -> Result<SourceResourcePagePlan, SourcePageError> {
    let package = Package::open(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })
    .unwrap();
    let index = read(bytes);
    let mut options = options();
    if !nearest {
        options.sampling = mo_raster::ImageSampling::Linear;
    }
    prepare(
        &package,
        &index,
        &request(&index),
        d,
        None,
        options,
        &|| false,
    )?
    .plan(&|| false)
}
fn image(id: u32, resource: &str, crop: &str) -> String {
    picture(
        id,
        "",
        &blip(resource, "", &(crop.to_owned() + STRETCH)),
        "",
    )
}
#[test]
fn multiple_4k_resources_fit_without_changing_source_identity_or_page_geometry() {
    let bytes = image_fixture(&(image(42, "owned-image", "") + &image(43, "owned-cyan", "")));
    let mut d = Decoder::default();
    let p = plan(&bytes, &mut d, false).unwrap();
    assert_eq!(
        d.demands,
        vec![
            Some(DecodeSize {
                width: 400,
                height: 400
            });
            2
        ]
    );
    assert_eq!(p.image_work.resource_bytes, 2 * 400 * 400 * 4);
    assert_eq!(p.images.decoded.len(), 2);
    assert!(
        p.images
            .decoded
            .iter()
            .all(|i| i.encoded_width == 4096 && i.width == 400)
    );
    // The exact-grid mode still observes the original aggregate safety budget.
    assert!(plan(&bytes, &mut Decoder::default(), true).is_err());
}
#[test]
fn shared_resource_uses_largest_cropped_footprint_before_single_decode() {
    let bytes = image_fixture(
        &(image(42, "owned-image", "")
            + &image(43, "owned-copy", "<a:srcRect l=\"25000\" r=\"25000\"/>")),
    );
    let mut d = Decoder::default();
    let p = plan(&bytes, &mut d, false).unwrap();
    assert_eq!(
        d.demands,
        vec![Some(DecodeSize {
            width: 800,
            height: 400
        })]
    );
    assert_eq!(p.images.bindings.len(), 2);
    assert_eq!(p.images.decoded.len(), 1);
    assert_eq!(p.images.gather_copy_bytes, 0);
}

fn playback(
    bytes: &[u8],
    decoder: &mut Decoder,
) -> mo_presentation_compile::source_playback::RetainedSourcePlaybackPlan {
    let package = Package::open(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })
    .unwrap();
    let index = read(bytes);
    let binding = mo_timeline::PlaybackBinding {
        session: mo_common::PlaybackSessionId::new("sampled-images").unwrap(),
        generation: mo_timeline::PlaybackGeneration::new(1),
        revision: index.source_sha256.clone(),
    };
    let plan = mo_presentation_compile::source_playback::SourcePlaybackPlan::new(
        &package,
        &index,
        request(&index),
        binding,
        mo_pptx::source::SourceLimits::default(),
        mo_timeline::TimelineLimits::default(),
        &|| false,
    )
    .unwrap();
    let mut options = options();
    options.sampling = mo_raster::ImageSampling::Linear;
    plan.retain(&package, index, decoder, None, options, &|| false)
        .unwrap()
}
#[derive(Default)]
struct Capture(Vec<u32>);
impl mo_raster::RasterBackend for Capture {
    fn raster(&mut self, _: &[u32]) -> Result<mo_raster::BackendReply, mo_raster::RasterError> {
        panic!("image backend required")
    }
    fn raster_images(
        &mut self,
        words: &[u32],
        _: &[u8],
    ) -> Result<mo_raster::BackendReply, mo_raster::RasterError> {
        self.0 = words.to_vec();
        Err(mo_raster::RasterError::Host("captured raster commands"))
    }
    fn invalidate(&mut self) {}
}
#[test]
fn retained_static_frames_preserve_sampled_image_brushes_and_decode_only_once() {
    let bytes = image_fixture(
        &(image(42, "owned-image", "<a:srcRect l=\"25000\"/>") + &image(43, "owned-cyan", "")),
    );
    let mut decoder = Decoder::default();
    let mut retained = playback(&bytes, &mut decoder);
    assert!(decoder.demands.iter().all(Option::is_some));
    assert!(retained.preparation().decoded_pixel_bytes < 2 * 1024 * 1024);
    let package = Package::open(
        bytes.as_slice(),
        bytes.len() as u64,
        PackageLimits::default(),
        &|| false,
    )
    .unwrap();
    let index = read(&bytes);
    let mut options = options();
    options.sampling = mo_raster::ImageSampling::Linear;
    let mut expected = Capture::default();
    assert!(
        prepare(
            &package,
            &index,
            &request(&index),
            &mut Decoder::default(),
            None,
            options,
            &|| false
        )
        .unwrap()
        .render(&mut expected, &|| false)
        .is_err()
    );
    assert!(!expected.0.is_empty());
    for second in [0, 2, 1] {
        let mut actual = Capture::default();
        let (_, frame) = retained
            .prepare_frame(
                mo_common::RationalTime::new(second, 1).unwrap(),
                None,
                &|| false,
            )
            .unwrap();
        assert!(frame.render(&mut actual, &|| false).is_err());
        assert_eq!(
            actual.0, expected.0,
            "retained brushes must address the sampled grid"
        );
    }
    assert_eq!(decoder.demands.len(), 2);
}
#[test]
fn animated_geometry_protects_descendants_and_shared_images_but_not_static_resources() {
    let frame = transform("", [0, 0, 800000, 800000], [0, 0, 800000, 800000]);
    let bytes = image_fixture(
        &(group(90, &frame, "", &image(42, "owned-image", ""))
            + &image(43, "owned-copy", "")
            + &image(44, "owned-cyan", "")),
    );
    for bytes in [
        scaled(&bytes, 42, [100000, 100000], [400000, 200000], "freeze"),
        scaled(&bytes, 90, [100000, 100000], [400000, 200000], "freeze"),
        animated(&bytes, 90, 0, 21600000, "freeze"),
    ] {
        let mut decoder = Decoder {
            original: 1024,
            ..Default::default()
        };
        let mut retained = playback(&bytes, &mut decoder);
        assert_eq!(
            decoder.demands,
            vec![
                None,
                Some(DecodeSize {
                    width: 400,
                    height: 400
                })
            ]
        );
        for second in [0, 1] {
            retained
                .prepare_frame(
                    mo_common::RationalTime::new(second, 1).unwrap(),
                    None,
                    &|| false,
                )
                .unwrap();
        }
        assert_eq!(decoder.demands.len(), 2);
    }
}

#[test]
fn retained_resize_readmits_larger_image_grids_and_abort_keeps_previous_resources() {
    let bytes = image_fixture(&(image(42, "owned-image", "") + &image(43, "owned-copy", "")));
    let package = Package::open(
        bytes.as_slice(),
        bytes.len() as u64,
        PackageLimits::default(),
        &|| false,
    )
    .unwrap();
    let index = read(&bytes);
    let mut decoder = Decoder::default();
    let mut retained = playback(&bytes, &mut decoder);
    let previous = serde_json::to_value(retained.preparation()).unwrap();
    let mut viewport = request(&index).viewport;
    viewport.width *= 2;
    viewport.height *= 2;
    viewport.scale.numerator *= 2;
    let candidate = retained
        .prepare_resize(&package, viewport.clone(), &mut decoder, &|| false)
        .unwrap();
    assert_eq!(candidate.preparation().decoded_pixel_bytes, 800 * 800 * 4);
    drop(candidate);
    assert_eq!(
        serde_json::to_value(retained.preparation()).unwrap(),
        previous
    );
    let candidate = retained
        .prepare_resize(&package, viewport, &mut decoder, &|| false)
        .unwrap();
    candidate.commit();
    assert_eq!(retained.preparation().decoded_pixel_bytes, 800 * 800 * 4);
    assert_eq!(
        decoder.demands,
        vec![
            Some(DecodeSize {
                width: 400,
                height: 400
            }),
            Some(DecodeSize {
                width: 800,
                height: 800
            }),
            Some(DecodeSize {
                width: 800,
                height: 800
            })
        ]
    );
    // A distinct package cannot supply replacement images to this source owner.
    let other = image_fixture(&image(99, "owned-cyan", ""));
    let other = Package::open(
        other.as_slice(),
        other.len() as u64,
        PackageLimits::default(),
        &|| false,
    )
    .unwrap();
    let before = serde_json::to_value(retained.preparation()).unwrap();
    assert!(
        retained
            .prepare_resize(&other, request(&index).viewport, &mut decoder, &|| false)
            .is_err()
    );
    assert_eq!(decoder.demands.len(), 3);
    assert_eq!(
        serde_json::to_value(retained.preparation()).unwrap(),
        before
    );
}

#[test]
fn failed_resize_decoder_keeps_live_pixels_and_diagnostics() {
    struct Fail;
    impl ImageDecoder for Fail {
        fn decode(&mut self, _: &[u8]) -> Result<DecoderReply, ImageError> {
            Err(ImageError::Host("injected resize decode failure"))
        }
        fn decode_sized(
            &mut self,
            bytes: &[u8],
            _: DecodeSize,
        ) -> Result<DecoderReply, ImageError> {
            self.decode(bytes)
        }
        fn invalidate(&mut self) {}
    }
    let bytes = image_fixture(&image(42, "owned-image", ""));
    let package = Package::open(
        bytes.as_slice(),
        bytes.len() as u64,
        PackageLimits::default(),
        &|| false,
    )
    .unwrap();
    let mut retained = playback(&bytes, &mut Decoder::default());
    let before = serde_json::to_value(retained.preparation()).unwrap();
    let mut viewport = request(&read(&bytes)).viewport;
    viewport.width *= 2;
    viewport.height *= 2;
    viewport.scale.numerator *= 2;
    assert!(
        retained
            .prepare_resize(&package, viewport, &mut Fail, &|| false)
            .is_err()
    );
    assert_eq!(
        serde_json::to_value(retained.preparation()).unwrap(),
        before
    );
    retained
        .prepare_frame(mo_common::RationalTime::new(0, 1).unwrap(), None, &|| false)
        .unwrap();
}
