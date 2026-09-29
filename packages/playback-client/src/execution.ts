import type {Frame, RasterTask, SteppedRasterPort, WasmCompletedFrame, WasmFrame, WasmOwner, WasmPixelValidation, WasmPreparedFrame} from './ports.js';
import {Owner, type OwnerLease, PlaybackComputationError, PlaybackStateError, decode, requireResponse} from './owner.js';
import type {PlaybackSessionResponse} from '../../contracts/src/generated/playback-session-response.js';
import type {PptxPlaybackSessionResponse} from '../../contracts/src/generated/pptx-playback-session-response.js';

/** Explicit bounded ownership, with no timers, queue or implicit async loop.
 * Close this execution before advancing/disposing its playback owner. */
export class PlaybackExecution<Info> {
  #prepared: WasmPreparedFrame | undefined;
  #task: RasterTask | undefined;
  #validation: WasmPixelValidation | undefined;
  #closed = false;
  #complete = false;
  private constructor(readonly kind: 'author' | 'source', private readonly lease: OwnerLease<WasmOwner>,
    private readonly raster: SteppedRasterPort,
    private readonly read: (frame: WasmFrame, invalidates: boolean) => Frame<Info>) {}
  /** Internal factory used by the typed playback owner's beginSample. */
  static start<Info>(owner: Owner<WasmOwner>, request: string, kind: 'author' | 'source',
    raster: SteppedRasterPort, read: (frame: WasmFrame, invalidates: boolean) => Frame<Info>): PlaybackExecution<Info> {
    const execution = new PlaybackExecution(kind, owner.acquire(), raster, read);
    return execution.#run(raw => {
      execution.#prepared = raw.prepare_render(request);
      const failure = execution.#prepared.failure;
      requireResponse(typeof failure === 'string');
      if (failure) {
        const response = decode<PlaybackSessionResponse | PptxPlaybackSessionResponse>(failure);
        requireResponse(response.status === 'error');
        throw new PlaybackComputationError(kind, response.error);
      }
      const started = execution.#prepared.begin(raster);
      if (started.status !== 0) {
        execution.#finish(raw, started);
        throw new PlaybackStateError('INVALID_RESPONSE');
      }
      execution.#task = started.execution;
      requireResponse(execution.#task && typeof execution.#task.step === 'function' &&
        typeof execution.#task.take === 'function' && typeof execution.#task.close === 'function');
      return execution;
    });
  }
  get closed(): boolean { return this.#closed; }
  get complete(): boolean { return this.#complete; }
  #run<T>(body: (raw: WasmOwner) => T): T {
    if (this.#closed) throw new PlaybackStateError('CLOSED');
    return this.lease.run(raw => {
      try { return body(raw); }
      catch (error) {
        const failures = [error];
        try { this.#release(); } catch (cleanup) { failures.push(cleanup); }
        if (!(error instanceof PlaybackComputationError) || error.invalidatesBackend || failures.length > 1) {
          try { this.raster.invalidate(); } catch (cleanup) { failures.push(cleanup); }
        }
        if (failures.length > 1) throw new AggregateError(failures, 'Playback execution and release failed');
        throw error;
      }
    });
  }
  #release(): void {
    const task = this.#task, prepared = this.#prepared, validation = this.#validation;
    this.#task = undefined; this.#prepared = undefined; this.#validation = undefined; this.#closed = true;
    const failures: unknown[] = [];
    try { task?.close(); } catch (error) { failures.push(error); }
    try { prepared?.free(); } catch (error) { failures.push(error); }
    try { validation?.free(); } catch (error) { failures.push(error); }
    this.lease.release();
    if (failures.length) throw new AggregateError(failures, 'Playback execution release failed');
  }
  #finish(raw: WasmOwner, reply: {status: number; pixels: Uint8Array}): Frame<Info> {
    const prepared = this.#prepared!;
    this.#prepared = undefined; // Matching wasm-bindgen consumes before entering Rust.
    return this.#readCompleted(raw.complete_render(prepared, reply));
  }
  #readCompleted(completed: WasmCompletedFrame): Frame<Info> {
    let invalidates: boolean;
    try {
      invalidates = completed.invalidates_backend;
      requireResponse(typeof invalidates === 'boolean');
    } catch (error) {
      try { completed.free(); } catch (cleanup) { throw new AggregateError([error, cleanup], 'Completion read and release failed'); }
      throw error;
    }
    return this.read(completed, invalidates);
  }
  #finishValidation(raw: WasmOwner): Frame<Info> {
    const validation = this.#validation!;
    this.#validation = undefined; // Matching wasm-bindgen consumes this allocation.
    return this.#readCompleted(raw.complete_validation(validation));
  }
  #checkValidation(raw: WasmOwner): void {
    const failed = this.#validation!.failed;
    requireResponse(typeof failed === 'boolean');
    if (failed) {
      this.#finishValidation(raw); // Preserve diagnosis and quarantine even if the host cancels next.
      throw new PlaybackStateError('INVALID_RESPONSE');
    }
  }
  /** A work unit is not a duration guarantee. The host chooses when to yield. */
  step(workUnits = 1): boolean {
    if (!Number.isInteger(workUnits) || workUnits < 1 || workUnits > 4096) throw RangeError('Invalid work units');
    return this.#run(raw => {
      if (this.#validation) {
        const complete = this.#validation.step(workUnits);
        requireResponse(typeof complete === 'boolean');
        this.#checkValidation(raw);
        this.#complete = complete;
        return complete;
      }
      const reply = this.#task!.step(workUnits);
      if (reply.status !== 0) {
        this.#finish(raw, reply);
        throw new PlaybackStateError('INVALID_RESPONSE');
      }
      requireResponse(typeof reply.complete === 'boolean');
      if (reply.complete) {
        const pixels = this.#task!.take();
        const prepared = this.#prepared!;
        this.#prepared = undefined; // begin_validation consumes before entering Rust.
        this.#validation = prepared.begin_validation(pixels);
        const task = this.#task!; this.#task = undefined;
        task.close();
        requireResponse(this.#validation && typeof this.#validation.step === 'function' &&
          typeof this.#validation.free === 'function');
        this.#checkValidation(raw);
      }
      // Snapshotting and validation are distinct phases; the host may yield here.
      return false;
    });
  }
  take(): Frame<Info> {
    if (this.#closed) throw new PlaybackStateError('CLOSED');
    if (!this.#complete) throw Error('Playback execution incomplete');
    return this.#run(raw => {
      const frame = this.#finishValidation(raw);
      this.#release(); return frame;
    });
  }
  close(): void {
    if (!this.#closed) this.#run(() => this.#release());
  }
}
