import type {Frame, RasterTask, SteppedRasterPort, WasmFrame, WasmOwner, WasmPreparedFrame} from './ports.js';
import {Owner, type OwnerLease, PlaybackComputationError, PlaybackStateError, decode, requireResponse} from './owner.js';
import type {PlaybackSessionResponse} from '../../contracts/src/generated/playback-session-response.js';
import type {PptxPlaybackSessionResponse} from '../../contracts/src/generated/pptx-playback-session-response.js';

/** Explicit bounded ownership, with no timers, queue or implicit async loop.
 * Close this execution before advancing/disposing its playback owner. */
export class PlaybackExecution<Info> {
  #prepared: WasmPreparedFrame | undefined;
  #task: RasterTask | undefined;
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
    const task = this.#task, prepared = this.#prepared;
    this.#task = undefined; this.#prepared = undefined; this.#closed = true;
    const failures: unknown[] = [];
    try { task?.close(); } catch (error) { failures.push(error); }
    try { prepared?.free(); } catch (error) { failures.push(error); }
    this.lease.release();
    if (failures.length) throw new AggregateError(failures, 'Playback execution release failed');
  }
  #finish(raw: WasmOwner, reply: {status: number; pixels: Uint8Array}): Frame<Info> {
    const prepared = this.#prepared!;
    this.#prepared = undefined; // Matching wasm-bindgen consumes before entering Rust.
    const completed = raw.complete_render(prepared, reply);
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
  /** A work unit is not a duration guarantee. The host chooses when to yield. */
  step(workUnits = 1): boolean {
    if (!Number.isInteger(workUnits) || workUnits < 1 || workUnits > 4096) throw RangeError('Invalid work units');
    return this.#run(raw => {
      const reply = this.#task!.step(workUnits);
      if (reply.status !== 0) {
        this.#finish(raw, reply);
        throw new PlaybackStateError('INVALID_RESPONSE');
      }
      requireResponse(typeof reply.complete === 'boolean');
      this.#complete = reply.complete;
      return this.#complete;
    });
  }
  take(): Frame<Info> {
    if (this.#closed) throw new PlaybackStateError('CLOSED');
    if (!this.#complete) throw Error('Playback execution incomplete');
    return this.#run(raw => {
      const frame = this.#finish(raw, this.#task!.take());
      this.#release(); return frame;
    });
  }
  close(): void {
    if (!this.#closed) this.#run(() => this.#release());
  }
}
