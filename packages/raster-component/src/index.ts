/** No fetching, files, fonts, clock or ambient environment. Host owns code trust. */
import {RasterExecution, type RasterExecutionStart} from './execution.js';
export {RasterExecution} from './execution.js';
export type {RasterExecutionStart, RasterExecutionStep} from './execution.js';
export interface RasterModule {
  HEAPU8: Uint8Array;
  HEAPU32: Uint32Array;
  _malloc(bytes: number): number;
  _free(pointer: number): void;
  _mo_skia_abi(): number;
  _mo_skia_clips_abi?(): number;
  _mo_skia_gradient_planes_abi?(): number;
  _mo_skia_office_gradients_abi?(): number;
  _mo_skia_rect_gradients_abi?(): number;
  _mo_skia_elliptic_gradients_abi?(): number;
  _mo_skia_opacity_groups_abi?(): number;
  _mo_skia_snapshot_scopes_abi?(): number;
  _mo_skia_compositing_abi?(): number;
  _mo_skia_images_abi?(): number;
  _mo_skia_raster_images?(request: number, words: number, images: number, imageBytes: number, output: number, bytes: number): number;
  _mo_skia_free(pointer: number): void;
  _mo_image_decode_abi?(): number;
  _mo_image_decode_sized_abi?(): number;
  _mo_image_decode_sized?(encoded: number, length: number, minWidth: number, minHeight: number, output: number, info: number): number;
  _mo_image_decode?(encoded: number, length: number, output: number, info: number): number;
  _mo_skia_raster(request: number, words: number, output: number, bytes: number): number;
  _mo_skia_execution_abi?(): number;
  _mo_skia_raster_begin?(request: number, words: number, images: number, imageBytes: number, withImages: number, task: number): number;
  _mo_skia_raster_step?(task: number, workUnits: number, complete: number): number;
  _mo_skia_raster_take?(task: number, pixels: number, bytes: number): number;
  _mo_skia_raster_drop?(task: number): void;
}
export type RasterFactory = (options: {
  instantiateWasm(imports: WebAssembly.Imports, receive: (instance: WebAssembly.Instance, module: WebAssembly.Module) => void): WebAssembly.Exports;
}) => Promise<RasterModule>;
export type RasterReply = {status: 0; width: number; height: number; pixels: Uint8Array} | {status: 1 | 2 | 3 | 4; pixels: Uint8Array};
export type DecodeReply = {status: number; words: Uint32Array; pixels: Uint8Array};
const claimed = new WeakSet<RasterModule>();
const io = new Set(["fd_close", "fd_write", "fd_seek", "environ_sizes_get", "environ_get"]);
const env = new Set(["emscripten_resize_heap", "_abort_js", "_tzset_js", "exit"]);

