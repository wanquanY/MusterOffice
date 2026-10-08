use super::*;
use js_sys::{Function, Uint8Array};
use mo_media::{CodecColorTags, DecodedFrame, FrameInfo, FramePlane, MediaKind, TimeBase};
use mo_video_audio::ByteOrder;
use mo_video_pcm::{
    CompletedPcmStream, MicrosecondPcmClock, MicrosecondPcmStream, UnsealedPcmTile,
    UnsealedPcmTileRef,
};

enum State {
    Decoding(MicrosecondPcmStream),
    Complete(CompletedPcmStream),
}
#[wasm_bindgen]
pub struct PcmDecode {
    state: Option<State>,
    options: Options,
    clock: MicrosecondPcmClock,
}
#[wasm_bindgen]
impl PcmDecode {
    pub fn open(options: &str) -> Result<Self> {
        let options = Options::read(options)?;
        let clock = MicrosecondPcmClock::new(options.raw_origin.clone(), options.sample_rate, 256)
            .map_err(failure)?;
        let stream = MicrosecondPcmStream::new(options.origin()?, 1, clock.clone(), limits())
            .map_err(failure)?;
        Ok(Self {
            state: Some(State::Decoding(stream)),
            options,
            clock,
        })
    }
    #[wasm_bindgen(js_name=packetTimestamp)]
    pub fn packet_timestamp(&self, ticks: i64, timescale: u32) -> Result<i64> {
        if !matches!(self.state, Some(State::Decoding(_))) || timescale == 0 {
            return Err(invalid());
        }
        let pts = ExactRatio::integer(ticks)
            .div(&ExactRatio::integer(i64::from(timescale)), 256)
            .map_err(|_| invalid())?;
        self.clock.packet_timestamp(&pts).map_err(failure)
    }
    /// `pcm` is interleaved f32 LITTLE-ENDIAN. Write is synchronous and must
    /// return true only after accepting this tile in the unpublished attempt.
    /// Promise/false/throw and every malformed frame poison the entire decode.
    pub fn push(
        &mut self,
        timestamp: i64,
        frames: u32,
        sample_rate: u32,
        channels: u32,
        pcm: &Uint8Array,
        write: &Function,
    ) -> Result<()> {
        let Some(State::Decoding(mut stream)) = self.state.take() else {
            return Err(invalid());
        };
        if frames == 0
            || frames > 8192
            || sample_rate != self.options.sample_rate
            || channels != self.options.channels
            || pcm.length() != frames * channels * 4
        {
            return Err(invalid());
        }
        let frame = DecodedFrame {
            info: FrameInfo {
                generation: 1,
                kind: MediaKind::Audio,
                time_base: TimeBase {
                    numerator: 1,
                    denominator: 1_000_000,
                },
                pts_ticks: Some(timestamp),
                duration_ticks: None,
                width: None,
                height: None,
                sample_rate: Some(sample_rate),
                channels: Some(channels),
                samples: Some(frames),
                format: "flt".into(),
                channel_layout: if channels == 1 { "mono" } else { "stereo" }.into(),
                colors: CodecColorTags {
                    primaries: 2,
                    transfer: 2,
                    matrix: 2,
                    range: 0,
                    chroma_location: 0,
                },
                sample_aspect_ratio: None,
                alpha_mode: 0,
                key_frame: true,
                interlaced: false,
                top_field_first: false,
            },
            planes: vec![FramePlane {
                row_bytes: pcm.length() as usize,
                rows: 1,
                bytes: pcm.to_vec(),
            }],
        };
        for tile in stream
            .push(&frame, ByteOrder::Little, &|| false)
            .map_err(failure)?
        {
            write_tile(tile, write)?;
        }
        self.state = Some(State::Decoding(stream));
        Ok(())
    }
    /// Successful flush/EOF is a host obligation. Returns computation metadata,
    /// not a storage commit. Seal each reference and atomically publish only
    /// after all tiles (including this call's final tail) were durably stored.
    pub fn finish(&mut self, write: &Function) -> Result<String> {
        let Some(State::Decoding(stream)) = self.state.take() else {
            return Err(invalid());
        };
        let mut complete = stream.finish_at_eof(&|| false).map_err(failure)?;
        if let Some(tail) = complete.tail.take() {
            write_tile(tail, write)?;
        }
        let record = json(&complete.record().map_err(failure)?)?;
        self.state = Some(State::Complete(complete));
        Ok(record)
    }
    pub fn seal(&self, reference: &str) -> Result<String> {
        let Some(State::Complete(complete)) = &self.state else {
            return Err(invalid());
        };
        let reference: UnsealedPcmTileRef = parse(reference, 4096)?;
        json(&complete.seal_ref(&reference).map_err(failure)?)
    }
    #[wasm_bindgen(js_name=retainedBytes)]
    pub fn retained_bytes(&self) -> usize {
        match &self.state {
            Some(State::Decoding(stream)) => stream.retained_bytes(),
            _ => 0,
        }
    }
}
fn write_tile(tile: UnsealedPcmTile, write: &Function) -> Result<()> {
    let accepted = write
        .call2(
            &JsValue::UNDEFINED,
            &JsValue::from_str(&json(&tile.reference)?),
            &Uint8Array::from(tile.bytes.as_slice()),
        )
        .map_err(|_| failure(PcmError::Store("host tile write failed".into())))?;
    if accepted.as_bool() != Some(true) {
        return Err(failure(PcmError::Store(
            "synchronous tile write acknowledgement required".into(),
        )));
    }
    Ok(())
}
