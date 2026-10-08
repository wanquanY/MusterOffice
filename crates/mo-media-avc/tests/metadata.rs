mod support;
use mo_media::ErrorKind;
use mo_media_avc::{AvcInspector, ReadBudget};
use support::*;

fn inspector() -> AvcInspector {
    AvcInspector::from_avcc(&config(&[sps(0, 1, None, false)], &[pps(0, 0)])).unwrap()
}
#[test]
fn normative_defaults_do_not_invent_colour_primaries() {
    let picture = inspector()
        .inspect(&packet(&[slice(0, true, 0)]), &|| false)
        .unwrap()
        .unwrap();
    let info = picture.sequence;
    assert_eq!(info.visible, [0, 0, 128, 96]);
    assert_eq!(info.progressive_chroma_location().unwrap(), Some("left"));
    assert!(!info.chroma_location_present);
    assert!(!info.color.signal_present);
    assert!(!info.color.full_range);
    assert_eq!(
        (info.color.primaries, info.color.transfer, info.color.matrix),
        (2, 2, 2)
    );
    assert_eq!(info.reorder_frames, Some(2));
}
#[test]
fn all_chroma_locations_and_non_420_semantics() {
    for (code, label) in ["left", "center", "topLeft", "top", "bottomLeft", "bottom"]
        .into_iter()
        .enumerate()
    {
        let mut core = AvcInspector::from_avcc(&config(
            &[sps(0, 1, Some((code as u32, code as u32)), true)],
            &[pps(0, 0)],
        ))
        .unwrap();
        let info = core
            .inspect(&packet(&[slice(0, true, 0)]), &|| false)
            .unwrap()
            .unwrap()
            .sequence;
        assert_eq!(info.progressive_chroma_location().unwrap(), Some(label));
        assert_eq!(
            (info.color.primaries, info.color.transfer, info.color.matrix),
            (9, 16, 9)
        );
        assert!(info.color.full_range);
    }
    for (chroma, label) in [(0, None), (2, Some("left")), (3, None)] {
        let mut core =
            AvcInspector::from_avcc(&config(&[sps(0, chroma, Some((5, 1)), true)], &[pps(0, 0)]))
                .unwrap();
        let info = core
            .inspect(&packet(&[slice(0, true, 0)]), &|| false)
            .unwrap()
            .unwrap()
            .sequence;
        assert_eq!(info.progressive_chroma_location().unwrap(), label);
    }
    let mut core =
        AvcInspector::from_avcc(&config(&[sps(0, 1, Some((1, 2)), true)], &[pps(0, 0)])).unwrap();
    let info = core
        .inspect(&packet(&[slice(0, true, 0)]), &|| false)
        .unwrap()
        .unwrap()
        .sequence;
    assert_eq!(
        info.progressive_chroma_location().unwrap_err().kind,
        ErrorKind::Unsupported
    );
}
#[test]
fn slice_selects_pps_then_sps_instead_of_first_parameter_set() {
    let mut core = AvcInspector::from_avcc(&config(
        &[sps(0, 1, None, false), sps(4, 1, Some((2, 2)), true)],
        &[pps(0, 0), pps(37, 4)],
    ))
    .unwrap();
    let picture = core
        .inspect(&packet(&[slice(37, true, 0)]), &|| false)
        .unwrap()
        .unwrap();
    assert_eq!(picture.pps_id, 37);
    assert_eq!(picture.sequence.id, 4);
    assert_eq!(
        picture.sequence.progressive_chroma_location().unwrap(),
        Some("topLeft")
    );
    let first = core
        .inspect(&packet(&[slice(0, false, 1)]), &|| false)
        .unwrap()
        .unwrap();
    assert!(!first.sequence.color.signal_present);
    assert_ne!(first.sps_revision, picture.sps_revision);
}
#[test]
fn in_band_updates_are_transactional_and_bootstrap_restores_seek_state() {
    let mut core = inspector();
    let first = core
        .inspect(&packet(&[slice(0, true, 0)]), &|| false)
        .unwrap()
        .unwrap();
    let saved = core.clone();
    let update = sps(0, 1, Some((1, 1)), true);
    let mut invalid = packet(std::slice::from_ref(&update));
    invalid.extend_from_slice(&[0, 0, 0, 10, 0x65]);
    assert_eq!(
        core.inspect(&invalid, &|| false).unwrap_err().kind,
        ErrorKind::Invalid
    );
    assert_eq!(core.revision(), saved.revision());
    let changed = core
        .inspect(&packet(&[update.clone(), slice(0, true, 0)]), &|| false)
        .unwrap()
        .unwrap();
    assert_ne!(first.sps_revision, changed.sps_revision);
    let revision = core.revision();
    core.inspect(&packet(&[update]), &|| false).unwrap();
    assert_eq!(core.revision(), revision); // repeated identical headers are not changes
    let mut restarted = inspector();
    let mut first_packet = core.bootstrap().unwrap();
    first_packet.extend(packet(&[slice(0, true, 0)]));
    let restored = restarted
        .inspect(&first_packet, &|| false)
        .unwrap()
        .unwrap();
    assert_eq!(changed.sequence, restored.sequence);
    assert_eq!(
        saved
            .clone()
            .inspect(&packet(&[slice(0, true, 0)]), &|| false)
            .unwrap()
            .unwrap(),
        first
    );
}
#[test]
fn picture_boundaries_and_unknown_references_fail_closed() {
    for nals in [
        vec![slice(99, true, 0)],
        vec![slice(0, true, 0), slice(0, false, 1)],
        vec![slice(0, true, 0), sps(0, 1, None, false)],
        vec![vec![0x74, 0]],
        vec![vec![0xe7]],
        vec![vec![0x05]],
        vec![vec![0x41, 0, 0, 1]],
    ] {
        assert!(inspector().inspect(&packet(&nals), &|| false).is_err());
    }
    let mut core = AvcInspector::from_avcc(&config(&[], &[pps(0, 31)])).unwrap();
    assert!(
        core.inspect(&packet(&[slice(0, true, 0)]), &|| false)
            .is_err()
    );
    assert!(
        inspector()
            .inspect(&packet(&[slice(0, true, 0), slice(0, true, 0)]), &|| false)
            .is_ok()
    );
}
#[test]
fn truncated_parameter_sets_and_reserved_locations_are_rejected() {
    let nal = sps(0, 1, None, true);
    for end in 0..nal.len() {
        assert!(
            AvcInspector::from_avcc(&config(&[nal[..end].to_vec()], &[pps(0, 0)])).is_err(),
            "prefix {end}"
        );
    }
    assert!(
        AvcInspector::from_avcc(&config(&[sps(0, 1, Some((6, 6)), true)], &[pps(0, 0)])).is_err()
    );
    let mut invalid = nal;
    invalid.push(0);
    assert!(AvcInspector::from_avcc(&config(&[invalid], &[])).is_err());
    let mut invalid = config(&[], &[]);
    invalid[4] = 0xfe;
    assert!(AvcInspector::from_avcc(&invalid).is_err());
}
#[test]
fn read_at_skips_payload_and_matches_complete_packet() {
    let mut body = slice(0, true, 0);
    body.resize(1 << 20, 0x55);
    let bytes = packet(&[sps(0, 1, Some((5, 5)), true), body]);
    let expected = inspector().inspect(&bytes, &|| false).unwrap();
    let mut budget = ReadBudget::new(4096, 100).unwrap();
    let mut reads = 0;
    let actual = inspector()
        .inspect_at(
            &mut |offset, out: &mut [u8]| {
                reads += 1;
                let size = out.len().min(7); // short reads are valid, zero is not
                out[..size].copy_from_slice(&bytes[offset as usize..offset as usize + size]);
                Ok(size)
            },
            0,
            bytes.len() as u64,
            &mut budget,
            &|| false,
        )
        .unwrap();
    assert_eq!(actual, expected);
    assert!(reads < 30);
    assert!(budget.remaining_bytes() > 3800);
    assert_eq!(budget.remaining_nals(), 98);
}
#[test]
fn read_budget_cancel_and_callback_failures_do_not_commit_updates() {
    let bytes = packet(&[sps(0, 1, Some((2, 2)), true), slice(0, true, 0)]);
    let mut core = inspector();
    let before = core.revision();
    let mut budget = ReadBudget::new(8, 100).unwrap();
    let result = core.inspect_at(
        &mut |offset, out: &mut [u8]| {
            out.copy_from_slice(&bytes[offset as usize..offset as usize + out.len()]);
            Ok(out.len())
        },
        0,
        bytes.len() as u64,
        &mut budget,
        &|| false,
    );
    assert_eq!(result.unwrap_err().kind, ErrorKind::Limit);
    assert_eq!(core.revision(), before);
    let mut budget = ReadBudget::new(4096, 100).unwrap();
    for (kind, mode) in [
        (ErrorKind::CallbackPanic, 0),
        (ErrorKind::CallbackContract, 1),
        (ErrorKind::Io, 2),
    ] {
        let result = core.inspect_at(
            &mut |_, _: &mut [u8]| match mode {
                0 => panic!("test callback"),
                1 => Ok(0),
                _ => Err(std::io::Error::other("test")),
            },
            0,
            bytes.len() as u64,
            &mut budget,
            &|| false,
        );
        assert_eq!(result.unwrap_err().kind, kind);
        assert_eq!(core.revision(), before);
    }
    assert_eq!(
        core.inspect(&bytes, &|| true).unwrap_err().kind,
        ErrorKind::Cancelled
    );
    assert_eq!(core.revision(), before);
}
