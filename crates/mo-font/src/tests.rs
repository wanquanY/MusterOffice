use super::*;
use std::cell::Cell;
const TTF: &[u8] = include_bytes!("../../../fixtures/fonts/owned.ttf");
const OTF: &[u8] = include_bytes!("../../../fixtures/fonts/owned.otf");
const TTC: &[u8] = include_bytes!("../../../fixtures/fonts/owned.ttc");
fn request(bytes: &[u8]) -> FontRequest {
    FontRequest {
        expected_sha256: Digest::from_sha256(Sha256::digest(bytes).into()),
        face_index: 0,
        characters: vec![FontCharacter {
            codepoint: 65,
            variation_selector: None,
        }],
    }
}
fn run(bytes: &[u8]) -> Result<FontInspection, FontError> {
    inspect(&request(bytes), bytes, FontLimits::default(), &|| false)
}
#[test]
fn verified_face_queries_reuse_immutable_bytes_and_keep_per_call_limits() {
    let q = request(TTF);
    let calls = Cell::new(0);
    let check = || {
        calls.set(calls.get() + 1);
        false
    };
    let face =
        VerifiedFont::load(&q.expected_sha256, 0, TTF, FontLimits::default(), &check).unwrap();
    let loading_checks = calls.get();
    assert_eq!(face.bytes().as_ptr(), TTF.as_ptr());
    assert!(face.metadata().coverage.is_empty());
    calls.set(0);
    let coverage = face.query(&q.characters, 1, &check).unwrap();
    assert_eq!(coverage[0].outcome, CoverageOutcome::Mapped { glyph_id: 2 });
    assert!(calls.get() < loading_checks);
    assert!(matches!(
        face.query(&q.characters, 0, &|| false),
        Err(FontError::Limit(_))
    ));
    assert!(matches!(
        face.query(&q.characters, 1, &|| true),
        Err(FontError::Cancelled)
    ));
    assert!(matches!(
        face.query(
            &[FontCharacter {
                codepoint: 0xd800,
                variation_selector: None
            }],
            1,
            &|| false
        ),
        Err(FontError::InvalidRequest(_))
    ));
    assert_eq!(
        face.query(&q.characters, 1, &|| false).unwrap()[0].outcome,
        coverage[0].outcome
    );
}
fn r16(b: &[u8], p: usize) -> usize {
    u16::from_be_bytes(b[p..p + 2].try_into().unwrap()) as usize
}
fn r32(b: &[u8], p: usize) -> usize {
    u32::from_be_bytes(b[p..p + 4].try_into().unwrap()) as usize
}
fn w16(b: &mut [u8], p: usize, v: u16) {
    b[p..p + 2].copy_from_slice(&v.to_be_bytes());
}
fn w32(b: &mut [u8], p: usize, v: u32) {
    b[p..p + 4].copy_from_slice(&v.to_be_bytes());
}
fn table(b: &[u8], tag: &[u8; 4]) -> usize {
    (0..r16(b, 4))
        .map(|i| 12 + i * 16)
        .find_map(|p| (&b[p..p + 4] == tag).then(|| r32(b, p + 8)))
        .unwrap()
}
fn checksum(b: &[u8]) -> u32 {
    b.chunks(4).fold(0_u32, |a, part| {
        let mut v = [0; 4];
        v[..part.len()].copy_from_slice(part);
        a.wrapping_add(u32::from_be_bytes(v))
    })
}
fn repair(b: &mut [u8]) {
    let head = table(b, b"head");
    w32(b, head + 8, 0);
    for i in 0..r16(b, 4) {
        let p = 12 + i * 16;
        let start = r32(b, p + 8);
        let end = start + r32(b, p + 12);
        let sum = checksum(&b[start..end]);
        w32(b, p + 4, sum);
    }
    let adjustment = 0xb1b0afba_u32.wrapping_sub(checksum(b));
    w32(b, head + 8, adjustment);
}
fn change(tag: &[u8; 4], edit: impl FnOnce(&mut [u8], usize)) -> Vec<u8> {
    let mut b = TTF.to_vec();
    let p = table(&b, tag);
    edit(&mut b, p);
    repair(&mut b);
    b
}
fn subtable(b: &[u8], cmap: usize, format: u16) -> usize {
    (0..r16(b, cmap + 2))
        .find_map(|i| {
            let off = cmap + r32(b, cmap + 8 + i * 8);
            (r16(b, off) == format as usize).then_some(off)
        })
        .unwrap()
}
#[test]
fn actual_ttf_cff_and_collection_metadata() {
    let t = run(TTF).unwrap();
    assert_eq!(t.units_per_em, 1000);
    assert_eq!(t.glyph_count, 7);
    assert_eq!(
        t.coverage[0].outcome,
        CoverageOutcome::Mapped { glyph_id: 2 }
    );
    assert_eq!(t.axes.len(), 2);
    assert_eq!(t.instances[1].coordinates_16_16, vec![700 << 16, 100 << 16]);
    assert_eq!(t.os2.unwrap().fs_type, 0x0104);
    assert!(t.names.iter().any(|n| n.text.is_none()));
    assert_eq!(run(OTF).unwrap().sfnt_version, u32::from_be_bytes(*b"OTTO"));
    for face_index in 0..2 {
        let mut q = request(TTC);
        q.face_index = face_index;
        let face = inspect(&q, TTC, FontLimits::default(), &|| false).unwrap();
        assert_eq!(face.face_count, 2);
        assert_eq!(face.face_index, face_index);
    }
}
#[test]
fn variation_mapping_never_silently_falls_back() {
    let mut q = request(TTF);
    q.characters = [None, Some(0xfe00), Some(0xfe01), Some(0xfe02), Some(0x180f)]
        .into_iter()
        .map(|variation_selector| FontCharacter {
            codepoint: 65,
            variation_selector,
        })
        .collect();
    let r = inspect(&q, TTF, FontLimits::default(), &|| false).unwrap();
    assert_eq!(
        r.coverage
            .iter()
            .map(|c| c.outcome.clone())
            .collect::<Vec<_>>(),
        vec![
            CoverageOutcome::Mapped { glyph_id: 2 },
            CoverageOutcome::Mapped { glyph_id: 2 },
            CoverageOutcome::Mapped { glyph_id: 6 },
            CoverageOutcome::UnsupportedVariation,
            CoverageOutcome::UnsupportedVariation
        ]
    );
}
#[test]
fn resource_identity_and_request_scalars_checked() {
    let mut q = request(TTF);
    q.expected_sha256 = request(OTF).expected_sha256;
    assert!(matches!(
        inspect(&q, TTF, FontLimits::default(), &|| false),
        Err(FontError::ResourceConflict)
    ));
    for cp in [0xd800, 0xdfff, 0x110000, u32::MAX] {
        let mut q = request(TTF);
        q.characters[0].codepoint = cp;
        assert!(matches!(
            inspect(&q, TTF, FontLimits::default(), &|| false),
            Err(FontError::InvalidRequest(_))
        ));
    }
    let mut q = request(TTF);
    q.characters[0].variation_selector = Some(65);
    assert!(matches!(
        inspect(&q, TTF, FontLimits::default(), &|| false),
        Err(FontError::InvalidRequest(_))
    ));
}
#[test]
fn every_configurable_limit_is_enforced() {
    let d = FontLimits::default();
    for l in [
        FontLimits { max_bytes: 1, ..d },
        FontLimits { max_tables: 1, ..d },
        FontLimits { max_names: 1, ..d },
        FontLimits {
            max_name_bytes: 1,
            ..d
        },
        FontLimits { max_axes: 1, ..d },
        FontLimits {
            max_instances: 1,
            ..d
        },
        FontLimits {
            max_queries: 0,
            ..d
        },
    ] {
        assert!(matches!(
            inspect(&request(TTF), TTF, l, &|| false),
            Err(FontError::Limit(_))
        ));
    }
    assert!(matches!(
        inspect(
            &request(TTC),
            TTC,
            FontLimits { max_faces: 1, ..d },
            &|| false
        ),
        Err(FontError::Limit(_))
    ));
}
#[test]
fn cancellation_interrupts_multiple_stages() {
    for stop in [1, 4, 10, 20, 50] {
        let count = Cell::new(0);
        let r = inspect(&request(TTF), TTF, FontLimits::default(), &|| {
            let n = count.get() + 1;
            count.set(n);
            n >= stop
        });
        assert!(matches!(r, Err(FontError::Cancelled)), "{stop}");
    }
}
#[test]
fn all_prefix_truncations_are_errors_without_panics() {
    for end in 0..TTF.len() {
        assert!(run(&TTF[..end]).is_err(), "length {end}");
    }
}
#[test]
fn directory_checksums_and_table_overlap_are_rejected() {
    let mut bad = TTF.to_vec();
    let last = bad.len() - 1;
    bad[last] ^= 1;
    assert!(run(&bad).is_err());
    let mut overlap = TTF.to_vec();
    let first = r32(&overlap, 20);
    w32(&mut overlap, 36, first as u32);
    assert!(matches!(
        run(&overlap),
        Err(FontError::Invalid("overlapping font tables"))
    ));
    let mut adjustment = TTF.to_vec();
    let head = table(&adjustment, b"head");
    adjustment[head + 8] ^= 1;
    assert!(matches!(
        run(&adjustment),
        Err(FontError::Invalid("font checksum adjustment mismatch"))
    ));
}
#[test]
fn collection_face_and_other_directory_ranges_are_checked() {
    let mut q = request(TTC);
    q.face_index = 2;
    assert!(inspect(&q, TTC, FontLimits::default(), &|| false).is_err());
    let mut bad = TTC.to_vec();
    let face0 = r32(&bad, 12);
    let face1 = r32(&bad, 16);
    w32(&mut bad, face0 + 20, face1 as u32);
    assert!(matches!(
        run(&bad),
        Err(FontError::Invalid(
            "table outside data or overlaps directory"
        ))
    ));
}
#[test]
fn malformed_consumed_metadata_is_not_replaced() {
    let bad_name = change(b"name", |b, p| {
        let storage = p + r16(b, p + 4);
        let n = r16(b, p + 2);
        let rec = (0..n)
            .map(|i| p + 6 + i * 12)
            .find(|i| r16(b, *i) == 3)
            .unwrap();
        let at = storage + r16(b, rec + 10);
        w16(b, at, 0xd800);
    });
    assert!(matches!(
        run(&bad_name),
        Err(FontError::Invalid("malformed UTF-16 font name"))
    ));
    let bad_axis = change(b"fvar", |b, p| {
        let at = p + r16(b, p + 4);
        w32(b, at + 4, 1000 << 16);
    });
    assert!(matches!(
        run(&bad_axis),
        Err(FontError::Invalid("variation axis tag or range"))
    ));
    let duplicate = change(b"fvar", |b, p| {
        let at = p + r16(b, p + 4);
        let tag = b[at..at + 4].to_vec();
        b[at + 20..at + 24].copy_from_slice(&tag);
    });
    assert!(run(&duplicate).is_err());
}
#[test]
fn malformed_cmap_cannot_masquerade_as_missing_glyph() {
    let bad = change(b"cmap", |b, p| {
        let t = subtable(b, p, 12);
        w32(b, t + 24, 50000);
    });
    assert!(matches!(
        run(&bad),
        Err(FontError::Invalid("cmap glyph outside glyph set"))
    ));
    let bad = change(b"cmap", |b, p| {
        let t = subtable(b, p, 12);
        w32(b, t + 20, 0x110000);
    });
    assert!(run(&bad).is_err());
    let bad = change(b"cmap", |b, p| {
        let t = subtable(b, p, 14);
        w32(b, t + 17, 1);
    });
    assert!(run(&bad).is_err());
}
#[test]
fn legacy_charmaps_are_not_implicit_unicode_fallbacks() {
    let legacy = change(b"cmap", |b, p| {
        for i in 0..r16(b, p + 2) {
            w16(b, p + 4 + i * 8, 1);
        }
    });
    let r = run(&legacy).unwrap();
    assert!(r.cmap.is_none());
    assert_eq!(r.coverage[0].outcome, CoverageOutcome::NoUnicodeCmap);
}

