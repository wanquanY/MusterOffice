use super::*;
use std::io::Write;
fn encoded(width: u32, height: u32) -> (Vec<u8>, Digest) {
    let pixels: Vec<_> = (0..width * height)
        .flat_map(|i| {
            let a = (i % 256) as u8;
            [a, a / 2, a / 3, a]
        })
        .collect();
    let digest = artifact::digest(&pixels, pixels.len() as u64, &|| false).unwrap();
    let mut png = Vec::new();
    mo_image::png::encode_to(&pixels, width, height, &mut png, 1024 * 1024, &|| false).unwrap();
    (png, digest)
}
fn inspect(png: &[u8], width: u32, height: u32, digest: &Digest) -> Result<(), DeliveryError> {
    verify(
        &Content {
            reader: &png,
            byte_length: png.len() as u64,
        },
        width,
        height,
        digest,
        &|| false,
    )
}
fn chunk(out: &mut Vec<u8>, tag: &[u8; 4], bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    out.extend_from_slice(tag);
    out.extend_from_slice(bytes);
    let mut crc = crc32fast::Hasher::new();
    crc.update(tag);
    crc.update(bytes);
    out.extend_from_slice(&crc.finalize().to_be_bytes());
}
fn compressed_png(rows: &[u8], suffix: &[u8]) -> Vec<u8> {
    let (original, _) = encoded(2, 2);
    let mut out = original[..46].to_vec(); // signature + IHDR + sRGB
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    z.write_all(rows).unwrap();
    let mut body = z.finish().unwrap();
    body.extend_from_slice(suffix);
    // Split the compressed stream across every possible byte boundary.
    for byte in body {
        chunk(&mut out, b"IDAT", &[byte]);
    }
    chunk(&mut out, b"IEND", &[]);
    out
}
#[test]
fn all_alpha_values_recover_the_exact_premultiplied_pixels() {
    let (png, sha) = encoded(256, 3);
    inspect(&png, 256, 3, &sha).unwrap();
    assert!(inspect(&png, 256, 3, &Digest::from_sha256([0; 32])).is_err());
}
#[test]
fn crc_truncation_dimensions_and_trailing_bytes_are_checked() {
    let (png, sha) = encoded(2, 2);
    for cut in [0, 7, 15, png.len() - 1] {
        assert!(inspect(&png[..cut], 2, 2, &sha).is_err());
    }
    let mut crc = png.clone();
    crc[40] ^= 1;
    assert!(inspect(&crc, 2, 2, &sha).is_err());
    let mut trailing = png.clone();
    trailing.push(0);
    assert!(inspect(&trailing, 2, 2, &sha).is_err());
    assert!(inspect(&png, 3, 2, &sha).is_err());
}
#[test]
fn zlib_exact_extent_and_profile_filters_are_checked() {
    // All-zero transparent RGBA; Sub-filter byte precedes each scanline.
    let mut rows = [0; 18];
    rows[0] = 1;
    rows[9] = 1;
    let pixels = vec![0; 16];
    let sha = artifact::digest(&pixels, 16, &|| false).unwrap();
    inspect(&compressed_png(&rows, &[]), 2, 2, &sha).unwrap();
    assert!(inspect(&compressed_png(&rows, &[0]), 2, 2, &sha).is_err());
    rows[0] = 0;
    assert!(inspect(&compressed_png(&rows, &[]), 2, 2, &sha).is_err());
    rows[0] = 1;
    let mut too_many = rows.to_vec();
    too_many.push(0);
    assert!(inspect(&compressed_png(&too_many, &[]), 2, 2, &sha).is_err());
    assert!(matches!(
        inspect(&compressed_png(&rows[..17], &[]), 2, 2, &sha),
        Err(DeliveryError::Invalid(_))
    ));
}
#[test]
fn invisible_rgb_and_cancellation_are_not_silently_accepted() {
    let pixels = vec![0; 16];
    let sha = artifact::digest(&pixels, 16, &|| false).unwrap();
    let mut rows = [0; 18];
    rows[0] = 1;
    rows[9] = 1;
    rows[1] = 1;
    assert!(inspect(&compressed_png(&rows, &[]), 2, 2, &sha).is_err());
    let (png, sha) = encoded(256, 3);
    let steps = Cell::new(0);
    let check = || {
        steps.set(steps.get() + 1);
        steps.get() > 8
    };
    assert!(matches!(
        verify(
            &Content {
                reader: &png,
                byte_length: png.len() as u64
            },
            256,
            3,
            &sha,
            &check
        ),
        Err(DeliveryError::Cancelled)
    ));
}

#[test]
fn source_io_failure_is_distinct_from_malformed_deflate() {
    struct FailsOnDecode {
        bytes: Vec<u8>,
        payload_reads: Cell<usize>,
    }
    impl mo_opc::ReaderAt for FailsOnDecode {
        fn read_at(&self, output: &mut [u8], offset: u64) -> io::Result<usize> {
            if offset == 54 {
                self.payload_reads.set(self.payload_reads.get() + 1);
                if self.payload_reads.get() == 2 {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "fixture storage timeout",
                    ));
                }
            }
            self.bytes.read_at(output, offset)
        }
    }
    let (bytes, sha) = encoded(2, 2);
    let source = FailsOnDecode {
        bytes,
        payload_reads: Cell::new(0),
    };
    assert!(
        matches!(verify(&Content {reader: &source, byte_length: source.bytes.len() as u64},2,2,&sha,&||false),
        Err(DeliveryError::Io(e)) if e.kind() == io::ErrorKind::TimedOut)
    );
}
