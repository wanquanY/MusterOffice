import type { ExactRatio, SampleInterval, TimeSample } from './contract.js';

/** Structural public ABI of the separately verified mo-media-index-wasm SDK.
 * Host injects the initialized runtime; this package never fetches code. */
export interface BrowserMediaIndexRuntime {
  readonly MediaIndex: {
    open(byteLength: bigint, readAt: (offset: bigint, count: number) => Uint8Array<ArrayBuffer>, options: string): BrowserMediaIndex;
  };
}
export interface BrowserMediaIndex {
  summary(): string;
  query(request: string): string;
  videoTiming(track: number): string;
  selectVideo(request: string): string;
  free(): void;
}
export type MovieVideoSelection = { readonly type: 'outside' }
  | { readonly type: 'gap'; readonly movie: SampleInterval; readonly edit: number | null }
  | { readonly type: 'sample'; readonly sample: number; readonly decodeStart: string;
      readonly access: { readonly sample: number; readonly decodeStart: string } | null;
      readonly edit: number | null; readonly movie: SampleInterval; readonly rawStart: string; readonly rawEnd: string;
      readonly mediaTime: ExactRatio; readonly mediaSide: TimeSample['side'] };

export interface MovieVideoInfo {
  readonly id: number;
  readonly mediaTimescale: number;
  readonly movieTimescale: number;
  readonly samples: number;
  readonly end: ExactRatio;
}
