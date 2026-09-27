import type {RasterModule, RasterReply} from './index.js';

export type RasterFailure = Extract<RasterReply, {status: 1 | 2 | 3 | 4}>;
export type RasterExecutionStart = {status: 0; execution: RasterExecution} | RasterFailure;
export type RasterExecutionStep = {status: 0; complete: boolean} | RasterFailure;
const failure = (status: 1 | 2 | 3 | 4): RasterFailure => ({status, pixels: new Uint8Array(0)});
function status(value: number): asserts value is 0 | 1 | 2 | 3 | 4 {
  if (!Number.isInteger(value) || value < 0 || value > 4) throw Error('Invalid execution status');
}

/** Owns one retained component task. Host schedules each synchronous step and
 * can close between steps. A work unit is not a pixel/time guarantee. Filled
 * paths retain scan state; geometry preparation, complex scan intervals,
 * hairlines, clips, validation, allocations and ownership transfers may block. */
export class RasterExecution {
  #module: RasterModule | undefined;
  #task = 0;
  #allocations: number[] = [];
  #slots = 0;
  #width: number;
  #height: number;
  #complete = false;
  #busy = false;
  #alive: (() => boolean) | undefined;
  #poison: (() => void) | undefined;
  #finish: (() => void) | undefined;
  private constructor(module: RasterModule, width: number, height: number,
    alive: () => boolean, poison: () => void, finish: () => void) {
    this.#module = module; this.#width = width; this.#height = height;
    this.#alive = alive; this.#poison = poison; this.#finish = finish;
  }
  /** Internal component factory. Call RasterComponent.beginRaster instead. */
  static begin(module: RasterModule, frame: Uint32Array, images: Uint8Array | undefined,
    alive: () => boolean, poison: () => void, finish: () => void): RasterExecutionStart {
    const task = new RasterExecution(module, frame[2]!, frame[3]!, alive, poison, finish);
    return task.#run(() => {
      const input = task.#allocate(frame.byteLength);
      module.HEAPU32.set(frame, input / 4);
      task.#slots = task.#allocate(8);
      module.HEAPU32.fill(0, task.#slots / 4, task.#slots / 4 + 2);
      let imagePointer = 0;
      if (images?.length) {
        imagePointer = task.#allocate(images.length); module.HEAPU8.set(images, imagePointer);
      }
      const reply = module._mo_skia_raster_begin!(input, frame.length, imagePointer,
        images?.length ?? 0, Number(images !== undefined), task.#slots);
      task.#task = module.HEAPU32[task.#slots / 4]!;
      status(reply);
      if (reply ? task.#task !== 0 : !task.#task || task.#task % 4 || task.#task >= module.HEAPU8.length) {
        throw Error('Invalid execution handle ownership');
      }
      if (reply) {task.#fail(reply); return failure(reply);}
      return {status: 0, execution: task};
    });
  }
  get closed(): boolean {return this.#module === undefined;}
  get complete(): boolean {return this.#complete;}
  #allocate(bytes: number): number {
    const m = this.#module!;
    const p = m._malloc(bytes);
    if (!p || p % 4 || p + bytes > m.HEAPU8.length) throw Error('Execution allocation failed');
    this.#allocations.push(p); return p;
  }
  #run<T>(body: () => T): T {
    if (this.#busy) throw Error('Execution busy');
    if (!this.#module) throw Error('Execution closed');
    if (!this.#alive!()) {this.#release(false); throw Error('Component invalid');}
    this.#busy = true;
    try {return body();}
    catch (error) {this.#poison?.(); this.#release(false); throw error;}
    finally {this.#busy = false;}
  }
  #fail(code: 1 | 2 | 3 | 4): void {
    if (code === 2 || code === 4) this.#poison!();
    this.#release(this.#alive!());
  }
  #release(cleanup: boolean): void {
    const m = this.#module;
    if (!m) return;
    this.#module = undefined;
    try {
      if (cleanup) {
        if (this.#task) m._mo_skia_raster_drop!(this.#task);
        for (const p of this.#allocations) m._free(p);
      }
    } catch (error) {this.#poison!(); throw error;}
    finally {
      this.#task = 0; this.#allocations = []; this.#alive = this.#poison = undefined;
      const finish = this.#finish; this.#finish = undefined; finish?.();
    }
  }
  step(workUnits = 1): RasterExecutionStep {
    if (!Number.isInteger(workUnits) || workUnits < 1 || workUnits > 4096) throw RangeError('Invalid work units');
    return this.#run(() => {
      const m = this.#module!;
      m.HEAPU32[this.#slots / 4] = 0;
      const reply = m._mo_skia_raster_step!(this.#task, workUnits, this.#slots);
      const done = m.HEAPU32[this.#slots / 4]!;
      status(reply);
      if ((reply && done) || done > 1) throw Error('Invalid execution completion');
      if (reply) {this.#fail(reply); return failure(reply);}
      this.#complete = done === 1;
      return {status: 0, complete: this.#complete};
    });
  }
  take(): Extract<RasterReply, {status: 0}> {
    if (!this.#complete) throw Error('Execution incomplete');
    return this.#run(() => {
      const m = this.#module!;
      m.HEAPU32.fill(0, this.#slots / 4, this.#slots / 4 + 2);
      const reply = m._mo_skia_raster_take!(this.#task, this.#slots, this.#slots + 4);
      const p = m.HEAPU32[this.#slots / 4]!, bytes = m.HEAPU32[this.#slots / 4 + 1]!;
      if (reply !== 0 || !p || this.#width < 1 || this.#width > 8192 || this.#height < 1 || this.#height > 8192 ||
          bytes !== this.#width * this.#height * 4 || bytes > 67108864 || p + bytes > m.HEAPU8.length) {
        throw Error('Invalid execution pixel ownership');
      }
      const pixels = m.HEAPU8.slice(p, p + bytes);
      m._mo_skia_free(p);
      this.#release(true);
      return {status: 0, width: this.#width, height: this.#height, pixels};
    });
  }
  close(): void {
    if (this.#busy) throw Error('Execution busy');
    if (this.#module) this.#release(this.#alive!());
  }
}
