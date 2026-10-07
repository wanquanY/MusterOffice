mod support;
use mo_media_isobmff::{movie::*, presentation::*, *};
use mo_time::{ExactRatio as R, SampleSide as Side};
fn r(n: i64, d: u32) -> R {
    R::new(n.into(), d.into(), 256).unwrap()
}
fn track() -> TrackIndex {
    let data = support::signed_fixture().file();
    let mut index = IsoBmffIndex::read(
        &mut |offset, out: &mut [u8]| {
            let n = out.len().min(data.len() - offset as usize);
            out[..n].copy_from_slice(&data[offset as usize..offset as usize + n]);
            Ok(n)
        },
        data.len() as u64,
        IndexLimits::default(),
        &|| false,
    )
    .unwrap();
    index.tracks.remove(0)
}
#[test]
fn audio_and_video_share_the_same_exact_edit_mapping() {
    let mut track = track();
    track.timescale = 48000;
    track.handler = *b"soun";
    track.edits = vec![
        Edit {
            movie_start: 0,
            movie_duration: 1,
            media_time: None,
            rate_16_16: 65536,
        },
        Edit {
            movie_start: 1,
            movie_duration: 1001,
            media_time: Some(1024),
            rate_16_16: 65536,
        },
        Edit {
            movie_start: 1002,
            movie_duration: 500,
            media_time: Some(1024),
            rate_16_16: 98304,
        },
        Edit {
            movie_start: 1502,
            movie_duration: 200,
            media_time: Some(2048),
            rate_16_16: 0,
        },
    ];
    let audio =
        MovieTimeline::build(&track, 1000, &r(9, 1), TimingLimits::default(), &|| false).unwrap();
    track.handler = *b"vide";
    let video =
        MovieTimeline::build(&track, 1000, &r(9, 1), TimingLimits::default(), &|| false).unwrap();
    assert_eq!(audio.end(), &r(851, 500));
    assert_eq!(audio.track_id(), video.track_id());
    for (n, d, side, index, media) in [
        (0, 1, Side::After, 0, None),
        (1, 1000, Side::Before, 0, None),
        (1, 1000, Side::After, 1, Some(r(1024, 48000))),
        (501, 500, Side::After, 2, Some(r(1024, 48000))),
        (
            1252,
            1000,
            Side::After,
            2,
            Some(r(1024, 48000).add(&r(3, 8), 256).unwrap()),
        ),
        (1702, 1000, Side::Before, 3, Some(r(2048, 48000))),
    ] {
        let time = r(n, d);
        let a = audio.segment_at(&time, side, &|| false).unwrap().unwrap();
        let v = video.segment_at(&time, side, &|| false).unwrap().unwrap();
        assert_eq!(a.edit_index(), Some(index));
        assert_eq!(a.media_time(&time, 256).unwrap(), media);
        assert_eq!(
            a.media_time(&time, 256).unwrap(),
            v.media_time(&time, 256).unwrap()
        );
    }
    assert!(
        audio
            .segment_at(&r(851, 500), Side::After, &|| false)
            .unwrap()
            .is_none()
    );
}
#[test]
fn unedited_endpoint_is_explicit_and_zero_length_edits_preserve_original_indices() {
    let mut track = track();
    track.edits.clear();
    track.declared_track_duration = None;
    let end = r(1234567, 48000);
    let t = MovieTimeline::build(&track, 1000, &end, TimingLimits::default(), &|| false).unwrap();
    assert_eq!(t.end(), &end);
    assert_eq!(t.segments()[0].edit_index(), None);
    track.edits = vec![
        Edit {
            movie_start: 0,
            movie_duration: 0,
            media_time: Some(0),
            rate_16_16: 65536,
        },
        Edit {
            movie_start: 0,
            movie_duration: 1,
            media_time: Some(0),
            rate_16_16: 65536,
        },
    ];
    let t = MovieTimeline::build(&track, 1000, &end, TimingLimits::default(), &|| false).unwrap();
    assert_eq!(t.segments().len(), 1);
    assert_eq!(t.segments()[0].edit_index(), Some(1));
}
#[test]
fn bad_sequence_overflow_budget_and_cancellation_do_not_construct_a_timeline() {
    for kind in 0..6 {
        let mut track = track();
        let mut limits = TimingLimits::default();
        match kind {
            0 => track.edits[0].movie_start = 1,
            1 => track.edits[0].media_time = Some(-2),
            2 => track.edits[0].rate_16_16 = -1,
            3 => limits.max_edits = 0,
            4 => track.timescale = 0,
            _ => {
                track.edits[0].movie_duration = u64::MAX;
                track.edits[1].movie_start = u64::MAX;
            }
        }
        assert!(MovieTimeline::build(&track, 1000, &r(1, 1), limits, &|| false).is_err());
    }
    assert!(matches!(
        MovieTimeline::build(&track(), 1000, &r(1, 1), TimingLimits::default(), &|| true),
        Err(TimingError::Cancelled)
    ));
    assert!(matches!(
        MovieTimeline::build(
            &track(),
            1000,
            &r(1, 1),
            TimingLimits::default(),
            &|| panic!("host")
        ),
        Err(TimingError::CallbackPanic)
    ));
}
