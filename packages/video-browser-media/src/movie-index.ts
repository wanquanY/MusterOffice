import { BrowserMediaError, requireMedia, type BrowserMediaLimits, type TimeSample } from './contract.js';
import type { BrowserMediaIndex, BrowserMediaIndexRuntime, MovieVideoInfo, MovieVideoSelection } from './movie-contract.js';
import { compare, contains, fraction, validateScale } from './time.js';

export interface RawVideoTrack { ordinal: number; id: number; timescale: number; samples: number; descriptions: number; }
export interface RawVideoPacket {
  bytes: { offset: string; length: string }; dts: string; pts: string;
  decodeDuration: number; description: number; syncSample: boolean;
}
function object(value: unknown): Record<string, unknown> {
  requireMedia(value && typeof value === 'object' && !Array.isArray(value), 'unsupported_media');
  return value as Record<string, unknown>;
}
export function u32(value: unknown): number {
  requireMedia(typeof value === 'number' && Number.isInteger(value) && value >= 0 && value <= 0xffffffff, 'unsupported_media');
  return value;
}
export function integer(value: unknown): bigint {
  requireMedia(typeof value === 'string' && value.length <= 20 && /^(0|-?[1-9][0-9]*)$/.test(value), 'invalid_timing');
  return BigInt(value);
}
function callIndex<T>(run: () => T): T {
  try {
    return run();
  } catch (error) {
    if (error instanceof BrowserMediaError) throw error;
    let kind: unknown;
    try { kind = object(JSON.parse(String(error))).kind; } catch { /* Unknown ABI failures are not valid media. */ }
    throw new BrowserMediaError(kind === 'limit' ? 'resource_limit' : kind === 'unsupported' ? 'unsupported_media' : 'invalid_request');
  }
}
function decode(run: () => string): Record<string, unknown> {
  return callIndex(() => {
    const text = run(); requireMedia(typeof text === 'string' && text.length <= 2 << 20, 'resource_limit');
    return object(JSON.parse(text));
  });
}

/** A single immutable Blob supplies both the Rust index and decoder bytes.
 * Every time/edit decision stays in Rust. TS only validates the public ABI and
 * reads bounded sample pages; it does not reconstruct a movie timeline. */
