use super::*;
use crate::{ByteRange, Sample};

// Header fragments from our generated 128x96 B/VFR colour fixture and FFmpeg's
// h264_metadata=chroma_sample_loc_type=1 rewrite. No coded picture data required.
const LEFT: &str = "0164000affe1001a6764000aacd9420db016a020202800001f480007530078912cb001000668ebe3cb22c0fdf8f800";
const CENTER: &str = "0164000affe1001b6764000aacd9420db016a0202034a000007d20001d4c01e244b2c001000668ebe3cb22c0fdf8f800";
const SLICE: &str = "658884009fd9674d7dda7811d6224d07599d810e76066ca6dbbc4cbe5f7aaa2d";
fn hex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks(2)
        .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
        .collect()
}
fn entry(configuration: &str) -> SampleDescription {
    let config = hex(configuration);
    let mut payload = vec![0; 78];
    payload.extend_from_slice(&((config.len() + 8) as u32).to_be_bytes());
    payload.extend_from_slice(b"avcC");
    payload.extend(config);
    SampleDescription {
        format: *b"avc3",
        payload,
    }
}
fn packet(nals: &[Vec<u8>]) -> Vec<u8> {
    nals.iter()
        .flat_map(|nal| [(nal.len() as u32).to_be_bytes().to_vec(), nal.clone()].concat())
        .collect()
}
fn fixture() -> (TrackIndex, Vec<u8>) {
    let config = hex(CENTER);
    let sps = config[8..35].to_vec();
    let packets = [
        packet(&[hex(SLICE)]),
        packet(&[sps, hex(SLICE)]),
        packet(&[hex(SLICE)]),
        packet(&[hex(SLICE)]),
    ];
    let mut bytes = vec![];
    let samples = packets
        .into_iter()
        .enumerate()
        .map(|(i, p)| {
            let sample = Sample {
                bytes: ByteRange {
                    offset: bytes.len() as u64,
                    length: p.len() as u64,
                },
                dts: i as i64,
                pts: i as i64,
                decode_duration: 1,
                description: if i == 3 { 1 } else { 0 },
                sync_sample: true,
            };
            bytes.extend(p);
            sample
        })
        .collect();
    (
        TrackIndex {
            id: 1,
            flags: 1,
            handler: *b"vide",
            declared_track_duration: None,
            timescale: 25,
            declared_media_duration: None,
            decode_duration: 4,
            display_matrix: [0; 9],
            display_size_16_16: [0; 2],
            descriptions: vec![entry(LEFT), entry(LEFT)],
            samples,
            edits: vec![],
            auxiliary: vec![],
        },
        bytes,
    )
}
#[test]
fn batch_scan_backward_seek_and_description_switch_keep_exact_parameter_state() {
    let (track, bytes) = fixture();
    let mut index = AvcIndex::new(1 << 20);
    assert_eq!(index.sample(0, 0).unwrap_err().kind, ErrorKind::State);
    let mut reader = |offset, out: &mut [u8]| {
        out.copy_from_slice(&bytes[offset as usize..offset as usize + out.len()]);
        Ok(out.len())
    };
    for expected in 1..=4 {
        assert_eq!(
            index.scan(0, &track, 3, 1, &mut reader, &|| false).unwrap(),
            expected
        );
    }
    let locations = ["left", "center", "center", "left"];
    for sample in [3, 0, 2, 1] {
        let (description, picture) = index.sample(0, sample).unwrap();
        assert_eq!(description, u32::from(sample == 3));
        assert_eq!(
            picture
                .unwrap()
                .sequence
                .progressive_chroma_location()
                .unwrap(),
            Some(locations[sample as usize])
        );
        let mut decoder = AvcInspector::from_avcc(&hex(LEFT)).unwrap();
        let mut first = index.bootstrap(0, sample).unwrap();
        let range = track.samples[sample as usize].bytes;
        first.extend_from_slice(
            &bytes[range.offset as usize..(range.offset + range.length) as usize],
        );
        assert_eq!(
            decoder
                .inspect(&first, &|| false)
                .unwrap()
                .unwrap()
                .sequence,
            picture.unwrap().sequence
        );
    }
    assert_eq!(
        index
            .scan(
                0,
                &track,
                0,
                1,
                &mut |_, _: &mut [u8]| panic!("cached query read"),
                &|| false
            )
            .unwrap(),
        4
    );
    assert_eq!(
        index
            .scan(0, &track, 0, 1, &mut reader, &|| true)
            .unwrap_err()
            .kind,
        ErrorKind::Cancelled
    );
}
#[test]
fn failed_sample_does_not_publish_a_checkpoint_or_poison_earlier_results() {
    let (track, bytes) = fixture();
    let mut index = AvcIndex::new(1 << 20);
    let mut reader = |offset, out: &mut [u8]| {
        out.copy_from_slice(&bytes[offset as usize..offset as usize + out.len()]);
        Ok(out.len())
    };
    index.scan(0, &track, 0, 1, &mut reader, &|| false).unwrap();
    let before = index.bootstrap(0, 0).unwrap();
    let mut failed = bytes.clone();
    let at = track.samples[1].bytes;
    failed[(at.offset + at.length - 33) as usize] = 0xff;
    let error = index.scan(
        0,
        &track,
        1,
        1,
        &mut |offset, out: &mut [u8]| {
            out.copy_from_slice(&failed[offset as usize..offset as usize + out.len()]);
            Ok(out.len())
        },
        &|| false,
    );
    assert!(error.is_err());
    assert_eq!(index.sample(0, 1).unwrap_err().kind, ErrorKind::State);
    assert_eq!(index.bootstrap(0, 0).unwrap(), before);
    assert_eq!(
        index.scan(0, &track, 1, 1, &mut reader, &|| false).unwrap(),
        2
    );
    assert_eq!(
        index
            .sample(0, 1)
            .unwrap()
            .1
            .unwrap()
            .sequence
            .progressive_chroma_location()
            .unwrap(),
        Some("center")
    );
}
#[test]
fn cache_and_request_limits_and_malformed_descriptions_are_explicit() {
    let (track, _) = fixture();
    let mut index = AvcIndex::new(1);
    assert_eq!(
        index
            .scan(
                0,
                &track,
                0,
                1,
                &mut |_, _: &mut [u8]| panic!("budget should precede read"),
                &|| false
            )
            .unwrap_err()
            .kind,
        ErrorKind::Limit
    );
    for (through, count) in [(4, 1), (0, 0), (0, 2049)] {
        assert_eq!(
            index
                .scan(
                    0,
                    &track,
                    through,
                    count,
                    &mut |_, _: &mut [u8]| panic!("invalid request"),
                    &|| false
                )
                .unwrap_err()
                .kind,
            ErrorKind::Invalid
        );
    }
    let mut e = entry(LEFT);
    let duplicate = e.payload[78..].to_vec();
    e.payload.extend(duplicate);
    assert_eq!(avcc(&e).unwrap_err().kind, ErrorKind::Invalid);
    e.payload.truncate(78);
    assert!(avcc(&e).is_err());
    e.format = *b"hvc1";
    assert_eq!(avcc(&e).unwrap_err().kind, ErrorKind::Unsupported);
}
