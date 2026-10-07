/** One explicit compatibility seam against the hash-pinned 1.61.3 source ABI.
 * Reuse its audited browser/codec corrections, not its fixed-size prefetch loop.
 * This module is internal to our independently bundled SDK, never a product API. */
import { VideoSampleSink, type InputVideoTrack, type VideoSample, type EncodedPacket } from 'mediabunny';
import { deserializeAvcDecoderConfigurationRecord, parseAvcSps, iterateAvcNalUnits, extractNalUnitTypeForAvc } from '@musteroffice/mediabunny-codec-data';
import { requireMedia } from './contract.js';
import { videoSurfaceBytes } from './video-budget.js';

interface SurfaceCapacity { surfaceBytes: number; pixels: number; references: number; reorder: number; }
export interface VideoCapacity extends SurfaceCapacity { validatePacket(data: Uint8Array): void; }
function avcSurface(bytes: Uint8Array): SurfaceCapacity {
  requireMedia(bytes.length <= 65536, 'resource_limit');
  const sps = parseAvcSps(bytes);
  requireMedia(sps && sps.frameMbsOnlyFlag === 1 && sps.chromaFormatIdc <= 3
    && sps.bitDepthLumaMinus8 <= 8 && sps.bitDepthChromaMinus8 <= 8, 'unsupported_codec');
  requireMedia(Number.isInteger(sps.maxDecFrameBuffering) && sps.maxDecFrameBuffering >= 0 && sps.maxDecFrameBuffering <= 16
    && Number.isInteger(sps.numReorderFrames) && sps.numReorderFrames >= 0 && sps.numReorderFrames <= sps.maxDecFrameBuffering, 'unsupported_media');
  return { surfaceBytes: videoSurfaceBytes(sps.codedWidth, sps.codedHeight,
    sps.bitDepthLumaMinus8 === 0 && sps.bitDepthChromaMinus8 === 0 ? 4 : 8),
    pixels: sps.codedWidth * sps.codedHeight, references: sps.maxDecFrameBuffering, reorder: sps.numReorderFrames };
}
export async function videoCapacity(track: InputVideoTrack): Promise<VideoCapacity> {
  const config = await track.getDecoderConfig();
  // Other codecs need their own certified access-unit / reference model. They
  // must not silently use the old unaccounted queue when byte admission fails.
  requireMedia(await track.getCodec() === 'avc' && config?.description, 'unsupported_codec');
  return avcCapacity(config);
}
/** Validates both initial and in-band parameter sets before decode. A changed
 * capacity needs a separately admitted decoder, never an implicit growth. */
export function avcCapacity(config: VideoDecoderConfig): VideoCapacity {
  requireMedia(config.description, 'unsupported_codec');
  const bytes = ArrayBuffer.isView(config.description)
    ? new Uint8Array(config.description.buffer, config.description.byteOffset, config.description.byteLength)
    : new Uint8Array(config.description);
  requireMedia(bytes.length <= 1024 * 1024, 'resource_limit');
  const record = deserializeAvcDecoderConfigurationRecord(bytes);
  requireMedia(record && record.sequenceParameterSets.length > 0 && record.sequenceParameterSets.length <= 32, 'unsupported_media');
  let surfaceBytes = 0, pixels = 0, references = 0, reorder = 0;
  for (const bytes of record.sequenceParameterSets) {
    const capacity = avcSurface(bytes);
    surfaceBytes = Math.max(surfaceBytes, capacity.surfaceBytes);
    pixels = Math.max(pixels, capacity.pixels);
    references = Math.max(references, capacity.references);
    reorder = Math.max(reorder, capacity.reorder);
  }
  return { surfaceBytes, pixels, references, reorder, validatePacket(data) {
    let end = 0, count = 0;
    for (const nal of iterateAvcNalUnits(data, config)) {
      requireMedia(++count <= 4096, 'resource_limit');
      end = nal.offset + nal.length;
      requireMedia(nal.length > 0 && end <= data.length && !(data[nal.offset]! & 0x80), 'unsupported_media');
      const kind = extractNalUnitTypeForAvc(data[nal.offset]!);
      // Scalable/multiview/depth extensions have a different reference model.
      requireMedia(kind !== 13 && kind !== 14 && kind !== 15 && kind !== 20 && kind !== 21, 'unsupported_codec');
      if (kind === 7) {
        const next = avcSurface(data.subarray(nal.offset, end));
        requireMedia(next.surfaceBytes <= surfaceBytes && next.pixels <= pixels && next.references <= references && next.reorder <= reorder, 'resource_limit');
      }
    }
    requireMedia(count > 0 && end === data.length, 'unsupported_media');
  } };
}
export interface PlatformVideoDecoder { decode(packet: EncodedPacket): void; flush(): Promise<void>; close(): void; }
export async function openPlatformVideo(track: InputVideoTrack, output: (sample: VideoSample) => void, error: (error: unknown) => void): Promise<PlatformVideoDecoder> {
  const sink = new VideoSampleSink(track);
  // Published declarations deliberately omit this source ABI. Validate the seam
  // explicitly; do not cast an arbitrary object to a functioning decoder.
  requireMedia('_createDecoder' in sink && typeof sink._createDecoder === 'function', 'unsupported_codec');
  const raw: unknown = await sink._createDecoder(output, error);
  requireMedia(raw && typeof raw === 'object' && 'decode' in raw && typeof raw.decode === 'function'
    && 'flush' in raw && typeof raw.flush === 'function' && 'close' in raw && typeof raw.close === 'function', 'unsupported_codec');
  const decode = raw.decode.bind(raw), flush = raw.flush.bind(raw), close = raw.close.bind(raw);
  return { decode: packet => { decode(packet); }, flush: async () => { await flush(); }, close: () => { close(); } };
}