#[test]
fn name_format_one_preserves_language_tag_identity() {
    // Replace only the name table using a rebuilt SFNT directory below.
    let offset = table(TTF, b"name");
    let record_count = r16(TTF, offset + 2);
    let storage = r16(TTF, offset + 4);
    let table_len = (0..r16(TTF, 4))
        .map(|i| 12 + i * 16)
        .find_map(|p| (&TTF[p..p + 4] == b"name").then(|| r32(TTF, p + 12)))
        .unwrap();
    let mut name = TTF[offset..offset + storage].to_vec();
    w16(&mut name, 0, 1);
    w16(&mut name, 4, (storage + 6) as u16);
    let record = (0..record_count)
        .map(|i| 6 + i * 12)
        .find(|i| r16(&name, *i) == 3)
        .unwrap();
    w16(&mut name, record + 4, 0x8000);
    name.extend_from_slice(&1_u16.to_be_bytes());
    name.extend_from_slice(&4_u16.to_be_bytes());
    name.extend_from_slice(&((table_len - storage) as u16).to_be_bytes());
    name.extend_from_slice(&TTF[offset + storage..offset + table_len]);
    name.extend_from_slice(&[0, b'e', 0, b'n']);
    let count = r16(TTF, 4);
    let mut bytes = TTF[..12 + count * 16].to_vec();
    for i in 0..count {
        while !bytes.len().is_multiple_of(4) {
            bytes.push(0);
        }
        let p = 12 + i * 16;
        let old = r32(TTF, p + 8);
        let len = r32(TTF, p + 12);
        let part = if &TTF[p..p + 4] == b"name" {
            name.as_slice()
        } else {
            &TTF[old..old + len]
        };
        let start = bytes.len();
        w32(&mut bytes, p + 8, start as u32);
        w32(&mut bytes, p + 12, part.len() as u32);
        bytes.extend_from_slice(part);
    }
    while !bytes.len().is_multiple_of(4) {
        bytes.push(0);
    }
    repair(&mut bytes);
    let result = run(&bytes).unwrap();
    assert_eq!(result.language_tags, vec!["en"]);
    assert!(result.names.iter().any(|n| n.language_id == 0x8000));
}
#[test]
fn untrusted_maximum_offsets_do_not_overflow_platform_width() {
    assert!(container::u16_at(&[], usize::MAX).is_err());
    assert!(container::u32_at(&[], usize::MAX).is_err());
    let b = change(b"cmap", |b, p| w32(b, p + 8, u32::MAX));
    assert!(run(&b).is_err());
}
