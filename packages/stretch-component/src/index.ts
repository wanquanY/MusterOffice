/** Thin serial owner of one pinned DSP instance. Host supplies verified code. */
export interface StretchModule {
  HEAPF32: Float32Array;
  HEAPU32: Uint32Array;
  _malloc(bytes: number): number;
  _free(pointer: number): void;
  _mo_stretch_abi(): number;
  _mo_stretch_create(channels: number, block: number, interval: number, maxBytes: number, out: number): number;
  _mo_stretch_destroy(state: number): void;
  _mo_stretch_latency(state: number, input: number, output: number): number;
  _mo_stretch_memory(state: number, live: number, peak: number): number;
  _mo_stretch_begin(state: number, input: number, frames: number, rate: number): number;
  _mo_stretch_process(state: number, input: number, inFrames: number, output: number, outFrames: number): number;
}
export type StretchFactory = (options: {
  instantiateWasm(imports: WebAssembly.Imports, receive: (instance: WebAssembly.Instance, module: WebAssembly.Module) => void): WebAssembly.Exports;
}) => Promise<StretchModule>;

export class StretchComponent {
  readonly channels: number;
  readonly sampleRate: number;
  readonly inputLatency: number;
  readonly outputLatency: number;
  #module: StretchModule | undefined;
  #state: number;
  #meta = 0;
  #input = 0;
  #output = 0;
  #busy = false;
  #begun = false;
  private constructor(module: StretchModule, sampleRate: number, channels: number, maxBytes: number) {
    this.#module = module; this.sampleRate = sampleRate; this.channels = channels; this.#state = 0;
    this.inputLatency = 0; this.outputLatency = 0;
    try {
      this.#meta = this.#allocate(12);
      this.#status(module._mo_stretch_create(channels, Math.floor(sampleRate * 12 / 100), Math.floor(sampleRate * 3 / 100), maxBytes, this.#meta));
      this.#state = module.HEAPU32[this.#meta / 4]!;
      if (!this.#state) throw new Error("Null stretch instance");
      this.#status(module._mo_stretch_latency(this.#state, this.#meta, this.#meta + 4));
      this.inputLatency = module.HEAPU32[this.#meta / 4]!;
      this.outputLatency = module.HEAPU32[this.#meta / 4 + 1]!;
      if (this.inputLatency < 1 || this.inputLatency > 65536 || this.outputLatency < 1 || this.outputLatency > 65536)
        throw new Error("Invalid stretch latency");
      // Fixed scratch, no per-block component-heap malloc/free.
      this.#input = this.#allocate(65536 * channels * 4);
      this.#output = this.#allocate(4096 * channels * 4);
    } catch (error) { this.dispose(); throw error; }
  }
  static async create(factory: StretchFactory, compiled: WebAssembly.Module, sampleRate: number, channels: number, maxBytes = 32 << 20): Promise<StretchComponent> {
    if (!Number.isInteger(sampleRate) || sampleRate < 8000 || sampleRate > 384000 || !Number.isInteger(channels) || channels < 1 || channels > 64 ||
        !Number.isInteger(maxBytes) || maxBytes < 1 || maxBytes > 128 << 20) throw new Error("Invalid stretch configuration");
    for (const item of WebAssembly.Module.imports(compiled))
      if (item.kind !== "function" || item.module !== "env" || !["emscripten_resize_heap", "_abort_js"].includes(item.name))
        throw new Error("Unexpected stretch component import");
    const module = await factory({ instantiateWasm(imports, receive) {
      const instance = new WebAssembly.Instance(compiled, imports); receive(instance, compiled); return instance.exports;
    }});
    if (module._mo_stretch_abi() !== 1) throw new Error("Stretch ABI mismatch");
    return new StretchComponent(module, sampleRate, channels, maxBytes);
  }
  #allocate(bytes: number): number {
    const pointer = this.#module!._malloc(bytes);
    if (!pointer) throw new Error("Stretch scratch allocation failed");
    return pointer;
  }
  #status(status: number): void { if (status !== 0) throw new Error(`Stretch component status ${status}`); }
  #validate(input: Float32Array): void {
    if (!(input instanceof Float32Array) || input.length % this.channels || input.length / this.channels > 65536 ||
        input.some(value => !Number.isFinite(value))) throw new Error("Invalid stretch PCM");
  }
  #call(operation: (module: StretchModule) => void): void {
    const module = this.#module;
    if (!module || this.#busy) throw new Error("Stretch instance unavailable");
    this.#busy = true;
    try { operation(module); }
    catch (error) { this.#busy = false; this.dispose(); throw error; }
    finally { this.#busy = false; }
  }
  begin(input: Float32Array, rate: number): void {
    this.#validate(input);
    if (this.#begun || input.length !== this.inputLatency * this.channels || !Number.isFinite(rate) || rate < 1/64 || rate > 64)
      throw new Error("Invalid stretch start");
    this.#call(m => {
      m.HEAPF32.set(input, this.#input / 4);
      this.#status(m._mo_stretch_begin(this.#state, this.#input, this.inputLatency, rate));
      this.#begun = true;
    });
  }
  process(input: Float32Array, output: Float32Array): void {
    this.#validate(input);
    if (!this.#begun || !(output instanceof Float32Array) || !output.length || output.length % this.channels || output.length / this.channels > 4096)
      throw new Error("Invalid stretch output");
    this.#call(m => {
      m.HEAPF32.set(input, this.#input / 4);
      this.#status(m._mo_stretch_process(this.#state, this.#input, input.length / this.channels, this.#output, output.length / this.channels));
      // Refresh heap view after the call in case a vector growth grew WASM memory.
      output.set(m.HEAPF32.subarray(this.#output / 4, this.#output / 4 + output.length));
    });
  }
  memory(): { liveBytes: number; peakBytes: number; scratchBytes: number } {
    let liveBytes = 0, peakBytes = 0;
    this.#call(m => {
      this.#status(m._mo_stretch_memory(this.#state, this.#meta, this.#meta + 4));
      liveBytes = m.HEAPU32[this.#meta / 4]!; peakBytes = m.HEAPU32[this.#meta / 4 + 1]!;
    });
    return { liveBytes, peakBytes, scratchBytes: (65536 + 4096) * this.channels * 4 + 12 };
  }
  dispose(): void {
    if (this.#busy) throw new Error("Stretch component busy");
    const m = this.#module; this.#module = undefined;
    if (!m) return;
    try {
      if (this.#state) m._mo_stretch_destroy(this.#state);
      for (const p of [this.#meta, this.#input, this.#output]) if (p) m._free(p);
    } finally { this.#state = 0; this.#meta = 0; this.#input = 0; this.#output = 0; }
  }
}
