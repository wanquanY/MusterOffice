/** One Rust runtime per host Worker. All code bytes are supplied by the host. */
import initializeKernel, * as kernel from './runtime/mo_wasm.js';
import rasterFactory from './runtime/mo-skia.mjs';
import textFactory from './runtime/mo-hb.mjs';
import {WasmPlayback, RasterComponent, ShapingComponent} from './lib/playback-client/src/index.js';
export * from './lib/playback-client/src/index.js';

let claimed = false;
export async function createPlaybackRuntime(modules) {
  if (claimed) throw new Error('Playback runtime already initialized in this module; use a new host Worker');
  claimed = true; // Claim before calling host accessors; failed init also stays terminal.
  // Capture each host property once; accessors cannot swap a URL for checked code.
  const {kernel: kernelModule, raster: rasterModule, text: textModule} = modules;
  for (const [name, code] of [['kernel',kernelModule],['raster',rasterModule],['text',textModule]]) {
    if (!(code instanceof WebAssembly.Module)) throw new TypeError('Expected explicit compiled '+name+' module');
  }
  await initializeKernel({module_or_path: kernelModule});
  const createRaster = () => RasterComponent.create(rasterFactory, rasterModule);
  const createShaping = () => ShapingComponent.create(textFactory, textModule);
  const [raster, shaping] = await Promise.all([createRaster(), createShaping()]);
  return {playback: new WasmPlayback(kernel), raster, shaping, createRaster, createShaping};
}