/** Single owner per instance. Host must terminate its Worker after invalidation. */
export class RasterComponent {
  #module: RasterModule | undefined;
  #busy = false;
  private constructor(module: RasterModule) { this.#module = module; }
  static async create(factory: RasterFactory, compiled: WebAssembly.Module): Promise<RasterComponent> {
    for (const item of WebAssembly.Module.imports(compiled)) {
      if (item.kind !== "function" || !(item.module === "env" ? env.has(item.name) :
        item.module === "wasi_snapshot_preview1" && io.has(item.name))) {
        throw new Error("Unexpected raster component import");
      }
    }
    const module = await factory({instantiateWasm(imports, receive) {
      let memory: WebAssembly.Memory | undefined;
      for (const item of WebAssembly.Module.imports(compiled)) {
        if (item.module === "env" && item.name === "emscripten_resize_heap") continue;
        const namespace = imports[item.module];
        if (!namespace) throw new Error("Missing raster import namespace");
        // libc initializes environ before calling the component. Its explicit
        // environment is empty; never inherit Emscripten's default USER/HOME/LANG.
        if (item.module === "wasi_snapshot_preview1" && item.name === "environ_sizes_get") {
          namespace[item.name] = (count: number, bytes: number) => {
            if (!memory) throw new Error("Raster memory unavailable");
            const view = new DataView(memory.buffer);
            for (const p of [count, bytes]) {
              if (!Number.isInteger(p) || p < 0 || p % 4 || p + 4 > view.byteLength) throw new Error("Invalid environment output pointer");
              view.setUint32(p, 0, true);
            }
            return 0;
          };
          continue;
        }
        if (item.module === "wasi_snapshot_preview1" && item.name === "environ_get") {
          namespace[item.name] = () => 0;
          continue;
        }
        namespace[item.name] = () => { throw new Error("Raster component trap or ambient access denied: " + item.name); };
      }
      const instance = new WebAssembly.Instance(compiled, imports);
      if (!(instance.exports.memory instanceof WebAssembly.Memory)) throw new Error("Missing raster memory");
      memory = instance.exports.memory;
      receive(instance, compiled);
      return instance.exports;
    }});
    if (claimed.has(module)) throw new Error("Raster module already owned");
    claimed.add(module);
    if (module._mo_skia_abi() !== 4) throw new Error("Raster component ABI mismatch");
    return new RasterComponent(module);
  }
  get invalid(): boolean { return !this.#module; }
  invalidate(): void { this.#module = undefined; }
  get supportsExecution(): boolean {
    const m = this.#module;
    try { return !!m && m._mo_skia_execution_abi?.() === 1 &&
      typeof m._mo_skia_raster_begin === 'function' && typeof m._mo_skia_raster_step === 'function' &&
      typeof m._mo_skia_raster_take === 'function' && typeof m._mo_skia_raster_drop === 'function'; }
    catch (error) {this.invalidate(); throw error;}
  }
  beginRaster(frame: Uint32Array, images?: Uint8Array): RasterExecutionStart {
    const m = this.#module;
    if (!m || this.#busy) throw Error('Raster instance unavailable');
    if (!this.supportsExecution) throw Error('Raster execution extension unavailable');
    for (const input of images ? [frame, images] : [frame]) {
      if (!(input.buffer instanceof ArrayBuffer) || input.buffer === m.HEAPU8.buffer) {
        throw Error('Execution inputs must use independent host ArrayBuffers');
      }
    }
    if ((frame[1] === 13 || frame[1] === 14) && !this.supportsOpacityGroups) throw Error("Raster opacity group extension unavailable");
    if (frame[1] === 14 && !this.supportsSnapshotScopes) throw Error("Raster snapshot scope extension unavailable");
    if (frame.length < 10 || frame.length > 2908303 || (images && images.byteLength > 67108864)) {
      return {status: 1, pixels: new Uint8Array(0)};
    }
    this.#busy = true;
    return RasterExecution.begin(m, frame, images, () => this.#module === m,
      () => this.invalidate(), () => {this.#busy = false;});
  }
  get supportsSnapshotScopes(): boolean {
    const m = this.#module;
    try { return !!m && m._mo_skia_snapshot_scopes_abi?.() === 1; }
    catch (error) { this.invalidate(); throw error; }
  }
  get supportsOpacityGroups(): boolean {
    const m = this.#module;
    try { return !!m && m._mo_skia_opacity_groups_abi?.() === 1; }
    catch (error) { this.invalidate(); throw error; }
  }
  get supportsClips(): boolean {
    const m = this.#module;
    try { return !!m && typeof m._mo_skia_clips_abi === "function" && m._mo_skia_clips_abi() === 1; }
    catch (error) { this.invalidate(); throw error; }
  }
  get supportsRectGradients(): boolean {
    const m = this.#module;
    try { return !!m && typeof m._mo_skia_rect_gradients_abi === "function" && m._mo_skia_rect_gradients_abi() === 1; }
    catch (error) { this.invalidate(); throw error; }
  }
  get supportsEllipticGradients(): boolean {
    const m = this.#module;
    try { return !!m && typeof m._mo_skia_elliptic_gradients_abi === "function" && m._mo_skia_elliptic_gradients_abi() === 1; }
    catch (error) { this.invalidate(); throw error; }
  }
  get supportsOfficeGradients(): boolean {
    const m = this.#module;
    try { return !!m && typeof m._mo_skia_office_gradients_abi === "function" && m._mo_skia_office_gradients_abi() === 1; }
    catch (error) { this.invalidate(); throw error; }
  }
  get supportsGradientPlanes(): boolean {
    const m = this.#module;
    try { return !!m && typeof m._mo_skia_gradient_planes_abi === "function" && m._mo_skia_gradient_planes_abi() === 1; }
    catch (error) { this.invalidate(); throw error; }
  }
  get supportsCompositing(): boolean {
    const m = this.#module;
    try { return !!m && typeof m._mo_skia_compositing_abi === "function" && m._mo_skia_compositing_abi() === 1; }
    catch (error) { this.invalidate(); throw error; }
  }
  get supportsImages(): boolean {
    const m = this.#module;
    try {
      return !!m && typeof m._mo_skia_images_abi === "function" &&
        typeof m._mo_skia_raster_images === "function" && [1,2].includes(m._mo_skia_images_abi());
    } catch (error) { this.invalidate(); throw error; }
  }
  raster(frame: Uint32Array): RasterReply { return this.#raster(frame); }
  get supportsImageDomains(): boolean {
    const m=this.#module;
    try { return this.supportsImages && !!m && m._mo_skia_images_abi!() === 2; }
    catch(error) { this.invalidate(); throw error; }
  }
  get supportsDecode(): boolean {
    const m = this.#module;
    try { return !!m && typeof m._mo_image_decode === "function" &&
      typeof m._mo_image_decode_abi === "function" && m._mo_image_decode_abi() === 1; }
    catch (error) { this.invalidate(); throw error; }
  }
  decodeImage(encoded: Uint8Array, minWidth = 0, minHeight = 0): DecodeReply {
    if (!this.supportsDecode) throw new Error("Image decode extension unavailable");
    if (![minWidth, minHeight].every(n => Number.isInteger(n) && n >= 0 && n <= 8192) || (minWidth === 0) !== (minHeight === 0)) {
      throw new Error("Invalid decode sample demand");
    }
    const m = this.#module!;
    if (this.#busy) throw new Error("Component busy");
    if (!(encoded.buffer instanceof ArrayBuffer) || encoded.buffer === m.HEAPU8.buffer) {
      throw new Error("Encoded image must use an independent host ArrayBuffer");
    }
    if (encoded.byteLength > 33554432 || encoded.byteLength === 0) {
      return {status: encoded.byteLength ? 3 : 1, words: new Uint32Array(9), pixels: new Uint8Array(0)};
    }
    this.#busy = true;
    const allocations: number[] = [];
    let output = 0;
    const allocate = (bytes: number): number => {
      const p = m._malloc(bytes);
      if (!p || p % 4 || p + bytes > m.HEAPU8.length) throw new Error("Decode allocation failed");
      allocations.push(p); return p;
    };
    try {
      const input = allocate(encoded.byteLength), slots = allocate(40);
      m.HEAPU8.set(encoded, input);
      m.HEAPU32.fill(0, slots / 4, slots / 4 + 10);
      const status = minWidth === 0
        ? m._mo_image_decode!(input, encoded.byteLength, slots, slots + 4)
        : (() => {
            if (typeof m._mo_image_decode_sized !== "function" || m._mo_image_decode_sized_abi?.() !== 1) {
              throw new Error("Sized image decode extension unavailable");
            }
            return m._mo_image_decode_sized(input, encoded.byteLength, minWidth, minHeight, slots, slots + 4);
          })();
      output = m.HEAPU32[slots / 4]!;
      const words = m.HEAPU32.slice(slots / 4 + 1, slots / 4 + 10);
      if (!Number.isInteger(status) || status < 0 || status > 5 || status === 4 ||
          (status !== 0 && (output || words.some(v => v !== 0)))) {
        throw new Error("Invalid decoder ownership");
      }
      if (status !== 0) {
        if (status === 2) this.invalidate();
        return {status, words, pixels: new Uint8Array(0)};
      }
      const width = words[0]!, height = words[1]!, bytes = words[8]!;
      if (!output || width < 1 || width > 8192 || height < 1 || height > 8192 ||
          bytes !== width * height * 4 || bytes > 67108864 || output + bytes > m.HEAPU8.length) {
        throw new Error("Invalid decoder allocation");
      }
      const pixels = m.HEAPU8.slice(output, output + bytes);
      m._mo_skia_free(output); output = 0;
      return {status: 0, words, pixels};
    } catch (error) { this.invalidate(); throw error; }
    finally {
      try { if (this.#module) { if (output) m._mo_skia_free(output); for (const p of allocations) m._free(p); } }
      catch (error) { this.invalidate(); throw error; }
      finally { this.#busy = false; }
    }
  }
  rasterImages(frame: Uint32Array, images: Uint8Array): RasterReply {
    if (!this.supportsImages) throw new Error("Raster image extension unavailable");
    if (frame[1] === 6 && !this.supportsImageDomains) throw new Error("Image source domain extension unavailable");
    return this.#raster(frame, images);
  }
  #raster(frame: Uint32Array, images?: Uint8Array): RasterReply {
    const m = this.#module;
    if (!m || this.#busy) throw new Error("Raster instance unavailable");
    // Only immutable host-owned inputs. Growth can detach component views;
    // shared buffers permit concurrent mutation during validation/copying.
    for (const value of images ? [frame, images] : [frame]) {
      if (!(value.buffer instanceof ArrayBuffer) || value.buffer === m.HEAPU8.buffer) {
        throw new Error("Raster inputs must use independent host ArrayBuffers");
      }
    }
    const scoped = frame[1] === 14;
    if (scoped && !this.supportsSnapshotScopes) throw new Error("Raster snapshot scope extension unavailable");
    const opacity = frame[1] === 13 || scoped;
    if (opacity && !this.supportsOpacityGroups) throw new Error("Raster opacity group extension unavailable");
    const elliptic = frame[1] === 12 || opacity;
    if (elliptic && !this.supportsEllipticGradients) throw new Error("Raster elliptic gradient extension unavailable");
    const rect = frame[1] === 11;
    if (rect && !this.supportsRectGradients) throw new Error("Raster rectangular gradient extension unavailable");
    const office = frame[1] === 10;
    if (office && !this.supportsOfficeGradients) throw new Error("Raster Office gradient extension unavailable");
    const planes = frame[1] === 9 || office || rect || elliptic;
    const composite = frame[1] === 8;
    if (planes && !this.supportsGradientPlanes) throw new Error("Raster gradient plane extension unavailable");
    const clipping = frame[1] === 7;
    if (composite && !this.supportsCompositing) throw new Error("Raster compositing extension unavailable");
    if (clipping && !this.supportsClips) throw new Error("Raster clip extension unavailable");
    if (frame.length < (opacity ? 15 : planes || composite ? 14 : clipping ? 13 : images ? 12 : 10) || frame.length > (scoped ? 2908303 : opacity ? 2908239 : elliptic ? 2895950 : rect ? 2887758 : planes ? 2883662 : composite ? 2854990 : clipping ? 2789389 : images ? 2691084 : 2617354) ||
        (images && images.byteLength > 67108864)) return {status: 1, pixels: new Uint8Array(0)};
    this.#busy = true;
    const allocations: number[] = [];
    let output = 0;
    const allocate = (bytes: number): number => {
      const pointer = m._malloc(bytes);
      if (!pointer || pointer % 4 || pointer + bytes > m.HEAPU8.length) throw new Error("Raster input allocation failed");
      allocations.push(pointer);
      return pointer;
    };
    try {
      const request = allocate(frame.byteLength), slots = allocate(8);
      m.HEAPU32.set(frame, request / 4);
      m.HEAPU32.fill(0, slots / 4, slots / 4 + 2);
      let imagePointer = 0;
      if (images?.byteLength) {
        imagePointer = allocate(images.byteLength);
        m.HEAPU8.set(images, imagePointer);
      }
      const status = images
        ? m._mo_skia_raster_images!(request, frame.length, imagePointer, images.byteLength, slots, slots + 4)
        : m._mo_skia_raster(request, frame.length, slots, slots + 4);
      output = m.HEAPU32[slots / 4]!;
      const bytes = m.HEAPU32[slots / 4 + 1]!;
      if (!Number.isInteger(status) || status < 0 || status > 4 || (status !== 0 && (output || bytes))) {
        throw new Error("Invalid raster status or ownership");
      }
      if (status !== 0) {
        if (status === 2 || status === 4) this.invalidate();
        return {status: status as 1 | 2 | 3 | 4, pixels: new Uint8Array(0)};
      }
      const width = frame[2]!, height = frame[3]!;
      if (!output || !bytes || width < 1 || width > 8192 || height < 1 || height > 8192 ||
          bytes > 67108864 || bytes !== width * height * 4 || output + bytes > m.HEAPU8.length) {
        throw new Error("Invalid raster result allocation");
      }
      // Copy only the fully completed frame, never expose mutable component memory.
      const pixels = m.HEAPU8.slice(output, output + bytes);
      m._mo_skia_free(output); output = 0;
      return {status: 0, width, height, pixels};
    } catch (error) {
      this.invalidate();
      throw error;
    } finally {
      // Never call back into a trapped or quarantined instance for cleanup.
      try { if (this.#module) { if (output) m._mo_skia_free(output); for (const p of allocations) m._free(p); } }
      catch (error) { this.invalidate(); throw error; }
      finally { this.#busy = false; }
    }
  }
}
