/** Structural subset of the matching, trusted wasm-bindgen module. The product
 * loads verified code in its Worker. These are not untrusted JSON transports. */
export interface RasterPort {
  raster(frame: Uint32Array): { status: number; pixels: Uint8Array };
  rasterImages?(frame: Uint32Array, images: Uint8Array): { status: number; pixels: Uint8Array };
  invalidate(): void;
}
export interface DecoderPort {
  decodeImage(encoded: Uint8Array, minWidth?: number, minHeight?: number): { status: number; words: Uint32Array; pixels: Uint8Array };
  invalidate(): void;
}
export interface ShapingPort {
  registerFont(font: Uint8Array): number;
  unregisterFont(handle: number): void;
  shapeRegistered(handle: number, frame: Uint32Array): Uint32Array;
  caretRegistered(handle: number, frame: Uint32Array): Uint32Array;
  measureRegistered(handle: number, frame: Uint32Array): Uint32Array;
  outlineRegistered(handle: number, frame: Uint32Array): Uint32Array;
  shapeBatch(font: Uint8Array, frame: Uint32Array): Uint32Array;
  outlineBatch(font: Uint8Array, frame: Uint32Array): Uint32Array;
  caretBatch(font: Uint8Array, frame: Uint32Array): Uint32Array;
  measureBatch(font: Uint8Array, frame: Uint32Array): Uint32Array;
  invalidate(): void;
}
export interface WasmFrame {
  readonly metadata: string;
  /** Consumes the Rust allocation, including when the generated binding traps. */
  take_pixels(): Uint8Array;
  free(): void;
}
export interface WasmOwner {
  command(request: string): string;
  render(request: string, raster: RasterPort): WasmFrame;
  prepare_render(request: string): WasmPreparedFrame;
  complete_render(frame: WasmPreparedFrame, reply: {status: number; pixels: Uint8Array}): WasmCompletedFrame;
  complete_validation(validation: WasmPixelValidation): WasmCompletedFrame;
  free(): void;
}
export interface WasmPreparedFrame {
  readonly failure: string;
  begin(raster: SteppedRasterPort): RasterStart;
  /** Consumes the prepared frame and snapshots the complete component reply. */
  begin_validation(reply: {status: number; pixels: Uint8Array}): WasmPixelValidation;
  free(): void;
}
export interface WasmPixelValidation { readonly failed: boolean; step(workUnits: number): boolean; free(): void; }
export interface WasmCompletedFrame extends WasmFrame { readonly invalidates_backend: boolean; }
export type RasterFailure = {status: 1 | 2 | 3 | 4; pixels: Uint8Array};
export type RasterStart = {status: 0; execution: RasterTask} | RasterFailure;
export interface RasterTask {
  step(workUnits?: number): {status: 0; complete: boolean} | RasterFailure;
  take(): {status: 0; pixels: Uint8Array};
  close(): void;
}
export interface SteppedRasterPort { beginRaster(frame: Uint32Array, images?: Uint8Array): RasterStart; invalidate(): void; }
export interface WasmSourceOwner extends WasmOwner {
  resize(request: string, source: Uint8Array, decoder: DecoderPort): string;
  prepare(request: string, source: Uint8Array, fonts: Uint8Array,
    decoder: DecoderPort, shaping: ShapingPort): string;
}
export interface PlaybackModule {
  prepare_delivery_playback(request: string, contents: Uint8Array): string;
  PlaybackSession: new () => WasmOwner;
  PptxPlaybackSession: new () => WasmSourceOwner;
}
export interface Frame<Info> {
  info: Info;
  /** Host publication fence, captured under the exclusive playback lease. */
  viewportRevision: number;
  /** Caller-owned premultiplied sRGB RGBA8, independent of WASM memory. */
  pixels: Uint8Array;
}
