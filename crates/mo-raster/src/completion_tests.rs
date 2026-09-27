use super::*;
use mo_common::Digest;
use sha2::{Digest as _, Sha256};
use std::{cell::Cell, sync::Arc};

fn batch() -> CompiledRaster {
    compile(&tests::request(), &|| false).unwrap()
}
fn reply() -> BackendReply {
    BackendReply {
        status: 0,
        pixels: vec![0; 16 * 16 * 4],
    }
}

#[test]
fn completion_checks_status_length_alpha_and_every_cancellation_checkpoint() {
    for status in [1, 2, 3, 4] {
        let error = batch()
            .complete(
                BackendReply {
                    status,
                    pixels: vec![],
                },
                &|| false,
            )
            .err()
            .unwrap();
        assert!(matches!(error, RasterError::Component(v) if v == status));
        assert_eq!(error.invalidates_backend(), matches!(status, 2 | 4));
    }
    for bad in [
        BackendReply {
            status: 5,
            pixels: vec![],
        },
        BackendReply {
            status: 1,
            pixels: vec![0],
        },
        BackendReply {
            status: 0,
            pixels: vec![0],
        },
        BackendReply {
            status: 0,
            pixels: [vec![1, 0, 0, 0], vec![0; 16 * 16 * 4 - 4]].concat(),
        },
    ] {
        let error = batch().complete(bad, &|| false).err().unwrap();
        assert!(matches!(error, RasterError::ComponentInvalid(_)));
        assert!(error.invalidates_backend());
    }
    let n = Cell::new(0);
    batch()
        .complete(reply(), &|| {
            n.set(n.get() + 1);
            false
        })
        .unwrap();
    for at in 1..=n.get() {
        let calls = Cell::new(0);
        assert!(matches!(
            batch().complete(reply(), &|| {
                calls.set(calls.get() + 1);
                calls.get() == at
            }),
            Err(RasterError::Cancelled)
        ));
    }
}

#[test]
fn shared_image_batch_keeps_original_allocation_until_completion_or_drop() {
    let bytes = vec![5, 6, 7, 255];
    let resources = Arc::new(
        PreparedImages::owned(
            &[ImageResource {
                width: 1,
                height: 1,
                alpha: ImageAlpha::Premultiplied,
                sha256: Digest::from_sha256(Sha256::digest(&bytes).into()),
            }],
            bytes,
            &|| false,
        )
        .unwrap(),
    );
    let weak = Arc::downgrade(&resources);
    let address = resources.bytes().as_ptr();
    let hash = resources.sha256().clone();
    let batch =
        compile_shared_images(&tests::request(), Arc::clone(&resources), &|| false).unwrap();
    let abandoned =
        compile_shared_images(&tests::request(), Arc::clone(&resources), &|| false).unwrap();
    assert_eq!(batch.images().as_ptr(), address);
    drop(resources);
    assert_eq!(weak.strong_count(), 2);
    drop(abandoned);
    assert_eq!(weak.strong_count(), 1);
    let result = batch.complete(reply(), &|| false).unwrap();
    assert_eq!(result.resources_sha256, hash);
    assert!(weak.upgrade().is_none());
}
