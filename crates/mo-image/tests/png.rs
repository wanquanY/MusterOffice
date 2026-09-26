use flate2::read::ZlibDecoder;
use mo_image::png::{PngError, encode_to};
use sha2::{Digest as _, Sha256};
use std::{
    cell::Cell,
    io::{self, Read, Write},
};
fn decode(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    let mut offset = 8;
    let mut size = (0, 0);
    let mut compressed = Vec::new();
    let mut kinds = Vec::new();
    while offset < bytes.len() {
        let n = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let kind = &bytes[offset + 4..offset + 8];
        let data = &bytes[offset + 8..offset + 8 + n];
        let mut crc = crc32fast::Hasher::new();
        crc.update(kind);
        crc.update(data);
        assert_eq!(
            crc.finalize(),
            u32::from_be_bytes(bytes[offset + 8 + n..offset + 12 + n].try_into().unwrap())
        );
        match kind {
            b"IHDR" => {
                size = (
                    u32::from_be_bytes(data[..4].try_into().unwrap()),
                    u32::from_be_bytes(data[4..8].try_into().unwrap()),
                );
                assert_eq!(&data[8..], &[8, 6, 0, 0, 0]);
            }
            b"sRGB" => assert_eq!(data, &[0]),
            b"IDAT" => compressed.extend_from_slice(data),
            b"IEND" => assert!(data.is_empty()),
            _ => panic!("unexpected chunk"),
        }
        kinds.push(kind.to_vec());
        offset += n + 12;
    }
    assert_eq!(&kinds[..2], &[b"IHDR".to_vec(), b"sRGB".to_vec()]);
    assert_eq!(kinds.last().unwrap(), b"IEND");
    let mut filtered = Vec::new();
    ZlibDecoder::new(compressed.as_slice())
        .read_to_end(&mut filtered)
        .unwrap();
    let stride = size.0 as usize * 4;
    assert_eq!(filtered.len(), (stride + 1) * size.1 as usize);
    let mut decoded = vec![];
    for row in filtered.chunks_exact(stride + 1) {
        assert_eq!(row[0], 1);
        let base = decoded.len();
        for (i, delta) in row[1..].iter().enumerate() {
            decoded.push(delta.wrapping_add(if i < 4 { 0 } else { decoded[base + i - 4] }));
        }
    }
    (size.0, size.1, decoded)
}
#[test]
fn every_valid_alpha_channel_roundtrips_without_premultiplied_loss() {
    let mut pixels = vec![];
    for a in 0..=255u8 {
        for c in 0..=255u8 {
            let c = c.min(a);
            pixels.extend_from_slice(&[c, c, c, a]);
        }
    }
    let mut bytes = Vec::new();
    let receipt = encode_to(&pixels, 256, 256, &mut bytes, 2_000_000, &|| false).unwrap();
    assert_eq!(receipt.byte_length, bytes.len() as u64);
    assert_eq!(
        receipt.sha256.as_str(),
        format!("{:x}", Sha256::digest(&bytes))
    );
    let (w, h, decoded) = decode(&bytes);
    assert_eq!((w, h), (256, 256));
    for (before, after) in pixels.chunks_exact(4).zip(decoded.chunks_exact(4)) {
        assert_eq!(before[3], after[3]);
        for i in 0..3 {
            assert_eq!(
                before[i],
                ((u32::from(after[i]) * u32::from(after[3]) + 127) / 255) as u8
            );
        }
    }
    let mut again = Vec::new();
    encode_to(&pixels, 256, 256, &mut again, 2_000_000, &|| false).unwrap();
    assert_eq!(bytes, again);
}
#[test]
fn multiple_idat_chunks_and_short_writes_preserve_exact_opaque_pixels() {
    struct Short(Vec<u8>);
    impl Write for Short {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            let n = b.len().min(7);
            self.0.extend_from_slice(&b[..n]);
            Ok(n)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut seed = 1u32;
    let mut pixels = vec![];
    for _ in 0..256 * 256 {
        for _ in 0..3 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            pixels.push((seed >> 24) as u8);
        }
        pixels.push(255);
    }
    let mut output = Short(vec![]);
    encode_to(&pixels, 256, 256, &mut output, 1_000_000, &|| false).unwrap();
    assert!(output.0.len() > 65536 * 2);
    assert_eq!(decode(&output.0).2, pixels);
}
#[test]
fn bad_pixels_are_rejected_before_writing_and_budget_cancel_flush_errors_propagate() {
    for (bytes, w, h) in [(vec![1, 0, 0, 0], 1, 1), (vec![0; 3], 1, 1), (vec![], 0, 1)] {
        let mut output = vec![];
        assert!(encode_to(&bytes, w, h, &mut output, 1000, &|| false).is_err());
        assert!(output.is_empty());
    }
    let mut out = vec![];
    assert!(matches!(
        encode_to(&[0; 4], 1, 1, &mut out, 8, &|| false),
        Err(PngError::Limit(_))
    ));
    let checks = Cell::new(0);
    let mut out = vec![];
    assert!(matches!(
        encode_to(&vec![0; 400], 10, 10, &mut out, 1000, &|| {
            let n = checks.get();
            checks.set(n + 1);
            n > 15
        }),
        Err(PngError::Cancelled)
    ));
    struct BadFlush(Vec<u8>);
    impl Write for BadFlush {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            self.0.extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("flush failed"))
        }
    }
    assert!(matches!(
        encode_to(&[0; 4], 1, 1, &mut BadFlush(vec![]), 1000, &|| false),
        Err(PngError::Io(_))
    ));
}
