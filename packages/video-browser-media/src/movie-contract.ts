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
  scanAvc(request: string, readAt: (offset: bigint, count: number) => Uint8Array<ArrayBuffer>): string;
  avcSample(request: string): string;
  avcBootstrap(request: string): Uint8Array<ArrayBuffer>;
  free(): void;
}
export type ChromaLocation = 'left' | 'center' | 'topLeft' | 'top' | 'bottomLeft' | 'bottom';
/** Shared codec facts, distinct from container tags or browser colour defaults. */
export interface AvcSampleMetadata {
  readonly sample: number; readonly description: number; readonly chromaLocation: ChromaLocation | null;
  readonly picture: null | {
    readonly ppsId: number; readonly ppsRevision: number; readonly spsRevision: number;
    readonly idr: boolean; readonly fieldPicture: boolean; readonly bottomField: boolean;
    readonly sequence: {
      readonly id: number; readonly profile: number; readonly level: number;
      readonly chromaFormat: number; readonly bitDepthLuma: number; readonly bitDepthChroma: number;
      readonly codedWidth: number; readonly codedHeight: number; readonly visible: readonly number[];
      readonly frameMbsOnly: boolean; readonly separateColourPlane: boolean;
      readonly maxReferences: number; readonly reorderFrames: number | null; readonly decoderBufferFrames: number | null;
      readonly chromaLocationTop: number; readonly chromaLocationBottom: number; readonly chromaLocationPresent: boolean;
      readonly color: {readonly signalPresent: boolean; readonly descriptionPresent: boolean; readonly fullRange: boolean;
        readonly primaries: number; readonly transfer: number; readonly matrix: number};
    };
  };
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
