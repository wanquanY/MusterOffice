use super::*;
use mo_common::Digest;
use mo_geometry::{Fixed, PathCommand, Point};
use sha2::{Digest as _, Sha256};
use std::cell::Cell;
fn p(x: i128, y: i128) -> Point {
    Point {
        x: Fixed::from_raw(x << 32),
        y: Fixed::from_raw(y << 32),
    }
}
fn manifest(bytes: &[u8]) -> Vec<ImageResource> {
    vec![ImageResource {
        width: 2,
        height: 2,
        alpha: ImageAlpha::Premultiplied,
        sha256: Digest::from_sha256(Sha256::digest(bytes).into()),
    }]
}
#[test]
fn owned_pixels_move_once_and_keep_the_borrowed_validation_contract() {
    let bytes = vec![17; 16];
    let address = bytes.as_ptr();
    let m = manifest(&bytes);
    let expected = PreparedImages::new(&m, &bytes, &|| false)
        .unwrap()
        .sha256()
        .clone();
    let owned = PreparedImages::owned(&m, bytes, &|| false).unwrap();
    assert_eq!(owned.bytes().as_ptr(), address);
    assert_eq!(owned.sha256(), &expected);
    assert_eq!(owned.len(), 1);
    assert!(matches!(
        PreparedImages::owned(&m, vec![18; 16], &|| false),
        Err(RasterError::Invalid("image resource digest"))
    ));
    assert!(matches!(
        PreparedImages::owned(&m, vec![17; 16], &|| true),
        Err(RasterError::Cancelled)
    ));
}
fn request() -> PathRasterRequest {
    PathRasterRequest {
        opacity_groups: vec![],
        clips: vec![],
        viewport: RasterViewport {
            width: 8,
            height: 8,
            origin: p(0, 0),
            scale: PixelScale {
                numerator: 1,
                denominator: 1,
            },
            coordinate_tolerance: Fixed::from_raw(1 << 24),
            background: [0; 4],
        },
        paths: vec![FillPath {
            fill_rule: FillRule::Nonzero,
            commands: vec![
                PathCommand::Move { to: p(0, 0) },
                PathCommand::Line { to: p(8, 0) },
                PathCommand::Line { to: p(8, 8) },
                PathCommand::Line { to: p(0, 8) },
                PathCommand::Close,
            ],
        }],
        draws: vec![PathDraw {
            blend: Default::default(),
            clip: None,
            path: 0,
            origin: p(0, 0),
            stroke: None,
            brush: Brush::Image {
                image: ImageBrush {
                    uncertainty: None,
                    source_domain: None,
                    resource: 0,
                    origin: p(0, 0),
                    x_step: p(4, 0),
                    y_step: p(0, 4),
                    tile_x: ImageTile::Clamp,
                    tile_y: ImageTile::Clamp,
                    sampling: ImageSampling::Nearest,
                },
            },
        }],
    }
}
fn brush(q: &mut PathRasterRequest) -> &mut ImageBrush {
    let Brush::Image { image } = &mut q.draws[0].brush else {
        panic!("image")
    };
    image
}
fn domain(left: i128, top: i128, right: i128, bottom: i128) -> ImageSourceDomain {
    ImageSourceDomain {
        left: Fixed::from_raw(left),
        top: Fixed::from_raw(top),
        right: Fixed::from_raw(right),
        bottom: Fixed::from_raw(bottom),
    }
}
#[test]
fn source_domains_reuse_pixels_and_preserve_legacy_frame_for_whole_images() {
    let bytes = [255; 16];
    let images = PreparedImages::new(&manifest(&bytes), &bytes, &|| false).unwrap();
    let mut q = request();
    let old = compile_images(&q, &images, &|| false).unwrap();
    assert_eq!(old.frame()[1], 5);
    let whole = q.draws[0].clone();
    brush(&mut q).source_domain = Some(domain(-1 << 32, 1 << 30, 3 << 32, 7 << 30));
    q.draws.push(whole);
    q.draws.push(q.draws[0].clone());
    let c = compile_images(&q, &images, &|| false).unwrap();
    assert_eq!(
        (
            c.frame()[1],
            c.work().resources,
            c.work().brushes,
            c.work().draws
        ),
        (6, 1, 2, 3)
    );
    assert_eq!(c.frame().len(), old.frame().len() + 4 + 14 + 12);
    assert_eq!(c.work().coordinate_error_bound, Fixed::ZERO);
    assert!(std::ptr::eq(images.bytes().as_ptr(), bytes.as_ptr()));
}
#[test]
fn source_domain_invalid_extent_and_component_range_fail_before_backend() {
    let bytes = [255; 16];
    let images = PreparedImages::new(&manifest(&bytes), &bytes, &|| false).unwrap();
    for d in [
        domain(0, 0, 0, 1 << 32),
        domain(2 << 32, 0, 1 << 32, 1 << 32),
        domain(0, 0, 1, 1 << 32),
        domain(0, 0, 32769 << 32, 1 << 32),
        domain(-9000 << 32, 0, 1 << 32, 1 << 32),
    ] {
        let mut q = request();
        brush(&mut q).source_domain = Some(d);
        assert!(compile_images(&q, &images, &|| false).is_err());
    }
}
#[test]
fn repeated_domain_quantization_accounts_for_accumulated_period_error() {
    let bytes = [255; 16];
    let images = PreparedImages::new(&manifest(&bytes), &bytes, &|| false).unwrap();
    let mut q = request();
    q.viewport.coordinate_tolerance = Fixed::from_raw(256);
    brush(&mut q).source_domain = Some(domain((1 << 32) / 3, 0, 1 << 32, 2 << 32));
    let clamped = compile_images(&q, &images, &|| false).unwrap();
    assert!(clamped.work().coordinate_error_bound > Fixed::ZERO);
    for tile in [ImageTile::Repeat, ImageTile::Mirror] {
        brush(&mut q).tile_x = tile;
        assert!(matches!(
            compile_images(&q, &images, &|| false),
            Err(RasterError::Precision)
        ));
    }
    q.viewport.coordinate_tolerance = Fixed::from_raw(1 << 24);
    assert!(compile_images(&q, &images, &|| false).is_ok());
}
#[test]
fn repeated_affine_quantization_covers_the_viewport_not_just_one_tile() {
    let bytes = [255; 16];
    let images = PreparedImages::new(&manifest(&bytes), &bytes, &|| false).unwrap();
    let mut q = request();
    q.viewport.width = 8192;
    q.viewport.coordinate_tolerance = Fixed::from_raw(16384);
    brush(&mut q).x_step.x = Fixed::from_raw((1i128 << 32) / 3);
    // Finite clamp/decal content has no repeated, accumulating lattice.
    assert!(compile_images(&q, &images, &|| false).is_ok());
    for tile in [ImageTile::Repeat, ImageTile::Mirror] {
        brush(&mut q).tile_x = tile;
        assert!(matches!(
            compile_images(&q, &images, &|| false),
            Err(RasterError::Precision)
        ));
    }
}
fn uncertainty() -> ImageBrushUncertainty {
    ImageBrushUncertainty {
        origin: p(0, 0),
        x_step: p(0, 0),
        y_step: p(0, 0),
        source_domain: [Fixed::ZERO; 4],
    }
}
#[test]
fn input_uncertainty_is_budgeted_without_changing_pixels_or_resource_identity() {
    let bytes = [255; 16];
    let images = PreparedImages::new(&manifest(&bytes), &bytes, &|| false).unwrap();
    let mut q = request();
    let exact = compile_images(&q, &images, &|| false).unwrap();
    brush(&mut q).uncertainty = Some(Box::new(uncertainty()));
    let zero = compile_images(&q, &images, &|| false).unwrap();
    assert_eq!(exact.frame(), zero.frame());
    assert_eq!(zero.work().coordinate_error_bound, Fixed::ZERO);
    brush(&mut q).uncertainty.as_mut().unwrap().origin.x = Fixed::from_raw(1024);
    let uncertain = compile_images(&q, &images, &|| false).unwrap();
    assert_eq!(uncertain.frame(), exact.frame());
    assert_eq!(
        uncertain.work().coordinate_error_bound,
        Fixed::from_raw(1024)
    );
    // Interning identical wire parameters must not hide the less precise use.
    let mut second = q.draws[0].clone();
    let Brush::Image { image } = &mut second.brush else {
        panic!("image");
    };
    image.uncertainty.as_mut().unwrap().origin.x = Fixed::from_raw(2048);
    q.draws.push(second);
    let c = compile_images(&q, &images, &|| false).unwrap();
    assert_eq!(c.work().brushes, 1);
    assert_eq!(c.work().coordinate_error_bound, Fixed::from_raw(2048));
    let origin = p(1 << 80, -(1 << 80));
    q.viewport.origin = origin;
    for draw in &mut q.draws {
        draw.origin = origin;
        let Brush::Image { image } = &mut draw.brush else {
            panic!("image");
        };
        image.origin = origin;
    }
    let rebased = compile_images(&q, &images, &|| false).unwrap();
    assert_eq!(c.frame(), rebased.frame());
    assert_eq!(
        c.work().coordinate_error_bound,
        rebased.work().coordinate_error_bound
    );
}
#[test]
fn input_errors_follow_device_scale_and_repeated_phase() {
    let bytes = [255; 16];
    let images = PreparedImages::new(&manifest(&bytes), &bytes, &|| false).unwrap();
    let mut q = request();
    q.viewport.scale.denominator = 3;
    brush(&mut q).x_step = p(3, 0);
    brush(&mut q).y_step = p(0, 3);
    let mut e = uncertainty();
    e.origin.x = Fixed::from_raw(1000);
    brush(&mut q).uncertainty = Some(Box::new(e));
    assert_eq!(
        compile_images(&q, &images, &|| false)
            .unwrap()
            .work()
            .coordinate_error_bound,
        Fixed::from_raw(334)
    );
    q.viewport.width = 8192;
    q.viewport.coordinate_tolerance = Fixed::from_raw(16384);
    e.x_step.x = Fixed::from_raw(12);
    brush(&mut q).uncertainty = Some(Box::new(e));
    assert!(compile_images(&q, &images, &|| false).is_ok());
    brush(&mut q).tile_x = ImageTile::Repeat;
    assert!(matches!(
        compile_images(&q, &images, &|| false),
        Err(RasterError::Precision)
    ));
    e.x_step.x = Fixed::ZERO;
    e.source_domain[2] = Fixed::from_raw(8);
    brush(&mut q).uncertainty = Some(Box::new(e));
    brush(&mut q).source_domain = Some(domain(0, 0, 2 << 32, 2 << 32));
    assert!(matches!(
        compile_images(&q, &images, &|| false),
        Err(RasterError::Precision)
    ));
    q.viewport.coordinate_tolerance = Fixed::from_raw(1 << 24);
    assert!(compile_images(&q, &images, &|| false).is_ok());
}
#[test]
fn malformed_or_degenerate_input_uncertainty_is_explicit() {
    let bytes = [255; 16];
    let images = PreparedImages::new(&manifest(&bytes), &bytes, &|| false).unwrap();
    for i in 0..10 {
        let mut q = request();
        let mut e = uncertainty();
        let fields = [
            &mut e.origin.x,
            &mut e.origin.y,
            &mut e.x_step.x,
            &mut e.x_step.y,
            &mut e.y_step.x,
            &mut e.y_step.y,
        ];
        if i < 6 {
            *fields.into_iter().nth(i).unwrap() = Fixed::from_raw(-1);
        } else {
            e.source_domain[i - 6] = Fixed::from_raw(-1);
        }
        brush(&mut q).uncertainty = Some(Box::new(e));
        assert!(matches!(
            compile_images(&q, &images, &|| false),
            Err(RasterError::Invalid("negative image input uncertainty"))
        ));
    }
    let mut q = request();
    let mut e = uncertainty();
    e.source_domain[2] = Fixed::from_raw(1);
    brush(&mut q).uncertainty = Some(Box::new(e));
    assert!(matches!(
        compile_images(&q, &images, &|| false),
        Err(RasterError::Invalid(
            "image domain uncertainty requires source domain"
        ))
    ));
    brush(&mut q).source_domain = Some(domain(0, 0, 2 << 32, 2 << 32));
    e.source_domain[2] = Fixed::from_raw(2 << 32);
    brush(&mut q).uncertainty = Some(Box::new(e));
    assert!(matches!(
        compile_images(&q, &images, &|| false),
        Err(RasterError::Precision)
    ));
    e = uncertainty();
    e.x_step.x = Fixed::from_raw(4 << 32);
    brush(&mut q).uncertainty = Some(Box::new(e));
    assert!(matches!(
        compile_images(&q, &images, &|| false),
        Err(RasterError::Precision)
    ));
    e.x_step.x = Fixed::from_raw(i128::MAX);
    q.viewport.scale.numerator = 2;
    brush(&mut q).uncertainty = Some(Box::new(e));
    assert!(matches!(
        compile_images(&q, &images, &|| false),
        Err(RasterError::Range)
    ));
}
#[test]
fn domain_compile_cancellation_is_atomic_at_every_checkpoint() {
    let bytes = [255; 16];
    let images = PreparedImages::new(&manifest(&bytes), &bytes, &|| false).unwrap();
    let mut q = request();
    brush(&mut q).source_domain = Some(domain(1 << 30, 0, 7 << 30, 2 << 32));
    let total = Cell::new(0);
    compile_images(&q, &images, &|| {
        total.set(total.get() + 1);
        false
    })
    .unwrap();
    for stop in 1..=total.get() {
        let calls = Cell::new(0);
        assert!(matches!(
            compile_images(&q, &images, &|| {
                calls.set(calls.get() + 1);
                calls.get() == stop
            }),
            Err(RasterError::Cancelled)
        ));
    }
}
#[test]
fn resource_integrity_alpha_and_spans_are_validated_before_rendering() {
    let bytes = [255; 16];
    let mut m = manifest(&bytes);
    assert!(PreparedImages::new(&m, &bytes, &|| false).is_ok());
    assert!(matches!(
        PreparedImages::new(&m, &bytes[..15], &|| false),
        Err(RasterError::Invalid("image bundle length"))
    ));
    let bad = [10; 16];
    assert!(matches!(
        PreparedImages::new(&m, &bad, &|| false),
        Err(RasterError::Invalid("image resource digest"))
    ));
    let mut bad = bytes;
    bad[3] = 0;
    m[0].sha256 = Digest::from_sha256(Sha256::digest(bad).into());
    assert!(matches!(
        PreparedImages::new(&m, &bad, &|| false),
        Err(RasterError::Invalid("premultiplied image channels"))
    ));
    m[0].alpha = ImageAlpha::Straight;
    assert!(PreparedImages::new(&m, &bad, &|| false).is_ok());
    m[0].width = 8192;
    m[0].height = 8192;
    assert!(matches!(
        PreparedImages::new(&m, &[], &|| false),
        Err(RasterError::Limit(_))
    ));
    assert!(matches!(
        PreparedImages::new(&vec![m[0].clone(); 4097], &[], &|| false),
        Err(RasterError::Limit(_))
    ));
}
#[test]
fn images_and_brushes_are_reused_independently_of_local_paths() {
    let bytes = [255; 16];
    let images = PreparedImages::new(&manifest(&bytes), &bytes, &|| false).unwrap();
    let mut q = request();
    assert!(matches!(
        compile(&q, &|| false),
        Err(RasterError::Invalid("image resources required"))
    ));
    let mut second = q.draws[0].clone();
    second.origin = p(2, 3);
    q.draws.push(second);
    let c = compile_images(&q, &images, &|| false).unwrap();
    assert_eq!(
        (
            c.work().resources,
            c.work().resource_bytes,
            c.work().brushes,
            c.work().draws
        ),
        (1, 16, 1, 2)
    );
    assert_eq!((c.frame()[1], c.frame()[10], c.frame()[11]), (5, 1, 1));
    assert_eq!(c.frame()[c.frame().len() - 1], 1);
    assert_eq!(c.frame()[c.frame().len() - 7], 1);
    assert_eq!(c.work().coordinate_error_bound, Fixed::ZERO);
    assert!(std::ptr::eq(images.bytes().as_ptr(), bytes.as_ptr()));
}
#[test]
fn world_image_rebase_preserves_large_origin_precision() {
    let bytes = [255; 16];
    let images = PreparedImages::new(&manifest(&bytes), &bytes, &|| false).unwrap();
    let mut q = request();
    let frame = compile_images(&q, &images, &|| false)
        .unwrap()
        .frame()
        .to_vec();
    let origin = p(1 << 80, -(1 << 80));
    q.viewport.origin = origin;
    q.draws[0].origin = origin;
    brush(&mut q).origin = origin;
    assert_eq!(
        compile_images(&q, &images, &|| false).unwrap().frame(),
        frame
    );
    let rebased = q.draws[0].brush.rebased(origin).unwrap();
    assert_eq!(rebased, request().draws[0].brush);
}
#[test]
fn invalid_image_transforms_references_and_precision_are_explicit() {
    let bytes = [255; 16];
    let images = PreparedImages::new(&manifest(&bytes), &bytes, &|| false).unwrap();
    let mut q = request();
    brush(&mut q).resource = 1;
    assert!(matches!(
        compile_images(&q, &images, &|| false),
        Err(RasterError::Invalid("image resource reference"))
    ));
    let mut q = request();
    brush(&mut q).y_step = p(8, 0);
    assert!(matches!(
        compile_images(&q, &images, &|| false),
        Err(RasterError::Precision)
    ));
    let mut q = request();
    brush(&mut q).x_step = p(20000, 0);
    assert!(matches!(
        compile_images(&q, &images, &|| false),
        Err(RasterError::Range)
    ));
    let mut q = request();
    brush(&mut q).x_step.x = Fixed::from_raw(1);
    assert!(matches!(
        compile_images(&q, &images, &|| false),
        Err(RasterError::Precision)
    ));
    let mut q = request();
    q.viewport.coordinate_tolerance = Fixed::from_raw(256);
    brush(&mut q).origin.x = Fixed::from_raw((16000 << 32) + 1);
    assert!(compile_images(&q, &images, &|| false).is_ok());
    brush(&mut q).origin.x = Fixed::from_raw((16000 << 32) + (1 << 20));
    assert!(matches!(
        compile_images(&q, &images, &|| false),
        Err(RasterError::Precision)
    ));
}
#[test]
fn resource_validation_checks_cancellation_between_pixel_chunks() {
    let bytes = vec![255; 65536];
    let mut m = manifest(&bytes);
    m[0].width = 128;
    m[0].height = 128;
    let calls = Cell::new(0);
    let check = || {
        calls.set(calls.get() + 1);
        calls.get() == 5
    };
    assert!(matches!(
        PreparedImages::new(&m, &bytes, &check),
        Err(RasterError::Cancelled)
    ));
    assert_eq!(calls.get(), 5);
}
#[test]
fn image_execution_preserves_resource_identity_and_rejects_partial_output() {
    struct Backend {
        invalid: bool,
        bad: bool,
        calls: usize,
    }
    impl RasterBackend for Backend {
        fn raster(&mut self, _: &[u32]) -> Result<BackendReply, RasterError> {
            panic!("wrong entry")
        }
        fn raster_images(
            &mut self,
            frame: &[u32],
            bytes: &[u8],
        ) -> Result<BackendReply, RasterError> {
            assert_eq!(frame[1], 5);
            assert_eq!(bytes, [255; 16]);
            self.calls += 1;
            Ok(BackendReply {
                status: u32::from(self.bad),
                pixels: vec![0; 256],
            })
        }
        fn invalidate(&mut self) {
            self.invalid = true;
        }
    }
    let bytes = [255; 16];
    let images = PreparedImages::new(&manifest(&bytes), &bytes, &|| false).unwrap();
    let mut backend = Backend {
        invalid: false,
        bad: false,
        calls: 0,
    };
    let result = render_images(&request(), &images, &mut backend, &|| false).unwrap();
    assert_eq!(result.raster.info.profile, IMAGE_PROFILE);
    assert_eq!(&result.resources_sha256, images.sha256());
    backend.bad = true;
    assert!(matches!(
        render_images(&request(), &images, &mut backend, &|| false),
        Err(RasterError::ComponentInvalid(_))
    ));
    assert!(backend.invalid);
    assert_eq!(backend.calls, 2);
}
