//! Extra structural checks around the ZIP reader. No compression or ZIP index parser.
use crate::{OpcError, ReaderAt};
use rawzip::ZipFileHeaderRecord;

/// Includes the descriptor in the entry's occupied range. A ZIP64 local header may
/// precede a small stream whose central sizes later fit in 32 bits (e.g. Python).
/// Optional signature ambiguity is resolved against the central CRC and sizes.
pub(crate) fn occupied_end(
    reader: &impl ReaderAt,
    header: &ZipFileHeaderRecord<'_>,
    data_end: u64,
    directory_start: u64,
) -> Result<u64, OpcError> {
    if data_end > directory_start {
        return Err(OpcError::Structure("entry overlaps ZIP directory".into()));
    }
    if !header.flags().has_data_descriptor() {
        return Ok(data_end);
    }
    let mut central_sizes = [0; 8];
    let mut local_sizes = [0; 8];
    reader.read_exact_at(&mut central_sizes, header.central_directory_offset() + 20)?;
    reader.read_exact_at(&mut local_sizes, header.local_header_offset() + 18)?;
    let zip64 = central_sizes
        .chunks_exact(4)
        .chain(local_sizes.chunks_exact(4))
        .any(|size| size == [0xFF; 4]);
    let available = (directory_start - data_end).min(24) as usize;
    let mut bytes = [0; 24];
    reader.read_exact_at(&mut bytes[..available], data_end)?;
    let length = descriptor_length(
        &bytes[..available],
        zip64,
        header.crc32(),
        header.compressed_size_hint(),
        header.uncompressed_size_hint(),
    )?;
    Ok(data_end + length as u64)
}

fn descriptor_length(
    bytes: &[u8],
    zip64: bool,
    crc: u32,
    compressed: u64,
    inflated: u64,
) -> Result<usize, OpcError> {
    let mut expected = Vec::with_capacity(24);
    expected.extend_from_slice(&crc.to_le_bytes());
    if zip64 {
        expected.extend_from_slice(&compressed.to_le_bytes());
        expected.extend_from_slice(&inflated.to_le_bytes());
    } else {
        let compressed = u32::try_from(compressed)
            .map_err(|_| OpcError::Structure("missing ZIP64 compressed size".into()))?;
        let inflated = u32::try_from(inflated)
            .map_err(|_| OpcError::Structure("missing ZIP64 inflated size".into()))?;
        expected.extend_from_slice(&compressed.to_le_bytes());
        expected.extend_from_slice(&inflated.to_le_bytes());
    }
    let unsigned = bytes.starts_with(&expected);
    let signed = bytes.starts_with(b"PK\x07\x08")
        && bytes.get(4..).is_some_and(|b| b.starts_with(&expected));
    match (unsigned, signed) {
        (true, false) => Ok(expected.len()),
        (false, true) => Ok(expected.len() + 4),
        (true, true) => Err(OpcError::Structure("ambiguous ZIP descriptor".into())),
        (false, false) => Err(OpcError::Structure(
            "ZIP descriptor disagreement or overlap".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptors_accept_both_widths_and_signature_modes_including_crc_collision() {
        for zip64 in [false, true] {
            for signed in [false, true] {
                for crc in [0x12345678_u32, 0x08074b50] {
                    let mut data = Vec::new();
                    if signed {
                        data.extend_from_slice(b"PK\x07\x08");
                    }
                    data.extend_from_slice(&crc.to_le_bytes());
                    if zip64 {
                        data.extend_from_slice(&32_u64.to_le_bytes());
                        data.extend_from_slice(&64_u64.to_le_bytes());
                    } else {
                        data.extend_from_slice(&32_u32.to_le_bytes());
                        data.extend_from_slice(&64_u32.to_le_bytes());
                    }
                    assert_eq!(
                        descriptor_length(&data, zip64, crc, 32, 64).unwrap(),
                        data.len()
                    );
                    assert!(
                        descriptor_length(&data[..data.len() - 1], zip64, crc, 32, 64).is_err()
                    );
                }
            }
        }
    }
}
