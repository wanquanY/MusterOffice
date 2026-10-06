mod support;
use mo_media_isobmff::{IndexError, IndexLimits, IsoBmffIndex};
use std::{cell::Cell, io};
use support::*;

fn parse(data: &[u8]) -> Result<IsoBmffIndex, IndexError> {
    IsoBmffIndex::read(
        &mut |offset, out: &mut [u8]| {
            let offset = usize::try_from(offset).map_err(io::Error::other)?;
            let n = out.len().min(data.len().saturating_sub(offset)).min(13);
            if n != 0 {
                out[..n].copy_from_slice(&data[offset..offset + n]);
            }
            Ok(n)
        },
        data.len() as u64,
        IndexLimits::default(),
        &|| false,
    )
}
#[test]
fn signed_composition_times_chunks_descriptions_edits_and_matrix_remain_exact() {
    let f = signed_fixture();
    let index = parse(&f.file()).unwrap();
    assert_eq!(
        (index.movie_timescale, index.declared_movie_duration),
        (1000, Some(700))
    );
    let t = &index.tracks[0];
    assert_eq!(
        (
            t.id,
            t.timescale,
            t.decode_duration,
            t.declared_media_duration
        ),
        (7, 100, 40, Some(40))
    );
    assert_eq!(
        t.samples
            .iter()
            .map(|s| (
                s.dts,
                s.pts,
                s.bytes.offset,
                s.bytes.length,
                s.description,
                s.sync_sample
            ))
            .collect::<Vec<_>>(),
        vec![
            (0, 0, 32, 2, 0, true),
            (10, 30, 34, 2, 0, false),
            (20, 10, 36, 2, 1, true),
            (30, 20, 38, 2, 1, false)
        ]
    );
    assert_eq!(
        t.edits
            .iter()
            .map(|e| (e.movie_start, e.movie_duration, e.media_time, e.rate_16_16))
            .collect::<Vec<_>>(),
        vec![
            (0, 100, None, 65536),
            (100, 400, Some(0), 65536),
            (500, 200, Some(20), 0)
        ]
    );
    assert_eq!(t.descriptions[0].format, *b"avc1");
    assert_eq!(t.display_matrix, [65536, 0, 0, 0, 65536, 0, 0, 0, 1 << 30]);
    let default = parse(&Fixture::default().file()).unwrap();
    assert_eq!(
        default.tracks[0].samples[2].pts,
        20 + i64::from((-10i32) as u32),
        "version zero CTS is unsigned"
    );
}
#[test]
fn large_header_co64_and_sparse_multigigabyte_media_are_never_buffered() {
    let mut f = signed_fixture();
    f.version = 1;
    f.edits = table(
        b"elst",
        1,
        &[edit(1, 1 << 33, -1, 65536), edit(1, 400, 1 << 34, -65536)],
    );
    let payload = 1u64 << 33;
    f.replace(
        b"stco",
        table(
            b"co64",
            0,
            &[32u64.to_be_bytes().to_vec(), 36u64.to_be_bytes().to_vec()],
        ),
    );
    let mut header = 1u32.to_be_bytes().to_vec();
    header.extend_from_slice(b"mdat");
    header.extend_from_slice(&(payload + 16).to_be_bytes());
    let movie = f.movie();
    let segments = [(0, header), (payload + 16, movie)];
    let length = payload + 16 + segments[1].1.len() as u64;
    let index = IsoBmffIndex::read(
        &mut |offset, out: &mut [u8]| {
            let (base, data) = segments
                .iter()
                .find(|(a, b)| offset >= *a && offset < *a + b.len() as u64)
                .expect("must not read mdat payload");
            let at = (offset - base) as usize;
            let n = out.len().min(data.len() - at).min(7);
            out[..n].copy_from_slice(&data[at..at + n]);
            Ok(n)
        },
        length,
        IndexLimits::default(),
        &|| false,
    )
    .unwrap();
    assert!(index.bytes_read < 1024);
    assert_eq!(index.tracks[0].edits[1].movie_start, 1 << 33);
    assert_eq!(index.tracks[0].edits[1].media_time, Some(1 << 34));
    assert_eq!(index.tracks[0].edits[1].rate_16_16, -65536);
}
#[test]
fn compact_and_fixed_size_samples_and_absent_sync_table() {
    for bits in [4u8, 8, 16] {
        let mut f = signed_fixture();
        let mut sizes = vec![0, 0, 0, 0, 0, 0, 0, bits];
        sizes.extend_from_slice(&4u32.to_be_bytes());
        sizes.extend_from_slice(match bits {
            4 => &[0x22, 0x22][..],
            8 => &[2, 2, 2, 2][..],
            _ => &[0, 2, 0, 2, 0, 2, 0, 2][..],
        });
        f.replace(b"stsz", atom(b"stz2", &sizes));
        f.stbl.retain(|b| &b[4..8] != b"stss");
        let index = parse(&f.file()).unwrap();
        assert!(
            index.tracks[0]
                .samples
                .iter()
                .all(|s| s.sync_sample && s.bytes.length == 2)
        );
    }
    let mut f = signed_fixture();
    f.replace(b"stsz", atom(b"stsz", &words(&[0, 2, 4])));
    assert_eq!(parse(&f.file()).unwrap().tracks[0].samples.len(), 4);
}
#[test]
fn malformed_tables_and_external_references_fail_without_partial_index() {
    let mut cases = Vec::new();
    let mut f = signed_fixture();
    f.replace(b"stts", table(b"stts", 0, &[words(&[5, 10])]));
    cases.push(f.file());
    let mut f = signed_fixture();
    f.replace(b"ctts", table(b"ctts", 1, &[words(&[3, 0])]));
    cases.push(f.file());
    let mut f = signed_fixture();
    f.replace(b"stsc", table(b"stsc", 0, &[words(&[2, 2, 1])]));
    cases.push(f.file());
    let mut f = signed_fixture();
    f.replace(b"stsc", table(b"stsc", 0, &[words(&[1, 2, 3])]));
    cases.push(f.file());
    let mut f = signed_fixture();
    f.replace(b"stco", table(b"stco", 0, &[words(&[39]), words(&[36])]));
    cases.push(f.file());
    let mut f = signed_fixture();
    f.replace(b"stss", table(b"stss", 0, &[words(&[2]), words(&[2])]));
    cases.push(f.file());
    let mut f = signed_fixture();
    f.stbl.push(f.stbl[1].clone());
    cases.push(f.file());
    let mut f = signed_fixture();
    f.dref = table(
        b"dref",
        0,
        &[atom(b"url ", b"\0\0\0\0https://example.test/secret\0")],
    );
    cases.push(f.file());
    let mut f = signed_fixture();
    f.stbl.push(atom(b"cslg", &[0; 24]));
    cases.push(f.file());
    let mut f = signed_fixture();
    f.edits = table(
        b"elst",
        1,
        &[edit(1, u64::MAX, 0, 65536), edit(1, 1, 0, 65536)],
    );
    cases.push(f.file());
    let file = signed_fixture().file();
    cases.push([file.clone(), atom(b"moof", &[])].concat());
    cases.push([file, signed_fixture().movie()].concat());
    for (i, bytes) in cases.iter().enumerate() {
        assert!(parse(bytes).is_err(), "case {i}");
    }
}
#[test]
fn every_truncation_and_structural_bit_flip_is_bounded_without_parser_panics() {
    let file = signed_fixture().file();
    for end in 0..file.len() {
        assert!(parse(&file[..end]).is_err(), "truncation {end}");
    }
    for index in 0..file.len() {
        for mask in [1, 128] {
            let mut changed = file.clone();
            changed[index] ^= mask;
            assert!(
                !matches!(parse(&changed), Err(IndexError::CallbackPanic)),
                "panic at {index}"
            );
        }
    }
}
#[test]
fn budgets_cancel_and_bad_callbacks_never_become_success() {
    let file = signed_fixture().file();
    let run = |limits, check: &dyn Fn() -> bool| {
        IsoBmffIndex::read(
            &mut |offset, out: &mut [u8]| {
                let at = offset as usize;
                let n = out.len().min(file.len().saturating_sub(at));
                out[..n].copy_from_slice(&file[at..at + n]);
                Ok(n)
            },
            file.len() as u64,
            limits,
            check,
        )
    };
    let count = Cell::new(0);
    run(IndexLimits::default(), &|| {
        count.set(count.get() + 1);
        false
    })
    .unwrap();
    for stop in 0..count.get() {
        let calls = Cell::new(0);
        assert!(matches!(
            run(IndexLimits::default(), &|| {
                let n = calls.get();
                calls.set(n + 1);
                n >= stop
            }),
            Err(IndexError::Cancelled)
        ));
    }
    for limits in [
        IndexLimits {
            max_samples: 3,
            ..IndexLimits::default()
        },
        IndexLimits {
            max_metadata_bytes: 8,
            ..IndexLimits::default()
        },
        IndexLimits {
            max_boxes: 2,
            ..IndexLimits::default()
        },
        IndexLimits {
            max_edits: 2,
            ..IndexLimits::default()
        },
        IndexLimits {
            max_sample_descriptions: 1,
            ..IndexLimits::default()
        },
    ] {
        assert!(matches!(run(limits, &|| false), Err(IndexError::Budget)));
    }
    assert!(matches!(
        IsoBmffIndex::read(
            &mut |_, _: &mut [u8]| -> io::Result<usize> { panic!("source callback") },
            file.len() as u64,
            IndexLimits::default(),
            &|| false
        ),
        Err(IndexError::CallbackPanic)
    ));
    assert!(matches!(
        IsoBmffIndex::read(
            &mut |_, out: &mut [u8]| Ok(out.len() + 1),
            file.len() as u64,
            IndexLimits::default(),
            &|| false
        ),
        Err(IndexError::Invalid(_))
    ));
}

