use mo_media::{DecodeLimits, FramePlane, MediaKind};
use mo_media_wire::{Event, Response, read_header, read_planes, validate_frame};
#[derive(Clone, Copy)]
pub(crate) struct Expected {
    pub call_id: u64,
    pub frame: Option<(MediaKind, u64)>,
}
#[derive(Debug)]
pub(crate) struct Reply {
    pub call_id: u64,
    pub event: Event,
    pub planes: Vec<FramePlane>,
}
pub(crate) fn read_reply(
    input: &mut impl std::io::Read,
    limits: Option<DecodeLimits>,
    expected: Expected,
) -> Result<Reply, String> {
    let response: Response = read_header(input)
        .map_err(|e| e.to_string())?
        .ok_or("missing worker response")?;
    if response.call_id != expected.call_id {
        return Err("worker response call id".into());
    }
    let planes = if let Event::Frame { info, planes } = &response.event {
        if expected.frame != Some((info.kind, info.generation)) {
            return Err("unsolicited or stale media frame".into());
        }
        let limits = limits.ok_or("frames not admitted by this session")?;
        validate_frame(info, planes, limits).map_err(|e| e.to_string())?;
        read_planes(input, planes, limits.max_frame_bytes).map_err(|e| e.to_string())?
    } else {
        Vec::new()
    };
    Ok(Reply {
        call_id: response.call_id,
        event: response.event,
        planes,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use mo_media::{CodecColorTags, FrameInfo, TimeBase};
    use mo_media_wire::{PlaneLayout, header_bytes};
    use std::io::{self, Cursor, Read};
    struct HeaderOnly(Cursor<Vec<u8>>);
    impl Read for HeaderOnly {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            assert!(
                self.0.position() < self.0.get_ref().len() as u64,
                "must reject before asking for frame bytes"
            );
            self.0.read(out)
        }
    }
    fn limits() -> DecodeLimits {
        DecodeLimits {
            max_input_bytes: 1 << 20,
            max_read_bytes: 1 << 20,
            max_frame_bytes: 1024,
            max_streams: 16,
            max_pixels: 4096,
            max_audio_samples: 4096,
            max_channels: 8,
            max_decode_steps: 1000,
        }
    }
    fn info() -> FrameInfo {
        FrameInfo {
            generation: 2,
            kind: MediaKind::Video,
            time_base: TimeBase {
                numerator: 1,
                denominator: 30,
            },
            pts_ticks: Some(8),
            duration_ticks: Some(1),
            width: Some(2),
            height: Some(2),
            sample_rate: None,
            channels: None,
            samples: None,
            format: "rgba".into(),
            channel_layout: String::new(),
            colors: CodecColorTags {
                primaries: 1,
                transfer: 1,
                matrix: 0,
                range: 2,
                chroma_location: 0,
            },
            sample_aspect_ratio: None,
            alpha_mode: 0,
            key_frame: true,
            interlaced: false,
            top_field_first: false,
        }
    }
    fn envelope(info: FrameInfo, planes: Vec<PlaneLayout>) -> Vec<u8> {
        header_bytes(&Response {
            call_id: 7,
            event: Event::Frame { info, planes },
        })
        .unwrap()
    }
    #[test]
    fn wrong_call_or_seek_generation_and_unsolicited_frames_are_rejected_before_payload() {
        for expected in [
            Expected {
                call_id: 6,
                frame: Some((MediaKind::Video, 2)),
            },
            Expected {
                call_id: 7,
                frame: Some((MediaKind::Video, 1)),
            },
            Expected {
                call_id: 7,
                frame: None,
            },
            Expected {
                call_id: 7,
                frame: Some((MediaKind::Audio, 2)),
            },
        ] {
            let mut input = HeaderOnly(Cursor::new(envelope(
                info(),
                vec![PlaneLayout {
                    row_bytes: 8,
                    rows: 2,
                }],
            )));
            assert!(read_reply(&mut input, Some(limits()), expected).is_err());
        }
    }
    #[test]
    fn metadata_and_total_plane_budgets_are_checked_before_payload() {
        let expected = Expected {
            call_id: 7,
            frame: Some((MediaKind::Video, 2)),
        };
        for plane in [
            PlaneLayout {
                row_bytes: u64::MAX,
                rows: 2,
            },
            PlaneLayout {
                row_bytes: 1025,
                rows: 1,
            },
            PlaneLayout {
                row_bytes: 8,
                rows: 0,
            },
        ] {
            assert!(
                read_reply(
                    &mut HeaderOnly(Cursor::new(envelope(info(), vec![plane]))),
                    Some(limits()),
                    expected
                )
                .is_err()
            );
        }
        let mut invalid = info();
        invalid.width = Some(999999);
        assert!(
            read_reply(
                &mut HeaderOnly(Cursor::new(envelope(
                    invalid,
                    vec![PlaneLayout {
                        row_bytes: 8,
                        rows: 2
                    }]
                ))),
                Some(limits()),
                expected
            )
            .is_err()
        );
    }
    #[test]
    fn complete_binary_frame_required_without_partial_success() {
        let expected = Expected {
            call_id: 7,
            frame: Some((MediaKind::Video, 2)),
        };
        let header = envelope(
            info(),
            vec![PlaneLayout {
                row_bytes: 8,
                rows: 2,
            }],
        );
        for n in 0..16 {
            let mut bytes = header.clone();
            bytes.extend(vec![7; n]);
            assert!(read_reply(&mut Cursor::new(bytes), Some(limits()), expected).is_err());
        }
        let mut bytes = header;
        bytes.extend([7; 16]);
        let frame = read_reply(&mut Cursor::new(bytes), Some(limits()), expected).unwrap();
        assert_eq!(frame.planes[0].bytes, [7; 16]);
    }
}
