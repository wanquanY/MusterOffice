import type { InputAudioTrack } from 'mediabunny';
import { parseAacAudioSpecificConfig, aacFrequencyTable } from '@musteroffice/mediabunny-aac';
import { BrowserMediaError, requireMedia, type BrowserMediaLimits } from './contract.js';

export interface AacCapacity {
  config: AudioDecoderConfig;
  description: Uint8Array<ArrayBuffer>;
  sampleRate: number;
  channels: number;
  frameSamples: number;
  frameBytes: number;
  decoderWeight: number;
}
/** Explicit admission profile for raw ISO BMFF AAC-LC. ASC is authoritative;
 * WebCodecs ignores config sampleRate/channels when description is supplied.
 * Other configurations require their own capacity and channel-layout contract. */
export async function aacCapacity(track: InputAudioTrack, limits: BrowserMediaLimits): Promise<AacCapacity> {
  const config = await track.getDecoderConfig();
  requireMedia(await track.getCodec() === 'aac' && config, 'unsupported_codec');
  return aacConfiguration(config, limits);
}
export function aacConfiguration(config: AudioDecoderConfig, limits: BrowserMediaLimits): AacCapacity {
  requireMedia(config.description && /^mp4a\.40\.0?2$/.test(config.codec), 'unsupported_codec');
  const source = ArrayBuffer.isView(config.description)
    ? new Uint8Array(config.description.buffer, config.description.byteOffset, config.description.byteLength)
    : new Uint8Array(config.description);
  requireMedia(source.length >= 2 && source.length <= 4096, 'resource_limit');
  const description = new Uint8Array(source);
  let asc: ReturnType<typeof parseAacAudioSpecificConfig>;
  try { asc = parseAacAudioSpecificConfig(description); } catch { throw new BrowserMediaError('unsupported_media'); }
  requireMedia(asc.objectType === 2 && asc.coreObjectType === 2 && asc.frequencyIndex < 13
    && (asc.channelConfiguration === 1 || asc.channelConfiguration === 2), 'unsupported_codec');
  const sampleRate = aacFrequencyTable[asc.frequencyIndex]!;
  requireMedia(asc.outputSampleRate === sampleRate && asc.outputNumberOfChannels === asc.channelConfiguration, 'unsupported_codec');
  // For this fixed ASC prefix, the final bits of byte 1 are GASpecificConfig:
  // frameLengthFlag, dependsOnCoreCoder, extensionFlag. Dependent/error resilient
  // profiles, PCE channel maps and SBR/PS are not admitted through AAC-LC.
  requireMedia((description[1]! & 3) === 0, 'unsupported_codec');
  const frameSamples = description[1]! & 4 ? 960 : 1024, channels = asc.channelConfiguration;
  requireMedia(sampleRate >= 8000 && sampleRate <= 192000, 'unsupported_codec');
  requireMedia(sampleRate <= limits.maxAudioSampleRate
    && channels <= limits.maxAudioChannels && frameSamples * channels * 4 <= limits.maxAudioBlockBytes, 'resource_limit');
  requireMedia(sampleRate === config.sampleRate && channels === config.numberOfChannels, 'unsupported_media');
  const frameBytes = frameSamples * channels * 4;
  // Application weight, not a claim about the browser's opaque codec heap.
  // Includes overlap/transform working room separately from pending outputs.
  return { config: { ...config, description }, description, sampleRate, channels, frameSamples, frameBytes,
    decoderWeight: (1 << 20) + frameBytes * 4 };
}
