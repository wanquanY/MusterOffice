import type {WasmPlayback, RasterComponent, ShapingComponent} from './lib/playback-client/src/index.js';
export * from './lib/playback-client/src/index.js';
/** Compile trusted bytes explicitly. Runtime performs no fetch or font discovery. */
export interface PlaybackCode {
  kernel: WebAssembly.Module;
  raster: WebAssembly.Module;
  text: WebAssembly.Module;
}
export interface PlaybackRuntime {
  playback: WasmPlayback;
  raster: RasterComponent;
  shaping: ShapingComponent;
  createRaster(): Promise<RasterComponent>;
  createShaping(): Promise<ShapingComponent>;
}
export function createPlaybackRuntime(modules: PlaybackCode): Promise<PlaybackRuntime>;
