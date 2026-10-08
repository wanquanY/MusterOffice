/** Structural ABI of the independently pinned shared Rust PCM component.
 * The host injects initialized code and owns all durable storage/publication. */
export interface BrowserPcmRuntime {
  readonly profile: 'musteroffice.pcm-worker/1-draft' | string;
  readonly PcmDecode: { open(options: string): BrowserPcmDecode };
  readonly MoviePcm: {
    open(options: string, record: string, clock: string,
      lookup: (source: string, index: number) => string | undefined,
      read: (reference: string, maxBytes: number) => Uint8Array<ArrayBuffer>): BrowserMoviePcm;
  };
}
export type WritePcmTile = (reference: string, bytes: Uint8Array<ArrayBuffer>) => true;
export interface BrowserPcmDecode {
  packetTimestamp(ticks: bigint, timescale: number): bigint;
  push(timestamp: bigint, frames: number, sampleRate: number, channels: number, pcm: Uint8Array<ArrayBuffer>, write: WritePcmTile): void;
  finish(write: WritePcmTile): string;
  seal(reference: string): string;
  retainedBytes(): number;
  free(): void;
}
export interface BrowserMoviePcm {
  info(): string;
  read(first: bigint, frames: number): Uint8Array;
  retainedBytes(): number;
  clear(): void;
  free(): void;
}
export interface BrowserPcmOrigin {
  readonly resource: string;
  /** SHA-256 of the SAME admitted immutable Blob, verified by the host. */
  readonly content: string;
  readonly stream: string;
  /** Host fingerprint for this browser/platform decoder build and policy.
   * Adapter adds its own profile and exact AudioSpecificConfig to this digest. */
  readonly decoderProfile: string;
}
export interface MovieAudioClock {
  readonly trackId: number;
  readonly movieTimescale: number;
  readonly mediaTimescale: number;
  readonly declaredTrackDuration: string | null;
  readonly edits: readonly { readonly movieStart: string; readonly movieDuration: string;
    readonly mediaTime: string | null; readonly rate1616: number }[];
}
/** Owned completion handle, independent of the source input after success.
 * It proves computation/EOF, not host durability or authorization. Seal ALL
 * provisional references, then atomically publish record/clock/options/tiles.
 * Closing releases the WASM handle and its shared application reservation. */
export interface BrowserPcmCompletion {
  readonly options: string;
  readonly clock: string;
  readonly record: string;
  readonly movieInfo: string;
  readonly decodedPackets: number;
  readonly decodedBlocks: number;
  readonly writtenTiles: number;
  seal(reference: string): string;
  close(): void;
}