export class BrowserMovieIndex {
  private index: BrowserMediaIndex | undefined;
  readonly tracks: readonly RawVideoTrack[];
  private readonly info = new Map<number, MovieVideoInfo>();
  private constructor(index: BrowserMediaIndex, tracks: RawVideoTrack[]) { this.index = index; this.tracks = tracks; }
  static open(blob: Blob, runtime: BrowserMediaIndexRuntime, limits: BrowserMediaLimits): BrowserMovieIndex {
    const worker = globalThis as unknown as { FileReaderSync?: unknown };
    requireMedia(typeof worker.FileReaderSync === 'function', 'unsupported_media');
    // FileReaderSync is Worker-only and absent from the DOM declaration library.
    const Reader = worker.FileReaderSync as new () => { readAsArrayBuffer(blob: Blob): ArrayBuffer };
    const reader = new Reader();
    const index = callIndex(() => runtime.MediaIndex.open(BigInt(blob.size), (offset, count) => {
      requireMedia(typeof offset === 'bigint' && offset >= 0n && Number.isInteger(count) && count > 0 && count <= 65536
        && offset + BigInt(count) <= BigInt(blob.size), 'resource_limit');
      return new Uint8Array(reader.readAsArrayBuffer(blob.slice(Number(offset), Number(offset) + count)));
    }, JSON.stringify({ maxInputBytes: limits.maxFileBytes, maxMetadataBytes: 32 << 20, maxBoxes: 8192,
      maxTracks: limits.maxTracks, maxSamples: 2_000_000, maxEdits: 4096, maxSampleDescriptions: 128, maxDescriptionBytes: 1 << 20 })));
    try {
      const summary = decode(() => index.summary());
      requireMedia(summary.profile === 'musteroffice.media-index/1-draft', 'unsupported_media');
      const count = u32(summary.tracks); requireMedia(count > 0 && count <= limits.maxTracks, 'resource_limit');
      const tracks: RawVideoTrack[] = [], ids = new Set<number>();
      for (let ordinal = 0; ordinal < count; ordinal++) {
        const page = decode(() => index.query(JSON.stringify({ type: 'track', track: ordinal })));
        requireMedia(page.type === 'track', 'unsupported_media');
        const track = object(page.track), id = u32(track.id);
        requireMedia(id > 0 && !ids.has(id), 'unsupported_media'); ids.add(id);
        if (JSON.stringify(track.handler) !== '[118,105,100,101]') continue;
        const timescale = u32(track.timescale); validateScale(timescale);
        tracks.push({ ordinal, id, timescale, samples: u32(track.samples), descriptions: u32(track.descriptions) });
      }
      return new BrowserMovieIndex(index, tracks);
    } catch (error) { index.free(); throw error; }
  }
  private handle(): BrowserMediaIndex { requireMedia(this.index, 'closed'); return this.index; }
  track(id: number): RawVideoTrack {
    this.handle(); const track = this.tracks.find(track => track.id === id);
    requireMedia(track, 'invalid_request'); return track;
  }
  timing(id: number): MovieVideoInfo {
    const track = this.track(id), previous = this.info.get(id); if (previous) return structuredClone(previous);
    const value = decode(() => this.handle().videoTiming(track.ordinal));
    requireMedia(value.trackId === id && value.mediaTimescale === track.timescale && value.samples === track.samples, 'unsupported_media');
    const end = object(value.movieEnd) as unknown as MovieVideoInfo['end']; fraction(end);
    const info = { id, mediaTimescale: track.timescale, movieTimescale: u32(value.movieTimescale), samples: track.samples, end };
    this.info.set(id, info); return structuredClone(info);
  }
  select(id: number, request: TimeSample): MovieVideoSelection {
    const track = this.track(id); fraction(request.time);
    requireMedia(request.side === 'before' || request.side === 'after', 'invalid_request');
    const text = JSON.stringify({ track: track.ordinal, time: request.time, side: request.side }); requireMedia(text.length <= 16384, 'resource_limit');
    const value = decode(() => this.handle().selectVideo(text));
    requireMedia(value.type === 'outside' || value.type === 'gap' || value.type === 'sample', 'unsupported_media');
    if (value.type === 'outside') return { type: 'outside' };
    const selection = value as unknown as Exclude<MovieVideoSelection, {type: 'outside'}>;
    requireMedia(selection.movie && compare(selection.movie.start, selection.movie.end) < 0 && contains(selection.movie, request), 'invalid_timing');
    if (selection.edit !== null) u32(selection.edit);
    if (selection.type === 'sample') {
      requireMedia(u32(selection.sample) < track.samples && integer(selection.rawStart) < integer(selection.rawEnd), 'invalid_timing');
      integer(selection.decodeStart); fraction(selection.mediaTime);
      requireMedia(selection.mediaSide === 'before' || selection.mediaSide === 'after', 'invalid_timing');
      if (selection.access !== null) {
        requireMedia(u32(selection.access.sample) <= selection.sample, 'invalid_timing'); integer(selection.access.decodeStart);
      }
    }
    return structuredClone(selection);
  }
  packets(track: RawVideoTrack, start: number): RawVideoPacket[] {
    const value = decode(() => this.handle().query(JSON.stringify({ type: 'samples', track: track.ordinal, start, count: 128 })));
    requireMedia(value.type === 'samples' && value.total === track.samples && Array.isArray(value.items)
      && value.items.length === Math.min(128, track.samples - start), 'unsupported_media');
    return value.items.map(item => {
      const packet = object(item), bytes = object(packet.bytes);
      requireMedia(integer(bytes.offset) >= 0n && integer(bytes.length) > 0n, 'unsupported_media');
      integer(packet.dts); integer(packet.pts); u32(packet.description);
      requireMedia(u32(packet.decodeDuration) > 0 && typeof packet.syncSample === 'boolean', 'unsupported_media');
      return packet as unknown as RawVideoPacket;
    });
  }
  close(): void { const index = this.index; this.index = undefined; this.info.clear(); index?.free(); }
}
