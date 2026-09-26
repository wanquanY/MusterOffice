use super::*;
use std::cell::Cell;
const SOURCE: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 2, 8, 6, 0,
    0, 0, 153, 129, 182, 39, 0, 0, 0, 11, 73, 68, 65, 84, 120, 156, 99, 96, 128, 1, 0, 0, 10, 0, 1,
    127, 128, 116, 94, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];
struct Backend {
    reply: Option<DecoderReply>,
    calls: u32,
    invalid: bool,
}
impl ImageDecoder for Backend {
    fn decode(&mut self, _: &[u8]) -> Result<DecoderReply, ImageError> {
        self.calls += 1;
        Ok(self.reply.take().unwrap())
    }
    fn invalidate(&mut self) {
        self.invalid = true;
    }
}
fn backend() -> Backend {
    Backend {
        reply: Some(DecoderReply {
            status: 0,
            words: [2, 1, 1, 2, 6, 1, 8, 0, 8],
            pixels: vec![5, 6, 7, 9, 0, 0, 0, 0],
        }),
        calls: 0,
        invalid: false,
    }
}
fn digest() -> Digest {
    hash(SOURCE, &|| false).unwrap()
}
#[test]
fn identity_is_checked_before_entering_component() {
    let mut b = backend();
    assert!(matches!(
        decode(b"wrong", &digest(), &mut b, &|| false),
        Err(ImageError::Invalid(_))
    ));
    assert_eq!(b.calls, 0);
    assert!(!b.invalid);
}
#[test]
fn normalized_resource_retains_source_identity_and_pixel_digest() {
    let mut b = backend();
    let image = decode(SOURCE, &digest(), &mut b, &|| false).unwrap();
    assert_eq!(image.info().orientation, 6);
    assert_eq!(image.info().source_sha256, digest());
    assert_eq!(
        image.info().pixels_sha256,
        hash(image.pixels(), &|| false).unwrap()
    );
    assert!(!b.invalid);
}
#[test]
fn every_cancellation_checkpoint_discards_output() {
    let total = Cell::new(0);
    decode(SOURCE, &digest(), &mut backend(), &|| {
        total.set(total.get() + 1);
        false
    })
    .unwrap();
    for stop in 1..=total.get() {
        let calls = Cell::new(0);
        let mut b = backend();
        let result = decode(SOURCE, &digest(), &mut b, &|| {
            calls.set(calls.get() + 1);
            calls.get() == stop
        });
        assert!(matches!(result, Err(ImageError::Cancelled)));
        assert_eq!(b.invalid, b.calls > 0);
    }
}
#[test]
fn contradictory_component_metadata_is_quarantined() {
    for (index, value) in [
        (0, 0),
        (1, 8193),
        (2, 2),
        (4, 9),
        (5, 2),
        (6, 32),
        (7, 3),
        (8, 4),
    ] {
        let mut b = backend();
        b.reply.as_mut().unwrap().words[index] = value;
        assert!(matches!(
            decode(SOURCE, &digest(), &mut b, &|| false),
            Err(ImageError::ComponentInvalid(_))
        ));
        assert!(b.invalid);
    }
}
#[test]
fn invalid_alpha_and_failure_payload_never_escape() {
    for failed in [false, true] {
        let mut b = backend();
        if failed {
            b.reply.as_mut().unwrap().status = 5;
        } else {
            b.reply.as_mut().unwrap().pixels[0] = 255;
        }
        assert!(matches!(
            decode(SOURCE, &digest(), &mut b, &|| false),
            Err(ImageError::ComponentInvalid(_))
        ));
        assert!(b.invalid);
    }
}
#[test]
fn ordinary_failure_is_reusable_but_allocation_failure_is_not() {
    for status in [1, 2, 3, 5] {
        let mut b = backend();
        b.reply = Some(DecoderReply {
            status,
            words: [0; 9],
            pixels: vec![],
        });
        assert!(
            matches!(decode(SOURCE, &digest(), &mut b, &|| false), Err(ImageError::Component(s)) if s==status)
        );
        assert_eq!(b.invalid, status == 2);
    }
}
