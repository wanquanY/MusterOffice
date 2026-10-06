use mo_media_wire::{Event, MAX_WRITE_BYTES, Response, read_encode_bytes, read_header};
#[derive(Debug)]
pub(super) struct Reply {
    pub event: Event,
    pub bytes: Vec<u8>,
}
pub(super) fn read(input: &mut impl std::io::Read, call: u64) -> Result<Reply, String> {
    let response: Response = read_header(input)
        .map_err(|e| e.to_string())?
        .ok_or("missing encoder response")?;
    if response.call_id != call {
        return Err("encoder response call id".into());
    }
    let bytes = match &response.event {
        Event::Write { byte_length, .. } if *byte_length > 0 && *byte_length <= MAX_WRITE_BYTES => {
            read_encode_bytes(input, *byte_length as usize).map_err(|e| e.to_string())?
        }
        Event::Error { .. }
        | Event::EncoderOpened { .. }
        | Event::VideoEncoded
        | Event::AudioEncoded
        | Event::Encoded { .. }
        | Event::Closed => Vec::new(),
        _ => return Err("unexpected encoder event or write payload size".into()),
    };
    Ok(Reply {
        event: response.event,
        bytes,
    })
}
