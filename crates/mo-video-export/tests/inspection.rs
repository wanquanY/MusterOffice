use mo_media::{EncodeConfig, EncodeProfile};
use mo_media_isobmff::{ByteRange, Edit, IsoBmffIndex, Sample, SampleDescription, TrackIndex};
use mo_video_export::{InspectionError, inspect_browser_candidate};
fn config() -> EncodeConfig {
    EncodeConfig {
        profile: EncodeProfile::H264WebCodecsAacMp4,
        width: 128,
        height: 96,
        video_time_scale: 30000,
        frame_rate_num: 30000,
        frame_rate_den: 1001,
        video_duration: 3500,
        video_frames: 4,
        video_bit_rate: 4000000,
        gop_frames: 30,
        audio_sample_rate: 48000,
        audio_channels: 2,
        audio_bit_rate: 128000,
        audio_samples: 5600,
    }
}
fn track(id: u32, audio: bool) -> TrackIndex {
    let timescale = if audio { 48000 } else { 30000 };
    let duration = if audio { 6624 } else { 3500 };
    let period = if audio { 1024 } else { 1001 };
    let samples = (0..duration)
        .step_by(period as usize)
        .enumerate()
        .map(|(i, at)| Sample {
            bytes: ByteRange {
                offset: i as u64 * 64,
                length: 64,
            },
            dts: at as i64,
            pts: at as i64,
            decode_duration: (duration - at).min(period) as u32,
            description: 0,
            sync_sample: i == 0,
        })
        .collect();
    TrackIndex {
        id,
        flags: 3,
        handler: if audio { *b"soun" } else { *b"vide" },
        declared_track_duration: Some(28000),
        timescale,
        declared_media_duration: Some(duration),
        decode_duration: duration,
        display_matrix: [65536, 0, 0, 0, 65536, 0, 0, 0, 1073741824],
        display_size_16_16: [128 << 16, 96 << 16],
        descriptions: vec![SampleDescription {
            format: if audio { *b"mp4a" } else { *b"avc1" },
            payload: vec![],
        }],
        samples,
        edits: vec![Edit {
            movie_start: 0,
            movie_duration: 28000,
            media_time: Some(if audio { 1024 } else { 0 }),
            rate_16_16: 65536,
        }],
        auxiliary: vec![],
    }
}
fn index() -> IsoBmffIndex {
    IsoBmffIndex {
        movie_timescale: 240000,
        declared_movie_duration: Some(28000),
        tracks: vec![track(1, false), track(2, true)],
        media_data: vec![],
        bytes_read: 0,
    }
}
#[test]
fn fractional_clock_short_tail_and_explicit_priming_are_preserved() {
    let value = inspect_browser_candidate(&index(), config(), &|| false).unwrap();
    assert_eq!(value.movie_duration, "28000");
    assert_eq!(value.video_spots.len(), 3);
    assert_eq!(
        value.video_spots[2],
        mo_time::ExactRatio::new(1001.into(), 10000.into(), 256).unwrap()
    );
    assert_eq!(value.audio_track, Some(2));
    let mut c = config();
    c.audio_samples = 0;
    c.audio_channels = 0;
    let mut i = index();
    i.tracks.pop();
    assert_eq!(
        inspect_browser_candidate(&i, c, &|| false)
            .unwrap()
            .audio_track,
        None
    );
}
#[test]
fn corruption_in_every_clock_layer_and_packet_coverage_is_rejected() {
    let mutations: Vec<fn(&mut IsoBmffIndex)> = vec![
        |i| i.declared_movie_duration = Some(27999),
        |i| i.tracks[0].declared_track_duration = Some(27999),
        |i| i.tracks[0].declared_media_duration = Some(3499),
        |i| i.tracks[0].samples[1].pts = 0,
        |i| i.tracks[0].samples[1].dts = 0,
        |i| i.tracks[0].samples[3].decode_duration = 1001,
        |i| i.tracks[0].samples[0].sync_sample = false,
        |i| i.tracks[0].edits[0].rate_16_16 = 32768,
        |i| i.tracks[0].edits[0].media_time = Some(1),
        |i| i.tracks[0].display_matrix[0] = 32768,
        |i| i.tracks[0].display_size_16_16[0] = 64 << 16,
        |i| i.tracks[1].edits[0].media_time = Some(2048),
        |i| i.tracks[1].edits[0].media_time = None,
        |i| i.tracks[1].edits[0].movie_duration = 27999,
        |i| i.tracks[1].samples[2].pts += 1,
        |i| i.tracks[1].samples[2].decode_duration = 0,
        |i| i.tracks[1].samples[2].description = 1,
        |i| i.tracks[1].id = 1,
        |i| i.tracks[1].descriptions[0].format = *b"ac-3",
    ];
    for (n, mutate) in mutations.into_iter().enumerate() {
        let mut i = index();
        mutate(&mut i);
        assert!(
            inspect_browser_candidate(&i, config(), &|| false).is_err(),
            "mutation {n}"
        );
    }
}
#[test]
fn unrepresentable_clocks_profile_mismatch_and_cancellation_fail_closed() {
    let mut c = config();
    c.frame_rate_den = u32::MAX;
    c.video_time_scale = u32::MAX;
    c.frame_rate_num = 1;
    let mut i = index();
    i.tracks[0].timescale = c.video_time_scale;
    i.movie_timescale = c.video_time_scale;
    assert!(inspect_browser_candidate(&i, c, &|| false).is_err());
    assert!(matches!(
        inspect_browser_candidate(&index(), config(), &|| true),
        Err(InspectionError::Cancelled)
    ));
    c = config();
    c.profile = EncodeProfile::H264VideoToolboxAacMp4;
    assert!(matches!(
        inspect_browser_candidate(&index(), c, &|| false),
        Err(InspectionError::Unsupported)
    ));
}
