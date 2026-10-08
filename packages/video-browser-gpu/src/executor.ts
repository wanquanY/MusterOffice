import { colorWords, colorKernels, drawColor, type ColorKernels, type VideoFrameLease } from './color.js';
import { rawInputPlan } from './raw-input.js';
import { extent, invalid, parseProgram } from './program.js';
import { i420Words, i420Kernels, encodeI420, type I420Kernels, type I420Readback } from './i420.js';

type Allocation = { buffer: GPUBuffer; bytes: number; refs: number };
type Capability = { allocation: Allocation; width: number; height: number; hasHdr: boolean; closed: boolean };
export interface GpuBitmapLease {
  readonly bitmap: ImageBitmap; readonly width: number; readonly height: number;
  close(): void;
}
export interface LinearGpuImage {
  readonly width: number; readonly height: number; readonly hasHdr: boolean;
  /** Idempotent. In-flight references remain alive until queue completion. */
  close(): void;
}
function aborted(signal?: AbortSignal): void { signal?.throwIfAborted(); }
/** Thin executor for the shared core's program/shader. Owns an exclusive device;
 * the caller selects the adapter. No codecs, timeline, color math or auth here.
 * Allocation counts cover owned GPU buffers, not driver/codec memory or CPU data.
 */
export class BrowserGpuExecutor {
  readonly #device: GPUDevice;
  readonly #pipeline: GPUComputePipeline;
  readonly #layout: GPUBindGroupLayout;
  readonly #images = new WeakMap<LinearGpuImage, Capability>();
  readonly #allocations = new Set<Allocation>();
  readonly #maxBytes: number;
  #bytes = 0;
  #surfaceBytes = 0;
  readonly #surfaceCharges = new Set<() => void>();
  readonly #bitmaps = new Set<GpuBitmapLease>();
  #canvas: { canvas: OffscreenCanvas; context: GPUCanvasContext; release: () => void } | undefined;
  #color: ColorKernels | undefined;
  #i420: I420Kernels | undefined;
  #busy = false;
  #closed = false;
  #lost: string | undefined;
  private constructor(device: GPUDevice, pipeline: GPUComputePipeline, maxBytes: number) {
    this.#device = device; this.#pipeline = pipeline; this.#layout = pipeline.getBindGroupLayout(0); this.#maxBytes = maxBytes;
    void device.lost.then(info => { this.#lost = info.message || 'GPU device lost'; this.close(); });
  }
  /** The supplied adapter is consumed to create one exclusive device. */
  static async create(adapter: GPUAdapter, shader: string, maxBytes = 512 * 2 ** 20): Promise<BrowserGpuExecutor> {
    if (!Number.isSafeInteger(maxBytes) || maxBytes < 16 || maxBytes > 2 ** 30 || shader.length > 128 * 1024) invalid('executor limits');
    const device = await adapter.requestDevice({ label: 'MusterOffice video compositor', requiredLimits: {
      maxStorageBufferBindingSize: Math.min(adapter.limits.maxStorageBufferBindingSize, 256 * 2 ** 20),
      maxBufferSize: Math.min(adapter.limits.maxBufferSize, 512 * 2 ** 20),
    } });
    try {
      const pipeline = await scoped(device, () => device.createComputePipelineAsync({
        label: 'MusterOffice shared linear composition', layout: 'auto',
        compute: { module: device.createShaderModule({ code: shader }), entryPoint: 'compose' },
      }));
      return new BrowserGpuExecutor(device, pipeline, maxBytes);
    } catch (e) { device.destroy(); throw e; }
  }
  get allocatedBytes(): number { return this.#bytes; }
  get retainedSurfaceBytes(): number { return this.#surfaceBytes; }
  #ready(): void {
    if (this.#closed || this.#lost) throw new Error(this.#lost ?? 'GPU executor closed');
  }
  async #exclusive<T>(run: () => Promise<T>, signal?: AbortSignal): Promise<T> {
    this.#ready(); aborted(signal);
    if (this.#busy) throw new Error('GPU executor operation already active');
    this.#busy = true;
    try { return await run(); } finally { this.#busy = false; }
  }
  #allocate(bytes: number, usage: GPUBufferUsageFlags, fill?: (view: DataView) => void, mapped = false): Allocation {
    this.#ready();
    if (bytes > this.#device.limits.maxBufferSize || ((usage & GPUBufferUsage.STORAGE) !== 0 && bytes > this.#device.limits.maxStorageBufferBindingSize) || this.#bytes + this.#surfaceBytes + bytes > this.#maxBytes) throw new RangeError('GPU allocation budget');
    const buffer = this.#device.createBuffer({ size: bytes, usage, mappedAtCreation: !!fill || mapped });
    const a = { buffer, bytes, refs: 1 }; this.#bytes += bytes; this.#allocations.add(a);
    try { if (fill) { fill(new DataView(buffer.getMappedRange())); buffer.unmap(); } return a; }
    catch (e) { this.#release(a); throw e; }
  }
  #release(a: Allocation): void {
    if (!this.#allocations.has(a)) return;
    if (--a.refs === 0) { a.buffer.destroy(); this.#allocations.delete(a); this.#bytes -= a.bytes; }
  }
  #cap(image: LinearGpuImage): Capability {
    this.#ready(); const c = this.#images.get(image);
    if (!c || c.closed) throw new TypeError('Foreign or closed GPU image'); return c;
  }
  #publish(a: Allocation, width: number, height: number, hasHdr: boolean): LinearGpuImage {
    const c: Capability = { allocation: a, width, height, hasHdr, closed: false };
    const result = Object.freeze({ width, height, hasHdr, close: () => {
      if (!c.closed) { c.closed = true; this.#release(a); }
    } });
    this.#images.set(result, c); return result;
  }
  /** Explicit CPU-normalized input path. Never an implicit color conversion.
   * Caller bytes are read synchronously; no borrowed array survives an await.
   */
  upload(width: number, height: number, hasHdr: boolean, pixels: Float32Array, signal?: AbortSignal): Promise<LinearGpuImage> {
    extent(width, height);
    if (typeof hasHdr !== 'boolean' || !(pixels instanceof Float32Array) || !(pixels.buffer instanceof ArrayBuffer) || pixels.length !== width * height * 4) invalid('linear image');
    return this.#exclusive(async () => {
      let a: Allocation | undefined;
      try {
        await scoped(this.#device, () => { a = this.#allocate(width * height * 16, GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC, view => {
          for (let i = 0; i < pixels.length; i += 4) {
            const alpha = pixels[i + 3]!;
            if (alpha < 0 || alpha > 1) invalid('linear alpha');
            for (let c = 0; c < 4; c++) {
              const v = pixels[i + c]!;
              if (!Number.isFinite(v) || Math.abs(v) > 1e12 || (alpha === 0 && c < 3 && v !== 0)) invalid('linear component');
              view.setFloat32((i + c) * 4, v, true);
            }
          }
        }); });
        this.#ready(); aborted(signal); return this.#publish(a!, width, height, hasHdr);
      } catch (e) { if (a) this.#release(a); throw e; }
    }, signal);
  }
  render(value: unknown, images: ReadonlyMap<number, LinearGpuImage>, signal?: AbortSignal): Promise<LinearGpuImage> {
    return this.#exclusive(async () => {
      const program = parseProgram(value, this.#device.limits);
      if (images.size !== program.images.length) invalid('image count');
      const retained: Capability[] = [], sources = new Map<number, Allocation>(), surfaces = new Map<number, Allocation>();
      const temporaries: Allocation[] = [];
      let published: Allocation | undefined;
      try {
        for (const i of program.images) {
          const image = images.get(i.id); if (!image) invalid('missing GPU image');
          const c = this.#cap(image);
          if (c.width !== i.width || c.height !== i.height || c.hasHdr !== i.hasHdr) invalid('GPU image metadata');
          c.allocation.refs++; retained.push(c); sources.set(i.id, c.allocation);
        }
        const alloc = (n: number, usage: GPUBufferUsageFlags, fill?: (v: DataView) => void) => {
          const a = this.#allocate(n, usage, fill); temporaries.push(a); return a;
        };
        const commands = await scoped(this.#device, () => {
          const dummy = alloc(16, GPUBufferUsage.STORAGE);
          const encoder = this.#device.createCommandEncoder();
          for (const command of program.commands) {
            aborted(signal);
            if (command.kind === 'create') {
              surfaces.set(command.id, alloc(command.width * command.height * 16, GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC)); continue;
            }
            const uniform = alloc(112, GPUBufferUsage.UNIFORM, view => command.parameters.forEach((word, index) => view.setUint32(index * 4, word, true)));
            const s = command.source;
            const source = s.kind === 'solid' ? dummy : (s.kind === 'image' ? sources : surfaces).get(s.id)!;
            const group = this.#device.createBindGroup({ layout: this.#layout, entries: [
              { binding: 0, resource: { buffer: source.buffer } },
              { binding: 1, resource: { buffer: surfaces.get(command.target)!.buffer } },
              { binding: 2, resource: { buffer: uniform.buffer } },
            ] });
            const pass = encoder.beginComputePass(); pass.setPipeline(this.#pipeline); pass.setBindGroup(0, group);
            pass.dispatchWorkgroups(Math.ceil(command.parameters[0]! / 8), Math.ceil(command.parameters[1]! / 8)); pass.end();
          }
          return encoder.finish();
        });
        this.#ready(); aborted(signal);
        this.#device.queue.submit([commands]);
        await this.#complete(signal);
        this.#ready(); aborted(signal);
        published = surfaces.get(program.output)!;
        return this.#publish(published, program.width, program.height, program.hasHdr);
      } finally {
        for (const a of temporaries) if (a !== published) this.#release(a);
        for (const c of retained) this.#release(c.allocation);
      }
    }, signal);
  }
  /** Explicit diagnostic/CPU fallback readback. GPU images can also be reused
   * as composition inputs. Returned CPU bytes are caller-owned and not part of allocatedBytes.
   */
  readback(image: LinearGpuImage, signal?: AbortSignal): Promise<Float32Array> {
    return this.#exclusive(async () => {
      const c = this.#cap(image); c.allocation.refs++;
      let staging: Allocation | undefined;
      try {
        const commands = await scoped(this.#device, () => {
          staging = this.#allocate(c.allocation.bytes, GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ);
          const encoder = this.#device.createCommandEncoder(); encoder.copyBufferToBuffer(c.allocation.buffer, 0, staging.buffer, 0, staging.bytes); return encoder.finish();
        });
        this.#ready(); aborted(signal); this.#device.queue.submit([commands]);
        await this.#complete(signal);
        await staging!.buffer.mapAsync(GPUMapMode.READ);
        this.#ready(); aborted(signal);
        const view = new DataView(staging!.buffer.getMappedRange());
        const result = new Float32Array(view.byteLength / 4);
        for (let i = 0; i < result.length; i++) result[i] = view.getFloat32(i * 4, true);
        staging!.buffer.unmap(); return result;
      } finally { if (staging) this.#release(staging); this.#release(c.allocation); }
    }, signal);
  }

  /** Install the matching core color kernels once on this exclusive device. */
  configureColor(shaders: unknown): Promise<void> {
    return this.#exclusive(async () => {
      if (this.#color) throw new Error('GPU color kernels already installed');
      const candidate = await scoped(this.#device, () => colorKernels(this.#device, shaders));
      this.#ready(); this.#color = candidate;
    });
  }
  #colors(): ColorKernels {
    if (!this.#color) throw new Error('GPU color kernels not installed'); return this.#color;
  }
  #uniform(words: number[]): Allocation {
    return this.#allocate(80, GPUBufferUsage.UNIFORM, view => words.forEach((v, i) => view.setUint32(i * 4, v, true)));
  }
  #surface(bytes: number): () => void {
    this.#ready();
    if (this.#bytes + this.#surfaceBytes + bytes > this.#maxBytes) throw new RangeError('GPU surface budget');
    const release = () => { if (this.#surfaceCharges.delete(release)) this.#surfaceBytes -= bytes; };
    this.#surfaceCharges.add(release); this.#surfaceBytes += bytes; return release;
  }
  /** Consumes the lease even on rejection. Keep the underlying decoded surface
   * admitted to the media budget until conversion completes; never pass a raw
   * frame after prematurely releasing its host media lease.
   */
  importVideo(lease: VideoFrameLease, value: unknown, signal?: AbortSignal): Promise<LinearGpuImage> {
    return this.#exclusive(async () => {
      const kernel = this.#colors().input, plan = rawInputPlan(value, lease.frame);
      const { width, height } = plan.source;
      let source: Allocation | undefined, output: Allocation | undefined, uniform: Allocation | undefined, status: Allocation | undefined, statusRead: Allocation | undefined, published = false;
      try {
        const commands = await scoped(this.#device, async () => {
          // Copy original planes directly into mapped GPU upload memory. The
          // WebCodecs copy crops visibleRect but does not apply display/SAR,
          // transfer functions, premultiplication or chroma reconstruction.
          source = this.#allocate(plan.byteLength, GPUBufferUsage.STORAGE, undefined, true);
          const layout = await lease.frame.copyTo(new Uint8Array(source.buffer.getMappedRange()), { layout: plan.planes as PlaneLayout[] });
          this.#ready(); aborted(signal);
          if (layout.length !== plan.planes.length || layout.some((v,i) => v.offset !== plan.planes[i]!.offset || v.stride !== plan.planes[i]!.stride)) invalid('platform copy layout');
          source.buffer.unmap();
          output = this.#allocate(width * height * 16, GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC);
          uniform = this.#allocate(224, GPUBufferUsage.UNIFORM, view => plan.parameters.forEach((v,i) => view.setUint32(i*4,v,true)));
          status = this.#allocate(4, GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC);
          statusRead = this.#allocate(4, GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ);
          const bindings = this.#device.createBindGroup({ layout: kernel.getBindGroupLayout(0), entries: [
            { binding: 3, resource: { buffer: status.buffer } },
            { binding: 0, resource: { buffer: source.buffer } }, { binding: 1, resource: { buffer: output.buffer } }, { binding: 2, resource: { buffer: uniform.buffer } },
          ] });
          const encoder = this.#device.createCommandEncoder(), pass = encoder.beginComputePass();
          pass.setPipeline(kernel); pass.setBindGroup(0, bindings); pass.dispatchWorkgroups(Math.ceil(width / 8), Math.ceil(height / 8)); pass.end();
          encoder.copyBufferToBuffer(status.buffer, 0, statusRead.buffer, 0, 4);
          return encoder.finish();
        });
        this.#ready(); aborted(signal); this.#device.queue.submit([commands]); await this.#complete(signal);
        await statusRead!.buffer.mapAsync(GPUMapMode.READ);
        this.#ready(); aborted(signal);
        const invalidSamples = new DataView(statusRead!.buffer.getMappedRange()).getUint32(0, true);
        statusRead!.buffer.unmap();
        if (invalidSamples !== 0) invalid('raw sample exceeds declared bit depth');
        published = true;
        return this.#publish(output!, width, height, plan.hasHdr);
      } finally { if (status) this.#release(status); if (statusRead) this.#release(statusRead); if (source) this.#release(source); if (uniform) this.#release(uniform); if (output && !published) this.#release(output); }
    }, signal).then(image => {
      try { lease.close(); return image; } catch (error) { image.close(); throw error; }
    }, error => { try { lease.close(); } finally { throw error; } });
  }
  /** Diagnostic/image readback with explicit straight sRGB alpha; no canvas
   * premultiplication roundtrip or CPU color conversion is used.
   */
  readSrgb8(image: LinearGpuImage, parameters: unknown, signal?: AbortSignal): Promise<Uint8Array> {
    return this.#exclusive(async () => {
      const c = this.#cap(image), kernel = this.#colors().output;
      const words = colorWords(parameters, c.width, c.height, false, c.hasHdr);
      c.allocation.refs++;
      const rowBytes = Math.ceil(c.width * 4 / 256) * 256;
      let texture: GPUTexture | undefined, uniform: Allocation | undefined, staging: Allocation | undefined, release: (() => void) | undefined;
      try {
        const commands = await scoped(this.#device, () => {
          release = this.#surface(c.width * c.height * 4);
          texture = this.#device.createTexture({ size: [c.width, c.height], format: 'rgba8unorm', usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC });
          uniform = this.#uniform(words);
          staging = this.#allocate(rowBytes * c.height, GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ);
          const encoder = this.#device.createCommandEncoder();
          drawColor(this.#device, encoder, kernel, c.allocation.buffer, uniform.buffer, texture.createView());
          encoder.copyTextureToBuffer({ texture }, { buffer: staging.buffer, bytesPerRow: rowBytes }, [c.width, c.height]);
          return encoder.finish();
        });
        this.#ready(); aborted(signal); this.#device.queue.submit([commands]); await this.#complete(signal);
        await staging!.buffer.mapAsync(GPUMapMode.READ); this.#ready(); aborted(signal);
        const source = new Uint8Array(staging!.buffer.getMappedRange()), out = new Uint8Array(c.width * c.height * 4);
        for (let y = 0; y < c.height; y++) out.set(source.subarray(y * rowBytes, y * rowBytes + c.width * 4), y * c.width * 4);
        staging!.buffer.unmap(); return out;
      } finally { texture?.destroy(); release?.(); if (uniform) this.#release(uniform); if (staging) this.#release(staging); this.#release(c.allocation); }
    }, signal);
  }
  /** Install exact pinned core television-output kernels on this device. */
  configureI420(shader: string): Promise<void> {
    return this.#exclusive(async () => {
      if (this.#i420) throw new Error('GPU I420 kernels already installed');
      this.#i420 = await scoped(this.#device, () => i420Kernels(this.#device, shader));
      this.#ready();
    });
  }
  /** Bounded explicit encode input. Host owns returned CPU bytes and encoder
   * admission; GPU temporaries are released on success, failure and cancellation.
   */
  readI420(image: LinearGpuImage, parameters: unknown, signal?: AbortSignal): Promise<I420Readback> {
    return this.#exclusive(async () => {
      const c = this.#cap(image), kernels = this.#i420;
      if (!kernels) throw new Error('GPU I420 kernels not installed');
      const plan = i420Words(parameters, c.width, c.height, c.hasHdr);
      c.allocation.refs++;
      const allocations: Allocation[] = [];
      const alloc = (bytes: number, usage: GPUBufferUsageFlags, fill?: (v: DataView) => void) => {
        const a = this.#allocate(bytes, usage, fill); allocations.push(a); return a;
      };
      let staging: Allocation | undefined;
      try {
        const commands = await scoped(this.#device, () => {
          const uniform = alloc(176, GPUBufferUsage.UNIFORM, view => plan.words.forEach((word, i) => view.setUint32(i * 4, word, true)));
          const television = alloc(c.width * c.height * 16, GPUBufferUsage.STORAGE);
          const chroma = alloc(c.width * c.height * 4, GPUBufferUsage.STORAGE);
          const packed = alloc(plan.bytes, GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC);
          staging = alloc(plan.bytes, GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ);
          const encoder = this.#device.createCommandEncoder();
          encodeI420(this.#device, encoder, kernels, [uniform.buffer, c.allocation.buffer, television.buffer, chroma.buffer, packed.buffer], c.width, c.height);
          encoder.copyBufferToBuffer(packed.buffer, 0, staging.buffer, 0, plan.bytes);
          return encoder.finish();
        });
        this.#ready(); aborted(signal); this.#device.queue.submit([commands]); await this.#complete(signal);
        await staging!.buffer.mapAsync(GPUMapMode.READ); this.#ready(); aborted(signal);
        const data = new Uint8Array(staging!.buffer.getMappedRange()).slice();
        staging!.buffer.unmap();
        return Object.freeze({ width: c.width, height: c.height, data,
          layout: Object.freeze(plan.layout.map(p => Object.freeze(p))) as I420Readback['layout'] });
      } finally { for (const a of allocations) this.#release(a); this.#release(c.allocation); }
    }, signal);
  }
  #display(width: number, height: number): GPUCanvasContext {
    if (this.#canvas && (this.#canvas.canvas.width !== width || this.#canvas.canvas.height !== height)) {
      this.#canvas.context.unconfigure(); this.#canvas.release(); this.#canvas = undefined;
    }
    if (!this.#canvas) {
      const release = this.#surface(width * height * 4);
      try {
        const canvas = new OffscreenCanvas(width, height), context = canvas.getContext('webgpu');
        if (!context) throw new Error('Offscreen WebGPU canvas unavailable');
        context.configure({ device: this.#device, format: 'rgba8unorm', colorSpace: 'srgb', alphaMode: 'premultiplied' });
        this.#canvas = { canvas, context, release };
      } catch (e) { release(); throw e; }
    }
    return this.#canvas.context;
  }
  /** GPU presentation snapshot for a host-owned bitmaprenderer. No pixel
   * readback. The host must close/ack the lease even after transferring bitmap.
   */
  toBitmap(image: LinearGpuImage, parameters: unknown, signal?: AbortSignal): Promise<GpuBitmapLease> {
    return this.#exclusive(async () => {
      const c = this.#cap(image), kernel = this.#colors().output;
      const words = colorWords(parameters, c.width, c.height, true, c.hasHdr);
      c.allocation.refs++;
      let uniform: Allocation | undefined, release: (() => void) | undefined, published = false;
      try {
        await scoped(this.#device, () => {
          uniform = this.#uniform(words);
          const context = this.#display(c.width, c.height);
          release = this.#surface(c.width * c.height * 4);
          const encoder = this.#device.createCommandEncoder();
          drawColor(this.#device, encoder, kernel, c.allocation.buffer, uniform.buffer, context.getCurrentTexture().createView());
          aborted(signal);
          // Canvas texture expires across tasks. Submit synchronously; the
          // private OffscreenCanvas is never published before scopes/completion.
          this.#device.queue.submit([encoder.finish()]);
        });
        await this.#complete(signal); this.#ready(); aborted(signal);
        const bitmap = this.#canvas!.canvas.transferToImageBitmap();
        let closed = false;
        const lease: GpuBitmapLease = Object.freeze({ bitmap, width: c.width, height: c.height, close: () => {
          if (!closed) { closed = true; bitmap.close(); release!(); this.#bitmaps.delete(lease); }
        } });
        this.#bitmaps.add(lease); published = true; return lease;
      } finally { if (!published) release?.(); if (uniform) this.#release(uniform); this.#release(c.allocation); }
    }, signal);
  }
  async #complete(signal?: AbortSignal): Promise<void> {
    // An abort retires this exclusive device. The host creates a fresh executor;
    // no future operation can reuse commands whose completion is now uncertain.
    const onAbort = () => this.close();
    signal?.addEventListener('abort', onAbort, { once: true });
    if (signal?.aborted) onAbort();
    try { await this.#device.queue.onSubmittedWorkDone(); } finally { signal?.removeEventListener('abort', onAbort); }
  }
  close(): void {
    if (this.#closed) return;
    this.#closed = true;
    for (const bitmap of this.#bitmaps) bitmap.close();
    this.#canvas?.context.unconfigure(); this.#canvas = undefined;
    for (const release of this.#surfaceCharges) release();
    this.#device.destroy();
    for (const a of this.#allocations) a.buffer.destroy();
    this.#allocations.clear(); this.#bytes = 0;
  }
}
async function scoped<T>(device: GPUDevice, build: () => T | Promise<T>): Promise<T> {
  for (const filter of ['out-of-memory', 'internal', 'validation'] as const) device.pushErrorScope(filter);
  let value: T | undefined, thrown: unknown, failed = false;
  try { value = await build(); } catch (e) { thrown = e; failed = true; }
  const errors = await Promise.all([device.popErrorScope(), device.popErrorScope(), device.popErrorScope()]);
  if (failed) throw thrown;
  const error = errors.find(x => x); if (error) throw new Error(`GPU operation failed: ${error.message}`);
  return value as T;
}
