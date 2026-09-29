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

#[test]
fn stepped_validation_covers_chunk_edges_and_keeps_pixels_and_digest_owned() {
    for length in [0usize, 4, 16380, 16384, 16388, 32768, 32772] {
        let pixels: Vec<u8> = (0..length)
            .map(|n| if n % 4 == 3 { 255 } else { (n % 253) as u8 })
            .collect();
        let digest = Digest::from_sha256(Sha256::digest(&pixels).into());
        for units in [1, 2, 7, 4096] {
            let mut validation = RasterReplyValidation::new(
                BackendReply {
                    status: 0,
                    pixels: pixels.clone(),
                },
                &|| false,
            )
            .unwrap();
            assert!(validation.step(0, &|| false).is_err());
            assert!(validation.step(4097, &|| false).is_err());
            let mut calls = 1;
            while !validation.step(units, &|| false).unwrap() {
                calls += 1;
            }
            assert_eq!(
                calls,
                length
                    .div_ceil(VALIDATION_BYTES_PER_UNIT * units as usize)
                    .max(1)
            );
            assert!(validation.step(1, &|| false).unwrap());
            let verified = validation.take().unwrap();
            assert_eq!(verified.pixels, pixels);
            assert_eq!(verified.sha256, digest);
        }
    }
}

#[test]
fn validation_failure_and_cancellation_are_terminal_without_partial_proofs() {
    for offset in [0, 16380, 16384, 32764, 32768] {
        for channel in 0..3 {
            let mut pixels = vec![0; 32772];
            pixels[offset + channel] = 1;
            let mut validation =
                RasterReplyValidation::new(BackendReply { status: 0, pixels }, &|| false).unwrap();
            for _ in 0..offset / 16384 {
                assert!(!validation.step(1, &|| false).unwrap());
            }
            assert!(matches!(
                validation.step(1, &|| false),
                Err(RasterError::ComponentInvalid("premultiplied channels"))
            ));
            assert!(validation.step(1, &|| false).is_err());
            assert!(validation.take().is_err());
        }
    }
    let new = || {
        RasterReplyValidation::new(
            BackendReply {
                status: 0,
                pixels: vec![0; 32772],
            },
            &|| false,
        )
        .unwrap()
    };
    assert!(new().take().is_err());
    let n = Cell::new(0);
    assert!(
        new()
            .step(7, &|| {
                n.set(n.get() + 1);
                false
            })
            .unwrap()
    );
    for at in 1..=n.get() {
        let mut validation = new();
        let calls = Cell::new(0);
        assert!(matches!(
            validation.step(7, &|| {
                calls.set(calls.get() + 1);
                calls.get() == at
            }),
            Err(RasterError::Cancelled)
        ));
        assert!(validation.take().is_err());
    }
    for length in [1, 3, MAX_PIXEL_BYTES + 4] {
        assert!(matches!(
            RasterReplyValidation::new(
                BackendReply {
                    status: 0,
                    pixels: vec![0; length]
                },
                &|| false
            ),
            Err(RasterError::ComponentInvalid("pixel length"))
        ));
    }
}

#[test]
fn validated_completion_matches_raw_and_still_checks_exact_batch_dimensions() {
    let validate = |reply| {
        let mut validation = RasterReplyValidation::new(reply, &|| false).unwrap();
        while !validation.step(1, &|| false).unwrap() {}
        validation.take().unwrap()
    };
    let raw = batch().complete(reply(), &|| false).unwrap();
    let stepped = batch().complete(validate(reply()), &|| false).unwrap();
    assert_eq!(raw.pixels, stepped.pixels);
    assert_eq!(
        serde_json::to_value(raw.info).unwrap(),
        serde_json::to_value(stepped.info).unwrap()
    );
    let short = validate(BackendReply {
        status: 0,
        pixels: vec![0; 4],
    });
    assert!(matches!(
        batch().complete(short, &|| false),
        Err(RasterError::ComponentInvalid("pixel length"))
    ));
}
