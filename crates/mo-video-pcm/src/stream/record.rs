use super::*;
use mo_common::{RationalTime, Ticks};
use mo_time::ExactRatio;
use mo_video_audio::{AudioAsset, Channel, ChannelLayout, PcmRange};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
enum Format {
    #[serde(rename = "musteroffice.pcm.stream/1-draft")]
    V1,
}
/// Compact immutable completion metadata. Hosts must deserialize within their
/// metadata byte limit and load only from their authorized, committed content
/// identity. Arbitrary JSON, uncommitted stages or file presence are NOT proof
/// of successful EOF, stored tiles or permissions. Missing tiles still fail in
/// PcmTileReader; no samples are reconstructed or synthesized by this record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PcmStreamRecord {
    format: Format,
    content: Digest,
    stream: StreamId,
    decoder_profile: Digest,
    tile_frames: u32,
    sample_rate: u32,
    layout: Vec<Channel>,
    sample_zero: RationalTime,
    sample_frames: Ticks,
    decode_frames: u32,
    source: Digest,
}
impl CompletedPcmStream {
    /// Serialize/publish with the complete tile inventory in the host's atomic
    /// completion. The record has no paths, tenant IDs or access tokens.
    pub fn record(&self) -> Result<PcmStreamRecord> {
        let binding = self
            .source
            .binding()
            .ok_or(PcmError::Invalid("decoded PCM record binding"))?;
        let asset = self.decoded.asset();
        Ok(PcmStreamRecord {
            format: Format::V1,
            content: binding.digest.clone(),
            stream: binding.stream.clone(),
            decoder_profile: self.decoder_profile.clone(),
            tile_frames: self.source.tile_frames(),
            sample_rate: asset.sample_rate,
            layout: asset.layout.channels().to_vec(),
            sample_zero: asset.sample_zero.to_time_exact()?,
            sample_frames: Ticks::new(asset.available.end),
            decode_frames: u32::try_from(self.decoded.decode_frames())
                .map_err(|_| PcmError::Budget)?,
            source: self.source.identity().clone(),
        })
    }
}
impl PcmStreamRecord {
    /// Only for a completion record the HOST has verified and authorized. The
    /// resource ID may change between projects; content, stream, decoder profile
    /// and geometry may not. The returned inspection is historical evidence,
    /// not a claim that this call decoded or read any PCM bytes.
    pub fn restore(
        &self,
        origin: &PcmStreamOrigin,
        limits: PcmStreamLimits,
    ) -> Result<(InspectedPcm, Arc<PcmSource>)> {
        limits.validate()?;
        if self.content != origin.content
            || self.stream != origin.stream
            || self.decoder_profile != origin.decoder_profile
            || self.tile_frames != limits.tile_frames
        {
            return Err(PcmError::Invalid("PCM completion record origin"));
        }
        if self.layout.is_empty() || self.layout.len() > 64 {
            return Err(PcmError::Invalid("PCM completion record channels"));
        }
        let asset = AudioAsset {
            sample_rate: self.sample_rate,
            layout: ChannelLayout::new(self.layout.clone())?,
            sample_zero: ExactRatio::from_time(self.sample_zero),
            available: PcmRange::new(0, self.sample_frames.get())?,
        };
        let decoded = InspectedPcm::restore(
            asset.clone(),
            u64::from(self.decode_frames),
            limits.inspection,
        )?;
        let binding = MediaBinding {
            resource: origin.resource.clone(),
            digest: origin.content.clone(),
            stream: origin.stream.clone(),
            descriptor: MediaStream::Audio {
                available: decoded.decoded_range(limits.inspection.exact_bits)?,
                sample_rate: asset.sample_rate,
                channels: asset.layout.len() as u16,
            },
        };
        let source = PcmSource::new(
            binding,
            asset,
            &origin.decoder_profile,
            self.tile_frames,
            limits.inspection.exact_bits,
        )?;
        if source.tile_count() > limits.max_tiles {
            return Err(PcmError::Budget);
        }
        if source.identity() != &self.source {
            return Err(PcmError::Invalid("PCM completion record identity"));
        }
        Ok((decoded, Arc::new(source)))
    }
}
