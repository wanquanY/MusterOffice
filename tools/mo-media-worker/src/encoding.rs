use crate::wire::Wire;
use mo_media::{EncodeConfig, EncodeLimits, EncodeVideoFrame, ErrorKind, MediaError};
use mo_media_ffmpeg::Encoder;
use mo_media_wire::{Event, encode_audio_bytes, encode_video_lengths, read_encode_bytes};
use std::{cell::RefCell, rc::Rc};
pub struct Encoding {
    encoder: Encoder<'static>,
    config: EncodeConfig,
    limits: EncodeLimits,
}
fn wire_error(error: std::io::Error) -> MediaError {
    MediaError::new(ErrorKind::Invalid, error.to_string())
}
impl Encoding {
    pub fn open(
        io: Rc<RefCell<Wire>>,
        config: EncodeConfig,
        limits: EncodeLimits,
    ) -> Result<Self, MediaError> {
        let encoder = Encoder::open(
            move |offset, bytes: &[u8]| io.borrow_mut().write_candidate(offset, bytes),
            config,
            limits,
            || false,
        )?;
        Ok(Self {
            encoder,
            config,
            limits,
        })
    }
    pub fn video(
        &mut self,
        io: &Rc<RefCell<Wire>>,
        pts: i64,
        duration: i64,
    ) -> Result<Event, MediaError> {
        let lengths = encode_video_lengths(self.config, self.limits).map_err(wire_error)?;
        let bytes = read_encode_bytes(&mut io.borrow_mut().input, lengths.iter().sum())
            .map_err(wire_error)?;
        let (y, uv) = bytes.split_at(lengths[0]);
        let (u, v) = uv.split_at(lengths[1]);
        self.encoder.video(EncodeVideoFrame {
            pts,
            duration,
            planes: [y, u, v],
            strides: [
                self.config.width as usize,
                self.config.width as usize / 2,
                self.config.width as usize / 2,
            ],
        })?;
        Ok(Event::VideoEncoded)
    }
    pub fn audio(
        &mut self,
        io: &Rc<RefCell<Wire>>,
        first_sample: i64,
        frames: u32,
    ) -> Result<Event, MediaError> {
        let n = encode_audio_bytes(self.config, self.limits, frames).map_err(wire_error)?;
        let bytes = read_encode_bytes(&mut io.borrow_mut().input, n).map_err(wire_error)?;
        let samples = bytes
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes(b.try_into().expect("four byte chunk")))
            .collect::<Vec<_>>();
        self.encoder.audio(first_sample, &samples)?;
        Ok(Event::AudioEncoded)
    }
    pub fn finish(&mut self) -> Result<Event, MediaError> {
        self.encoder.finish().map(|stats| Event::Encoded { stats })
    }
}
