import { BrowserMediaError, requireMedia, type BrowserMediaLimits, type TimeSample } from './contract.js';
import type { MovieAudioClock } from './pcm-contract.js';
import type { AvcSampleMetadata, BrowserMediaIndex, BrowserMediaIndexRuntime, MovieVideoInfo, MovieVideoSelection } from './movie-contract.js';
import { compare, contains, fraction, validateScale } from './time.js';

export interface RawMovieTrack { ordinal: number; id: number; kind: 'video' | 'audio'; timescale: number;
  samples: number; descriptions: number; edits: number; declaredTrackDuration: string | null; }
export interface RawMoviePacket {
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
function callIndex<T>(run: () => T, invalidCode: 'invalid_request' | 'unsupported_media' = 'invalid_request'): T {
  try {
    return run();
  } catch (error) {
    if (error instanceof BrowserMediaError) throw error;
    let kind: unknown;
    try { kind = object(JSON.parse(String(error))).kind; } catch { /* Unknown ABI failures are not valid media. */ }
    throw new BrowserMediaError(kind === 'limit' ? 'resource_limit' : kind === 'unsupported' ? 'unsupported_media' : invalidCode);
  }
}
function decode(run: () => string, invalidCode: 'invalid_request' | 'unsupported_media' = 'invalid_request'): Record<string, unknown> {
  return callIndex(() => {
    const text = run(); requireMedia(typeof text === 'string' && text.length <= 2 << 20, 'resource_limit');
    return object(JSON.parse(text));
  }, invalidCode);
}

/** A single immutable Blob supplies both the Rust index and decoder bytes.
 * Every time/edit decision stays in Rust. TS only validates the public ABI and
 * reads bounded sample pages; it does not reconstruct a movie timeline. */
export class BrowserMovieIndex {
  private index: BrowserMediaIndex | undefined;
  readonly tracks: readonly RawMovieTrack[];
  private readonly info = new Map<number, MovieVideoInfo>();
  private constructor(index: BrowserMediaIndex, tracks: RawMovieTrack[], private readonly movieTimescale: number,
    private readonly readAt: (offset: bigint, count: number) => Uint8Array<ArrayBuffer>) { this.index = index; this.tracks = tracks; }
  static open(blob: Blob, runtime: BrowserMediaIndexRuntime, limits: BrowserMediaLimits): BrowserMovieIndex {
    const worker = globalThis as unknown as { FileReaderSync?: unknown };
    requireMedia(typeof worker.FileReaderSync === 'function', 'unsupported_media');
    // FileReaderSync is Worker-only and absent from the DOM declaration library.
    const Reader = worker.FileReaderSync as new () => { readAsArrayBuffer(blob: Blob): ArrayBuffer };
    const reader = new Reader();
    const readAt = (offset: bigint, count: number): Uint8Array<ArrayBuffer> => {
      requireMedia(typeof offset === 'bigint' && offset >= 0n && Number.isInteger(count) && count > 0 && count <= 65536
        && offset + BigInt(count) <= BigInt(blob.size), 'resource_limit');
      return new Uint8Array(reader.readAsArrayBuffer(blob.slice(Number(offset), Number(offset) + count)));
    };
    const index = callIndex(() => runtime.MediaIndex.open(BigInt(blob.size), readAt, JSON.stringify({ maxInputBytes: limits.maxFileBytes, maxMetadataBytes: 32 << 20, maxBoxes: 8192,
      maxTracks: limits.maxTracks, maxSamples: 2_000_000, maxEdits: 4096, maxSampleDescriptions: 128, maxDescriptionBytes: 1 << 20 })));
    try {
      const summary = decode(() => index.summary());
      requireMedia(summary.profile === 'musteroffice.media-index/1-draft', 'unsupported_media');
      const count = u32(summary.tracks); requireMedia(count > 0 && count <= limits.maxTracks, 'resource_limit');
      const tracks: RawMovieTrack[] = [], ids = new Set<number>();
      for (let ordinal = 0; ordinal < count; ordinal++) {
        const page = decode(() => index.query(JSON.stringify({ type: 'track', track: ordinal })));
        requireMedia(page.type === 'track', 'unsupported_media');
        const track = object(page.track), id = u32(track.id);
        requireMedia(id > 0 && !ids.has(id), 'unsupported_media'); ids.add(id);
        const handler = JSON.stringify(track.handler);
        if (handler !== '[118,105,100,101]' && handler !== '[115,111,117,110]') continue;
        if (track.declaredTrackDuration !== null) requireMedia(integer(track.declaredTrackDuration) >= 0n, 'invalid_timing');
        const timescale = u32(track.timescale); validateScale(timescale);
        tracks.push({ ordinal, id, kind: handler === '[118,105,100,101]' ? 'video' : 'audio', timescale,
          samples: u32(track.samples), descriptions: u32(track.descriptions), edits: u32(track.edits),
          declaredTrackDuration: track.declaredTrackDuration as string | null });
      }
      const movieTimescale = u32(summary.movieTimescale); validateScale(movieTimescale);
      return new BrowserMovieIndex(index, tracks, movieTimescale, readAt);
    } catch (error) { index.free(); throw error; }
  }
  private handle(): BrowserMediaIndex { requireMedia(this.index, 'closed'); return this.index; }
  async scanAvc(id: number, through: number, signal: AbortSignal): Promise<void> {
    const track = this.track(id);
    requireMedia(Number.isInteger(through) && through >= 0 && through < track.samples, 'invalid_request');
    let previous = -1;
    for (;;) {
      signal.throwIfAborted();
      const value = decode(() => this.handle().scanAvc(JSON.stringify({track:track.ordinal,through,count:128}), this.readAt), 'unsupported_media');
      const scanned = u32(value.scanned);
      requireMedia(scanned > previous && scanned <= track.samples && typeof value.ready === 'boolean'
        && value.ready === (scanned > through), 'unsupported_media');
      if (value.ready) return;
      previous = scanned;
      // Yield to host cancel/deadline messages between bounded synchronous batches.
      await new Promise<void>(resolve => setTimeout(resolve, 0));
    }
  }
  avcSample(id: number, sample: number): AvcSampleMetadata {
    const track = this.track(id);
    const value = decode(() => this.handle().avcSample(JSON.stringify({track:track.ordinal,sample})));
    requireMedia(value.sample === sample && u32(value.description) < track.descriptions
      && (value.chromaLocation === null || ['left','center','topLeft','top','bottomLeft','bottom'].includes(String(value.chromaLocation))), 'unsupported_media');
    if (value.picture !== null) {
      const picture = object(value.picture), sequence = object(picture.sequence), color = object(sequence.color);
      requireMedia(typeof picture.idr === 'boolean' && typeof picture.fieldPicture === 'boolean'
        && typeof sequence.frameMbsOnly === 'boolean' && typeof sequence.separateColourPlane === 'boolean'
        && Array.isArray(sequence.visible) && sequence.visible.length === 4, 'unsupported_media');
      u32(picture.ppsId); u32(picture.ppsRevision); u32(picture.spsRevision); u32(sequence.id);
      u32(sequence.codedWidth); u32(sequence.codedHeight); u32(sequence.chromaFormat);
      Object.freeze(color); Object.freeze(sequence.visible); Object.freeze(sequence); Object.freeze(picture);
    }
    return Object.freeze(value) as unknown as AvcSampleMetadata;
  }
  avcBootstrap(id: number, sample: number): Uint8Array<ArrayBuffer> {
    const track = this.track(id);
    const bytes = callIndex(() => this.handle().avcBootstrap(JSON.stringify({track:track.ordinal,sample})));
    requireMedia(bytes instanceof Uint8Array && bytes.byteLength <= (1 << 20) + 1152, 'resource_limit');
    return bytes;
  }
  track(id: number, kind: 'video' | 'audio' = 'video'): RawMovieTrack {
    this.handle(); const track = this.tracks.find(track => track.id === id);
    requireMedia(track?.kind === kind, 'invalid_request'); return track;
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
  audioClock(id: number): MovieAudioClock {
    const track = this.track(id, 'audio'); requireMedia(track.edits <= 4096, 'resource_limit');
    const edits: MovieAudioClock['edits'][number][] = [];
    for (let start = 0; start < track.edits; start += 128) {
      const value = decode(() => this.handle().query(JSON.stringify({ type: 'edits', track: track.ordinal, start, count: 128 })));
      requireMedia(value.type === 'edits' && value.total === track.edits && Array.isArray(value.items)
        && value.items.length === Math.min(128, track.edits - start), 'unsupported_media');
      for (const item of value.items) {
        const edit = object(item);
        requireMedia(integer(edit.movieStart) >= 0n && integer(edit.movieDuration) >= 0n, 'invalid_timing');
        if (edit.mediaTime !== null) integer(edit.mediaTime);
        requireMedia(typeof edit.rate1616 === 'number' && Number.isInteger(edit.rate1616)
          && edit.rate1616 >= -2147483648 && edit.rate1616 <= 2147483647, 'invalid_timing');
        edits.push({ movieStart: edit.movieStart as string, movieDuration: edit.movieDuration as string,
          mediaTime: edit.mediaTime as string | null, rate1616: edit.rate1616 });
      }
    }
    return { trackId: track.id, movieTimescale: this.movieTimescale, mediaTimescale: track.timescale,
      declaredTrackDuration: track.declaredTrackDuration, edits };
  }
  packets(track: RawMovieTrack, start: number): RawMoviePacket[] {
    const value = decode(() => this.handle().query(JSON.stringify({ type: 'samples', track: track.ordinal, start, count: 128 })));
    requireMedia(value.type === 'samples' && value.total === track.samples && Array.isArray(value.items)
      && value.items.length === Math.min(128, track.samples - start), 'unsupported_media');
    return value.items.map(item => {
      const packet = object(item), bytes = object(packet.bytes);
      requireMedia(integer(bytes.offset) >= 0n && integer(bytes.length) > 0n, 'unsupported_media');
      integer(packet.dts); integer(packet.pts); u32(packet.description);
      requireMedia(u32(packet.decodeDuration) > 0 && typeof packet.syncSample === 'boolean', 'unsupported_media');
      return packet as unknown as RawMoviePacket;
    });
  }
  close(): void { const index = this.index; this.index = undefined; this.info.clear(); index?.free(); }
}
