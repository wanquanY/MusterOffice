import type { PlaybackBinding, RationalTime } from '../../contracts/src/generated/playback-session-request.js';
import type { PlaybackSessionFailure } from '../../contracts/src/generated/playback-session-response.js';
import type { PptxPlaybackSessionFailure } from '../../contracts/src/generated/pptx-playback-session-response.js';
import type { WasmFrame, WasmOwner } from './ports.js';

/** A typed core rejection; the immutable prepared plan remains usable. */
export class PlaybackComputationError extends Error {
  constructor(readonly kind: 'author' | 'source',
    readonly diagnostic: PlaybackSessionFailure | PptxPlaybackSessionFailure,
    readonly invalidatesBackend = false) {
    super(`${kind} playback computation rejected`);
    this.name = 'PlaybackComputationError';
  }
}
export class PlaybackStateError extends Error {
  constructor(readonly code: 'CLOSED' | 'BUSY' | 'INVALID_RESPONSE' | 'INPUT_LIMIT') {
    super(`WASM playback: ${code}`); this.name = 'PlaybackStateError';
  }
}
export function requireResponse(condition: unknown): asserts condition {
  if (!condition) throw new PlaybackStateError('INVALID_RESPONSE');
}
export function sameBinding(a: PlaybackBinding, b: PlaybackBinding): boolean {
  return a.session === b.session && a.revision === b.revision && a.generation === b.generation;
}
export function sameTime(a: RationalTime, b: RationalTime): boolean {
  // Domain validation and normalization are owned by Rust. Compare exact
  // accepted instants without losing int64 precision through Number.
  return BigInt(a.ticks) * BigInt(b.timescale) === BigInt(b.ticks) * BigInt(a.timescale);
}
export function encode(value: unknown): string {
  const text = JSON.stringify(value);
  if (text === undefined || !fitsUtf8(text, 32 * 1024 * 1024)) {
    throw new PlaybackStateError('INPUT_LIMIT');
  }
  return text;
}
export function decode<T>(text: string): T {
  requireResponse(typeof text === 'string' && fitsUtf8(text, 64 * 1024 * 1024));
  // This is the trusted matching Rust module's output, not a general wire
  // validator. Rust validates the complete request and component reply.
  return JSON.parse(text) as T;
}
export function fitsUtf8(text: string, limit: number): boolean {
  // At most three UTF-8 bytes per UTF-16 code unit, including replacement for
  // lone surrogates. Normal frame metadata needs no temporary encoding buffer.
  return text.length <= limit && (text.length <= Math.floor(limit / 3) ||
    new TextEncoder().encode(text).byteLength <= limit);
}
export function inputBytes(value: Uint8Array): void {
  if (!(value instanceof Uint8Array) || !(value.buffer instanceof ArrayBuffer) ||
      value.byteLength > 128 * 1024 * 1024) throw new PlaybackStateError('INPUT_LIMIT');
}
export function takeFrame<T>(frame: WasmFrame): { response: T; pixels: Uint8Array } {
  let consumed = false;
  try {
    const response = decode<T>(frame.metadata);
    consumed = true; // wasm-bindgen consumes self before transferring the Vec.
    const pixels = frame.take_pixels();
    requireResponse(pixels instanceof Uint8Array && pixels.buffer instanceof ArrayBuffer &&
      pixels.byteLength <= 64 * 1024 * 1024);
    return { response, pixels };
  } catch (error) {
    if (!consumed) {
      try { frame.free(); }
      catch (cleanup) { throw new AggregateError([error, cleanup], 'Frame read failed and release failed'); }
    }
    throw error;
  }
}
export function frameSize(width: number, height: number, bytes: string, pixels: Uint8Array,
  expected: readonly [number, number]): void {
  requireResponse(width === expected[0] && height === expected[1] &&
    Number.isSafeInteger(width) && Number.isSafeInteger(height) && width > 0 && height > 0 &&
    width * height * 4 === pixels.byteLength && bytes === String(pixels.byteLength));
}

/** Exclusively owns one Rust allocation, with no finalizer or frame cache.
 * Synchronous calls must run inside a host Worker for hard cancellation. */
export class Owner<Raw extends WasmOwner> {
  #raw: Raw | undefined;
  #busy = false;
  #lease: object | undefined;
  constructor(raw: Raw) { this.#raw = raw; }
  get closed(): boolean { return this.#raw === undefined; }
  run<T>(body: (raw: Raw) => T): T {
    return this.#run(body, undefined);
  }
  acquire(): OwnerLease<Raw> {
    if (this.#busy || this.#lease) throw new PlaybackStateError('BUSY');
    if (!this.#raw) throw new PlaybackStateError('CLOSED');
    const token = {}; this.#lease = token;
    return {run: body => this.#run(body, token), release: () => {
      if (this.#lease === token) this.#lease = undefined;
    }};
  }
  #run<T>(body: (raw: Raw) => T, lease: object | undefined): T {
    if (this.#busy || this.#lease !== lease) throw new PlaybackStateError('BUSY');
    if (!this.#raw) throw new PlaybackStateError('CLOSED');
    this.#busy = true;
    try { return body(this.#raw); }
    catch (error) {
      if (!(error instanceof PlaybackComputationError) || error.invalidatesBackend) {
        try { this.#release(); }
        catch (cleanup) { throw new AggregateError([error, cleanup], 'Playback failed and release failed'); }
      }
      throw error;
    } finally { this.#busy = false; }
  }
  dispose(body: (raw: Raw) => void): void {
    this.run(raw => {
      let failed = false;
      let primary: unknown;
      try { body(raw); }
      catch (error) { failed = true; primary = error; }
      try { this.#release(); }
      catch (cleanup) {
        if (failed) throw new AggregateError([primary, cleanup], 'Dispose failed and release failed');
        throw cleanup;
      }
      if (failed) throw primary;
    });
  }
  /** Idempotent local release. Never touches host-owned component instances. */
  close(): void {
    if (this.#busy || this.#lease) throw new PlaybackStateError('BUSY');
    this.#release();
  }
  #release(): void {
    const raw = this.#raw;
    this.#raw = undefined; // No retry/double free if the matching module traps.
    this.#lease = undefined;
    raw?.free();
  }
}
export interface OwnerLease<Raw> { run<T>(body: (raw: Raw) => T): T; release(): void; }

export function initialize<Raw extends WasmOwner, T>(raw: Raw, body: (owner: Owner<Raw>) => T): T {
  const owner = new Owner(raw);
  try { return body(owner); }
  catch (error) {
    try { owner.close(); }
    catch (cleanup) { throw new AggregateError([error, cleanup], 'Prepare failed and release failed'); }
    throw error;
  }
}
