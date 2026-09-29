use super::*;
use mo_common::Emu;
use mo_geometry::{Fixed, PathCommand as C};
use std::cell::Cell;
const ZERO: Point = Point {
    x: Fixed::ZERO,
    y: Fixed::ZERO,
};
fn owner() -> Digest {
    Digest::from_sha256([1; 32])
}
fn key(object: u32, segments: u32) -> Key {
    Key {
        object,
        size: Size {
            width: Emu::new(1000),
            height: Emu::new(700),
        },
        anchor: ZERO,
        segments,
    }
}
fn outline(segments: u32, commands: usize) -> Outline {
    Outline {
        commands: vec![C::Move { to: ZERO }; commands],
        error: ZERO,
        curve_segments: segments,
    }
}
#[test]
fn owner_and_local_geometry_inputs_fence_reuse_without_rebuilding_a_hit() {
    let mut cache = PageGeometry::new(owner(), SlideId::new("slide:1").unwrap());
    cache
        .verify_owner(&owner(), &SlideId::new("slide:1").unwrap())
        .unwrap();
    assert!(
        cache
            .verify_owner(&owner(), &SlideId::new("slide:2").unwrap())
            .is_err()
    );
    assert!(
        cache
            .verify_owner(
                &Digest::from_sha256([2; 32]),
                &SlideId::new("slide:1").unwrap()
            )
            .is_err()
    );
    let original = key(1, 4);
    cache
        .outline(original, &|| false, || Ok(outline(4, 10)))
        .unwrap();
    cache
        .outline(original, &|| false, || panic!("must reuse"))
        .unwrap();
    let mut size = original;
    size.size.width = Emu::new(2000);
    let mut anchor = size;
    anchor.anchor.x = Fixed::from_raw(100);
    let mut precision = anchor;
    precision.segments = 8;
    for changed in [size, anchor, precision, original] {
        let called = Cell::new(false);
        let result = cache
            .outline(changed, &|| false, || {
                called.set(true);
                Ok(outline(changed.segments, 20))
            })
            .unwrap();
        assert!(called.get());
        assert_eq!(result.curve_segments, changed.segments);
        assert_eq!(cache.entries.len(), 1);
        assert_eq!(cache.payload_bytes, cache.entries[&1].bytes);
    }
}
#[test]
fn capacity_is_bounded_and_non_admitted_geometry_does_not_evict_useful_entries() {
    let mut cache = PageGeometry::new(owner(), SlideId::new("slide:1").unwrap());
    for id in 0..MAX_ENTRIES + 32 {
        let result = cache
            .outline(key(id as u32, 1), &|| false, || Ok(outline(1, 0)))
            .unwrap();
        assert_eq!(
            matches!(result, LocalOutline::Retained(_)),
            id < MAX_ENTRIES
        );
        assert!(cache.payload_bytes <= MAX_PAYLOAD_BYTES);
    }
    assert_eq!(cache.entries.len(), MAX_ENTRIES);
    cache
        .outline(key(0, 1), &|| false, || {
            panic!("capacity must not evict the first object")
        })
        .unwrap();
    let large = MAX_PAYLOAD_BYTES / size_of::<C>() + 1;
    assert!(matches!(
        cache
            .outline(key(0, 2), &|| false, || Ok(outline(2, large)))
            .unwrap(),
        LocalOutline::Transient(_)
    ));
    cache
        .outline(key(0, 1), &|| false, || {
            panic!("oversized replacement must preserve the prior value")
        })
        .unwrap();
    let mut cache = PageGeometry::new(owner(), SlideId::new("slide:1").unwrap());
    let commands = MAX_PAYLOAD_BYTES / size_of::<C>() / 2;
    cache
        .outline(key(0, 1), &|| false, || Ok(outline(1, commands)))
        .unwrap();
    assert!(matches!(
        cache
            .outline(key(1, 1), &|| false, || Ok(outline(1, commands)))
            .unwrap(),
        LocalOutline::Transient(_)
    ));
    assert_eq!(cache.entries.len(), 1);
    assert!(cache.payload_bytes <= MAX_PAYLOAD_BYTES);
}
#[test]
fn cancelled_hits_and_failed_replacements_never_publish_incomplete_geometry() {
    let mut cache = PageGeometry::new(owner(), SlideId::new("slide:1").unwrap());
    cache
        .outline(key(1, 4), &|| false, || Ok(outline(4, 5)))
        .unwrap();
    let before = cache.payload_bytes;
    assert!(matches!(
        cache.outline(key(1, 4), &|| true, || panic!("cancelled hit")),
        Err(CompileError::Cancelled)
    ));
    assert!(
        cache
            .outline(key(1, 8), &|| false, || Err(CompileError::Range))
            .is_err()
    );
    let cancel = Cell::new(false);
    assert!(matches!(
        cache.outline(key(1, 8), &|| cancel.get(), || {
            cancel.set(true);
            Ok(outline(8, 50))
        }),
        Err(CompileError::Cancelled)
    ));
    assert_eq!(cache.payload_bytes, before);
    assert_eq!(
        cache
            .outline(key(1, 4), &|| false, || panic!(
                "old value must remain valid"
            ))
            .unwrap()
            .curve_segments,
        4
    );
}
