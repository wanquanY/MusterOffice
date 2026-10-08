import type { ExactRatio, TimeSample, RationalTime } from '../../contracts/src/generated/video-frame-plan.js';
export type { ExactRatio, TimeSample, RationalTime };

export type BrowserMediaErrorCode = 'invalid_request' | 'resource_limit' | 'unsupported_media'
  | 'unsupported_codec' | 'pending_source' | 'invalid_timing' | 'busy' | 'closed' | 'cancelled' | 'deadline';
export class BrowserMediaError extends Error {
  constructor(readonly code: BrowserMediaErrorCode) { super(code); this.name = 'BrowserMediaError'; }
}
export function requireMedia(value: unknown, code: BrowserMediaErrorCode): asserts value {
  if (!value) throw new BrowserMediaError(code);
}
export interface BrowserMediaLimits {
  readonly maxFileBytes: number;
  readonly sourceCacheBytes: number;
  readonly maxTracks: number;
  readonly maxPixels: number;
  readonly maxAudioChannels: number;
  readonly maxAudioSampleRate: number;
  readonly maxAudioBlockBytes: number;
  readonly maxEncodedPacketBytes: number;
  /** Max forward sample-ordinal distance reused across movie requests. Larger
   * jumps reopen from a verified access point; this never changes selection. */
  readonly maxSequentialVideoSamples: number;
  /** Bounded submitted raw audio packets, including codec-internal output.
   * A decoder that withholds this entire window is stopped by the deadline. */
  readonly maxPendingAudioPackets: number;
  readonly maxDurationSeconds: number;
  readonly operationDeadlineMs: number;
}
export const DEFAULT_MEDIA_LIMITS: Readonly<BrowserMediaLimits> = Object.freeze({
  maxFileBytes: 32 * 1024 ** 3, sourceCacheBytes: 8 * 1024 ** 2, maxTracks: 32,
  maxPixels: 3840 * 2160, maxAudioChannels: 8, maxAudioSampleRate: 192000,
  maxAudioBlockBytes: 8 * 1024 ** 2, maxDurationSeconds: 7 * 86400,
  maxEncodedPacketBytes: 8 * 1024 ** 2,
  maxSequentialVideoSamples: 256, maxPendingAudioPackets: 16,
  operationDeadlineMs: 30000,
});
export interface TrackTime {
  readonly id: number;
  readonly codec: string | null;
  /** Track/container tick grid; it is not a guessed constant frame rate. */
  readonly timescale: number;
  /** First packet timestamp, including negative decoder preroll if present.
   * This is NOT the playable start after movie edit lists / padding removal. */
  readonly first: RationalTime;
  /** End timestamp, not duration measured from the first timestamp. */
  readonly end: RationalTime;
  /** Configuration probe only. A successful real decode is still required. */
  readonly decodeConfigurationSupported: boolean;
}
export interface BrowserVideoTrack extends TrackTime {
  readonly kind: 'video';
  readonly codedWidth: number; readonly codedHeight: number;
  readonly displayWidth: number; readonly displayHeight: number;
  readonly pixelAspectRatio: { readonly num: number; readonly den: number };
  readonly transformationMatrix: readonly number[];
  readonly rotation: number; readonly flip: boolean;
  readonly colorSpace: VideoColorSpaceInit;
}
export interface BrowserAudioTrack extends TrackTime {
  readonly kind: 'audio';
  readonly sampleRate: number; readonly channels: number;
}
export type BrowserTrack = BrowserVideoTrack | BrowserAudioTrack;
export interface BrowserMediaInfo { readonly byteLength: number; readonly tracks: readonly BrowserTrack[] }
export interface SampleInterval { readonly start: ExactRatio; readonly end: ExactRatio }
export interface BrowserVideoSample {
  readonly width: number; readonly height: number;
  readonly displayWidth: number; readonly displayHeight: number;
  readonly rotation: number; readonly flip: boolean;
  readonly pixelAspectRatio: { readonly num: number; readonly den: number };
  /** Explicit SDR sRGB conversion. It is never labelled raw YUV. */
  copyRgba(destination: Uint8Array<ArrayBuffer>): Promise<void>;
  /** Return an owned platform frame for GPU import. Caller MUST close the lease. */
  cloneFrame(): BrowserVideoFrameLease;
}
export interface BrowserVideoFrameLease {
  readonly frame: VideoFrame;
  /** Releases both the platform frame and its shared host-budget admission. */
  close(): void;
}
export interface BrowserAudioSample {
  readonly sampleRate: number; readonly numberOfFrames: number; readonly numberOfChannels: number;
  copyInterleaved(destination: Float32Array<ArrayBuffer>): void;
}
/** Borrowed until the awaited callback completes. Retained methods fail closed.
 * Only explicit cloneFrame transfers ownership outside the callback. */
export type ConsumeVideo = (sample: BrowserVideoSample, interval: SampleInterval) => void | boolean | Promise<void | boolean>;
export type ConsumeAudio = (sample: BrowserAudioSample, interval: SampleInterval) => void | boolean | Promise<void | boolean>;
