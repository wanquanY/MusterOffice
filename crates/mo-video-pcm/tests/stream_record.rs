#[path = "stream/support.rs"]
mod fixture;
use fixture::*;
use mo_common::{Digest, ResourceId};
use mo_video_model::StreamId;
use mo_video_pcm::*;

#[test]
fn verified_record_restores_exact_clock_and_source_without_a_live_decoder() {
    let (complete, tiles) = streamed(8199, &[1, 997, 13], 2);
    let bytes = serde_json::to_vec(&complete.record().unwrap()).unwrap();
    assert!(bytes.len() < 2048);
    let record: PcmStreamRecord = serde_json::from_slice(&bytes).unwrap();
    let expected = complete.source().identity().clone();
    let available = complete.decoded().decoded_range(256).unwrap();
    let refs = tiles
        .iter()
        .map(|t| complete.seal_ref(&t.reference).unwrap())
        .collect::<Vec<_>>();
    drop(complete);
    let mut imported = origin();
    imported.resource = ResourceId::new("other-project-resource").unwrap();
    let (decoded, source) = record
        .restore(
            &imported,
            PcmStreamLimits {
                tile_frames: 256,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(source.identity(), &expected);
    assert_eq!(source.binding().unwrap().resource, imported.resource);
    assert_eq!(decoded.decoded_range(256).unwrap(), available);
    for reference in refs {
        source.validate_ref(&reference).unwrap();
    }
}
#[test]
fn wrong_origin_malformed_completion_or_lower_limits_cannot_restore() {
    let (complete, _) = streamed(513, &[257], 2);
    let record = complete.record().unwrap();
    let limits = PcmStreamLimits {
        tile_frames: 256,
        ..Default::default()
    };
    for kind in 0..3 {
        let mut origin = origin();
        match kind {
            0 => origin.content = Digest::from_sha256([9; 32]),
            1 => origin.stream = StreamId::new("track-9").unwrap(),
            _ => origin.decoder_profile = Digest::from_sha256([9; 32]),
        }
        assert!(record.restore(&origin, limits).is_err());
    }
    let original = serde_json::to_value(&record).unwrap();
    for (key, value) in [
        ("sampleFrames", serde_json::json!("0")),
        ("sampleFrames", serde_json::json!("512")),
        ("decodeFrames", serde_json::json!(0)),
        ("decodeFrames", serde_json::json!(514)),
        ("sampleRate", serde_json::json!(16000)),
        ("layout", serde_json::json!(["frontLeft", "frontLeft"])),
        ("sampleZero", serde_json::json!({"ticks":"0","timescale":1})),
    ] {
        let mut changed = original.clone();
        changed[key] = value;
        let changed: PcmStreamRecord = serde_json::from_value(changed).unwrap();
        assert!(changed.restore(&origin(), limits).is_err(), "{key}");
    }
    for key in ["format", "unexpected"] {
        let mut changed = original.clone();
        changed[key] = serde_json::json!("unsupported");
        assert!(serde_json::from_value::<PcmStreamRecord>(changed).is_err());
    }
    for kind in 0..4 {
        let mut low = limits;
        match kind {
            0 => low.max_tiles = 2,
            1 => low.tile_frames = 512,
            2 => low.inspection.max_sample_frames = 512,
            _ => low.inspection.max_decode_frames = 1,
        }
        assert!(record.restore(&origin(), low).is_err());
    }
}