#[test]
fn generated_chunk_layouts_match_an_independent_expanded_mapping() {
    let mut seed = 0x7be12u32;
    for _ in 0..256 {
        let mut next = || {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            seed
        };
        let chunk_count = (next() % 12 + 1) as usize;
        let mut stsc = Vec::new();
        let mut offsets = Vec::new();
        let mut sizes = Vec::new();
        let mut deltas = Vec::new();
        let mut composition = Vec::new();
        let mut expected = Vec::new();
        let mut offset = 32u32;
        let mut dts = 0i64;
        for chunk in 0..chunk_count {
            let n = next() % 5 + 1;
            let description = next() % 2 + 1;
            stsc.push(words(&[chunk as u32 + 1, n, description]));
            offsets.push(words(&[offset]));
            for _ in 0..n {
                let size = next() % 12 + 1;
                let delta = next() % 100 + 1;
                let cts = (next() % 201) as i32 - 100;
                sizes.push(size);
                deltas.push(words(&[1, delta]));
                composition.push(words(&[1, cts as u32]));
                expected.push((
                    u64::from(offset),
                    u64::from(size),
                    dts,
                    dts + i64::from(cts),
                    description - 1,
                ));
                offset += size;
                dts += i64::from(delta);
            }
        }
        let mut f = signed_fixture();
        f.replace(
            b"stsz",
            atom(
                b"stsz",
                &words(&[vec![0, 0, sizes.len() as u32], sizes].concat()),
            ),
        );
        f.replace(b"stts", table(b"stts", 0, &deltas));
        f.replace(b"ctts", table(b"ctts", 1, &composition));
        f.replace(b"stco", table(b"stco", 0, &offsets));
        f.replace(b"stsc", table(b"stsc", 0, &stsc));
        f.stbl.retain(|b| &b[4..8] != b"stss");
        let file = [
            atom(b"ftyp", b"isom\0\0\0\0isomiso2"),
            atom(b"mdat", &vec![0; (offset - 32) as usize]),
            f.movie(),
        ]
        .concat();
        let index = parse(&file).unwrap();
        let track = &index.tracks[0];
        assert_eq!(track.decode_duration, dts as u64);
        assert_eq!(
            track
                .samples
                .iter()
                .map(|s| (s.bytes.offset, s.bytes.length, s.dts, s.pts, s.description))
                .collect::<Vec<_>>(),
            expected
        );
    }
}
