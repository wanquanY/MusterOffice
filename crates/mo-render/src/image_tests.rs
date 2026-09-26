use super::*;
use mo_common::Digest;
use mo_geometry::{Fixed, Point};
use mo_raster::{
    BackendReply, Brush, ImageAlpha, ImageBrush, ImageResource, ImageSampling, ImageTile,
    PreparedImages, RasterBackend, RasterError,
};
use sha2::{Digest as _, Sha256};
use std::cell::Cell;
static PIXELS: [u8; 16] = [255, 0, 0, 255, 0, 128, 0, 128, 0, 0, 255, 255, 0, 0, 0, 0];
fn images() -> PreparedImages<'static> {
    PreparedImages::new(
        &[ImageResource {
            width: 2,
            height: 2,
            alpha: ImageAlpha::Premultiplied,
            sha256: Digest::from_sha256(Sha256::digest(PIXELS).into()),
        }],
        &PIXELS,
        &|| false,
    )
    .unwrap()
}
fn with_images(mut q: SceneRasterRequest) -> SceneRasterRequest {
    for instance in &mut q.scene.instances {
        instance.brush = Brush::Image {
            image: ImageBrush {
                uncertainty: None,
                source_domain: None,
                resource: 0,
                origin: q.viewport.origin,
                x_step: Point {
                    x: Fixed::from_raw(4 << 32),
                    y: Fixed::ZERO,
                },
                y_step: Point {
                    x: Fixed::ZERO,
                    y: Fixed::from_raw(4 << 32),
                },
                tile_x: ImageTile::Repeat,
                tile_y: ImageTile::Mirror,
                sampling: ImageSampling::Linear,
            },
        };
    }
    q
}
#[derive(Default)]
struct Backend {
    calls: usize,
    invalid: bool,
    bad: bool,
}
impl RasterBackend for Backend {
    fn raster(&mut self, _: &[u32]) -> Result<BackendReply, RasterError> {
        panic!("image resources must reach the image executor")
    }
    fn raster_images(&mut self, frame: &[u32], bytes: &[u8]) -> Result<BackendReply, RasterError> {
        self.calls += 1;
        assert_eq!(bytes.as_ptr(), PIXELS.as_ptr());
        assert_eq!(bytes, PIXELS);
        Ok(BackendReply {
            status: 0,
            pixels: vec![
                0;
                if self.bad {
                    0
                } else {
                    frame[2] as usize * frame[3] as usize * 4
                }
            ],
        })
    }
    fn invalidate(&mut self) {
        self.invalid = true;
    }
}
#[test]
fn scene_images_reuse_both_geometry_and_world_paints() {
    let q = with_images(tests::request());
    let images = images();
    let compiled = compile_images(&q, &images, &|| false).unwrap();
    assert_eq!(compiled.work().compiled_paths, 1);
    assert_eq!(compiled.raster().work().resources, 1);
    assert_eq!(compiled.raster().work().brushes, 1);
    assert_eq!(compiled.raster().work().draws, 2);
    assert_eq!(compiled.raster().frame()[1], 5);
    assert_eq!(compiled.raster().raster_work().draws, 2);
    let mut backend = Backend::default();
    for _ in 0..2 {
        let image = render_images(&q, &images, &mut backend, &|| false).unwrap();
        assert_eq!(image.info.resources_sha256, images.sha256().clone());
        assert_eq!(image.info.scene.raster.profile, mo_raster::IMAGE_PROFILE);
    }
    assert_eq!(backend.calls, 2);
    assert!(!backend.invalid);
    assert!(matches!(
        compile(&q, &|| false),
        Err(RasterError::Invalid("image resources required"))
    ));
}
#[test]
fn enormous_scene_origin_is_removed_once_from_paths_and_image_paints() {
    let q = with_images(tests::request());
    let images = images();
    let compiled = compile_images(&q, &images, &|| false).unwrap();
    let mut translated = q.clone();
    let delta = Fixed::from_raw(1 << 100);
    translated.viewport.origin.x = delta;
    translated.scene.transforms[0].affine.translation.x = translated.scene.transforms[0]
        .affine
        .translation
        .x
        .checked_add(delta)
        .unwrap();
    for instance in &mut translated.scene.instances {
        let Brush::Image { image } = &mut instance.brush else {
            panic!()
        };
        image.origin.x = delta;
    }
    let shifted = compile_images(&translated, &images, &|| false).unwrap();
    assert_eq!(compiled.raster().frame(), shifted.raster().frame());
    assert_eq!(
        compiled.work().combined_coordinate_error_bound,
        shifted.work().combined_coordinate_error_bound
    );
}
#[test]
fn image_scenes_share_original_chain_precision_recovery() {
    let q = with_images(tests::precision_request());
    let images = images();
    let compiled = compile_images(&q, &images, &|| false).unwrap();
    assert_eq!(compiled.work().lowering_attempts, 2);
    assert!(compiled.work().combined_coordinate_error_bound <= q.viewport.coordinate_tolerance);
    assert!(compiled.raster().work().coordinate_error_bound <= q.viewport.coordinate_tolerance);
}
#[test]
fn invalid_image_and_graph_inputs_never_reach_backend() {
    let images = images();
    for mutation in 0..4 {
        let mut q = with_images(tests::request());
        let mut backend = Backend::default();
        match mutation {
            0 => {
                let Brush::Image { image } = &mut q.scene.instances[0].brush else {
                    panic!()
                };
                image.resource = 1;
            }
            1 => q.scene.instances[0].path = 10,
            2 => q.scene.transforms[0].parent = Some(0),
            _ => {
                let Brush::Image { image } = &mut q.scene.instances[0].brush else {
                    panic!()
                };
                image.x_step = Point {
                    x: Fixed::ZERO,
                    y: Fixed::ZERO,
                };
            }
        }
        assert!(render_images(&q, &images, &mut backend, &|| false).is_err());
        assert_eq!(backend.calls, 0);
        assert!(!backend.invalid);
    }
}
#[test]
fn scene_image_cancellation_is_atomic_at_every_checkpoint() {
    let images = images();
    for q in [tests::request(), tests::precision_request()].map(with_images) {
        let total = Cell::new(0);
        render_images(&q, &images, &mut Backend::default(), &|| {
            total.set(total.get() + 1);
            false
        })
        .unwrap();
        for stop in 0..total.get() {
            let n = Cell::new(0);
            let mut backend = Backend::default();
            let result = render_images(&q, &images, &mut backend, &|| {
                let previous = n.get();
                n.set(previous + 1);
                previous == stop
            });
            assert!(matches!(result, Err(RasterError::Cancelled)), "{stop}");
            assert_eq!(backend.invalid, backend.calls != 0);
        }
    }
}
#[test]
fn invalid_component_reply_discards_scene_image_output() {
    let q = with_images(tests::request());
    let images = images();
    let mut backend = Backend {
        bad: true,
        ..Default::default()
    };
    assert!(matches!(
        render_images(&q, &images, &mut backend, &|| false),
        Err(RasterError::ComponentInvalid(_))
    ));
    assert_eq!(backend.calls, 1);
    assert!(backend.invalid);
}
