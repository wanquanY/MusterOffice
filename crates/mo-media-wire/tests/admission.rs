use mo_media_wire::{
    MAX_HEADER_BYTES, PlaneLayout, Request, Response, header_bytes, read_header, read_planes,
};
use std::io::Cursor;
#[test]
fn headers_are_bounded_before_payload_and_truncation_is_not_eof() {
    for length in [0, MAX_HEADER_BYTES as u32 + 1, u32::MAX] {
        let mut input = Cursor::new(length.to_le_bytes());
        assert!(read_header::<Response>(&mut input).is_err());
        assert_eq!(input.position(), 4);
    }
    for bytes in [vec![1], vec![5, 0, 0, 0, b'{']] {
        assert!(read_header::<Response>(&mut Cursor::new(bytes)).is_err());
    }
    assert!(
        read_header::<Response>(&mut Cursor::new([]))
            .unwrap()
            .is_none()
    );
    assert!(header_bytes(&"a".repeat(MAX_HEADER_BYTES)).is_err());
}
#[test]
fn duplicated_unknown_and_extra_protocol_fields_are_rejected() {
    for json in [
        r#"{"callId":1,"callId":2,"command":{"type":"next"}}"#,
        r#"{"callId":1,"command":{"type":"next"},"path":"/private"}"#,
        r#"{"callId":1,"command":{"type":"readReply","readId":1,"readId":2,"byteLength":4}}"#,
        r#"{"callId":1,"command":{"type":"openUrl","url":"https://example.com"}}"#,
    ] {
        let mut bytes = (json.len() as u32).to_le_bytes().to_vec();
        bytes.extend(json.as_bytes());
        assert!(
            read_header::<Request>(&mut Cursor::new(bytes)).is_err(),
            "{json}"
        );
    }
}
#[test]
fn aggregate_frame_budget_is_preflighted_before_reading_any_plane() {
    let mut input = Cursor::new(vec![3; 1024]);
    let planes = [
        PlaneLayout {
            row_bytes: 8,
            rows: 64,
        },
        PlaneLayout {
            row_bytes: 8,
            rows: 65,
        },
    ];
    assert!(read_planes(&mut input, &planes, 1024).is_err());
    assert_eq!(input.position(), 0);
    assert!(read_planes(&mut input, &[], 1024).is_err());
}
