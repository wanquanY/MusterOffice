//! Byte framing before UTF-8/JSON interpretation. An invalid text line can be
//! rejected without losing the following frame. No rescanning of old bytes.
use std::io;
use tokio_util::{
    bytes::{Bytes, BytesMut},
    codec::Decoder,
};

#[derive(Default)]
pub(super) struct Frames {
    scanned: usize,
}
impl Decoder for Frames {
    type Item = Bytes;
    type Error = io::Error;
    fn decode(&mut self, input: &mut BytesMut) -> io::Result<Option<Bytes>> {
        let end = input.len().min(super::INPUT_BYTES + 1);
        if let Some(offset) = input[self.scanned..end].iter().position(|b| *b == b'\n') {
            let length = self.scanned + offset;
            let mut frame = input.split_to(length + 1).freeze();
            frame.truncate(length);
            self.scanned = 0;
            return Ok(Some(frame));
        }
        if input.len() > super::INPUT_BYTES {
            return Err(super::invalid("input frame byte budget"));
        }
        self.scanned = input.len();
        Ok(None)
    }
    fn decode_eof(&mut self, input: &mut BytesMut) -> io::Result<Option<Bytes>> {
        match self.decode(input)? {
            Some(frame) => Ok(Some(frame)),
            None if input.is_empty() => Ok(None),
            None => Err(super::invalid("unterminated input frame")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_limit_accepts_delayed_delimiter_without_consuming_next_frame() {
        let mut decoder = Frames::default();
        let mut input = BytesMut::from(vec![b' '; super::super::INPUT_BYTES].as_slice());
        assert!(decoder.decode(&mut input).unwrap().is_none());
        input.extend_from_slice(b"\nnext\n");
        assert_eq!(
            decoder.decode(&mut input).unwrap().unwrap().len(),
            super::super::INPUT_BYTES
        );
        assert_eq!(
            decoder.decode(&mut input).unwrap().unwrap().as_ref(),
            b"next"
        );
        assert!(decoder.decode_eof(&mut input).unwrap().is_none());
    }

    #[test]
    fn over_limit_delimiter_does_not_make_an_oversized_frame_legal() {
        let mut decoder = Frames::default();
        let mut input = BytesMut::from(vec![b' '; super::super::INPUT_BYTES + 1].as_slice());
        input.extend_from_slice(b"\n");
        assert!(decoder.decode(&mut input).is_err());
    }

    #[test]
    fn eof_retains_complete_frames_but_rejects_an_unterminated_tail() {
        let mut decoder = Frames::default();
        let mut input = BytesMut::from(&b"\nvalid\ntail"[..]);
        assert!(decoder.decode_eof(&mut input).unwrap().unwrap().is_empty());
        assert_eq!(
            decoder.decode_eof(&mut input).unwrap().unwrap().as_ref(),
            b"valid"
        );
        assert!(decoder.decode_eof(&mut input).is_err());
    }
}
