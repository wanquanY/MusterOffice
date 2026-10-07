import {
  Input, BlobSource, MP4, WEBM, WAVE, MP3, OGG, FLAC,
  InputVideoTrack, InputAudioTrack,
  type InputTrack, type VideoSample, type AudioSample,
} from 'mediabunny';
import {
  BrowserMediaError, DEFAULT_MEDIA_LIMITS, requireMedia,
  type BrowserMediaLimits, type BrowserMediaInfo, type BrowserTrack,
  type ConsumeVideo, type ConsumeAudio, type ExactRatio, type TimeSample,
} from './contract.js';
import { compare, contains, interval, lookupSeconds } from './time.js';
import { probeTrack } from './probe.js';
import { audioView, videoView } from './samples.js';
import { BudgetedVideoStream } from './video-stream.js';
import { BrowserVideoBudget } from './video-budget.js';
import { BrowserAudioCursor } from './audio-cursor.js';

/** Media I/O only, intended for a host-owned terminable Worker. The host admits
 * immutable bytes and owns deadlines across Worker stalls. No URL, paths, storage,
 * project edits, credentials, UI, or media upload are accepted by this adapter. */
export class BrowserMediaInput {
  private readonly input: Input;
  private readonly tracks = new Map<number, { track: InputTrack; info: BrowserTrack }>();
  private readonly borrowed = new Set<VideoSample | AudioSample>();
  private readonly iterators = new Set<AsyncGenerator<VideoSample | AudioSample, void, unknown>>();
  private readonly lifetime = new AbortController();
  private active = false;
  private infoValue: BrowserMediaInfo | undefined;
  private detach: (() => void) | undefined;
  private audioCursor: { id: number; cursor: BrowserAudioCursor } | undefined;
  private constructor(blob: Blob, private readonly limits: BrowserMediaLimits, private readonly budget: BrowserVideoBudget) {
    this.input = new Input({
      formats: [MP4, WEBM, WAVE, MP3, OGG, FLAC],
      source: new BlobSource(blob, {
        maxCacheSize: limits.sourceCacheBytes,
        handleUnhandledError: () => this.close(),
      }),
    });
  }
  static async open(blob: Blob, options: { limits?: Partial<BrowserMediaLimits>; signal?: AbortSignal; videoBudget?: BrowserVideoBudget } = {}): Promise<BrowserMediaInput> {
    const limits = { ...DEFAULT_MEDIA_LIMITS, ...options.limits };
    for (const value of Object.values(limits)) requireMedia(Number.isSafeInteger(value) && value > 0, 'invalid_request');
    requireMedia(blob instanceof Blob && blob.size > 0 && blob.size <= limits.maxFileBytes, 'resource_limit');
    requireMedia(limits.operationDeadlineMs <= 120000, 'invalid_request');
    requireMedia(options.videoBudget === undefined || options.videoBudget instanceof BrowserVideoBudget, 'invalid_request');
    const owner = new BrowserMediaInput(blob, Object.freeze(limits), options.videoBudget ?? new BrowserVideoBudget());
    const abort = () => owner.close(new BrowserMediaError('cancelled'));
    options.signal?.addEventListener('abort', abort, { once: true });
    owner.detach = () => options.signal?.removeEventListener('abort', abort);
    if (options.signal?.aborted) abort();
    try {
      await owner.run(async () => {
        const tracks = await owner.input.getTracks();
        requireMedia(tracks.length > 0 && tracks.length <= limits.maxTracks, 'resource_limit');
        for (const track of tracks) {
          owner.check();
          requireMedia(!owner.tracks.has(track.id) && !(await track.isLive()), 'unsupported_media');
          const info = await probeTrack(track, limits);
          owner.tracks.set(track.id, { track, info });
        }
        owner.infoValue = { byteLength: blob.size, tracks: [...owner.tracks.values()].map(x => x.info) };
      });
      return owner;
    } catch (error) { owner.close(); throw error; }
  }
  get info(): BrowserMediaInfo { this.check(); return structuredClone(this.infoValue!); }
  private check(): void { if (this.lifetime.signal.aborted) throw this.lifetime.signal.reason; }
  private async run<T>(body: () => Promise<T>, signal?: AbortSignal): Promise<T> {
    this.check();
    requireMedia(!this.active, 'busy');
    this.active = true;
    const abort = () => this.close(new BrowserMediaError('cancelled'));
    signal?.addEventListener('abort', abort, { once: true });
    const timeout = setTimeout(() => this.close(new BrowserMediaError('deadline')), this.limits.operationDeadlineMs);
    let onClose: () => void = () => {};
    const cancelled = new Promise<never>((_, reject) => {
      onClose = () => reject(this.lifetime.signal.reason);
      this.lifetime.signal.addEventListener('abort', onClose, { once: true });
    });
    if (signal?.aborted) abort();
    try {
      return await Promise.race([cancelled, (async () => { this.check(); const result = await body(); this.check(); return result; })()]);
    } finally {
      clearTimeout(timeout);
      signal?.removeEventListener('abort', abort);
      this.lifetime.signal.removeEventListener('abort', onClose);
      this.active = false;
    }
  }
  private video(id: number) {
    const value = this.tracks.get(id);
    requireMedia(value?.track instanceof InputVideoTrack && value.info.kind === 'video', 'invalid_request');
    requireMedia(value.info.decodeConfigurationSupported, 'unsupported_codec');
    return { track: value.track, info: value.info };
  }
  private audio(id: number) {
    const value = this.tracks.get(id);
    requireMedia(value?.track instanceof InputAudioTrack && value.info.kind === 'audio', 'invalid_request');
    requireMedia(value.info.decodeConfigurationSupported, 'unsupported_codec');
    return { track: value.track, info: value.info };
  }
  private async borrow<S extends VideoSample | AudioSample, T>(sample: S, run: () => Promise<T>): Promise<T> {
    this.borrowed.add(sample);
    try { this.check(); return await run(); }
    finally { this.borrowed.delete(sample); sample.close(); }
  }
  private checkBorrowed(sample: VideoSample | AudioSample): void {
    this.check(); requireMedia(this.borrowed.has(sample), 'closed');
  }
  private async consume<S extends VideoSample | AudioSample>(samples: AsyncGenerator<S, void, unknown>, run: (sample: S) => Promise<boolean>): Promise<void> {
    this.iterators.add(samples);
    try {
      for await (const sample of samples) {
        if (!(await this.borrow(sample, () => run(sample)))) break;
      }
    } finally { this.iterators.delete(samples); await samples.return(); }
  }
  /** The prior sample is not stretched into a gap or past EOF. Exact one-sided
   * selection is taken directly from the Rust frame plan's TimeSample. */
  videoAt(id: number, request: TimeSample, consume: ConsumeVideo, signal?: AbortSignal): Promise<boolean> {
    request = structuredClone(request);
    return this.run(async () => {
      const { track, info } = this.video(id);
      const stream = await BudgetedVideoStream.open(track, lookupSeconds(request, info.timescale), this.budget, this.limits.maxEncodedPacketBytes, this.limits.maxPixels, this.lifetime.signal);
      for await (const sample of stream) {
        const result = await this.borrow(sample, async () => {
          const span = interval(sample.timestamp, sample.duration, info.timescale);
          if (!contains(span, request)) return compare(span.start, request.time) >= 0 ? 'absent' : 'continue';
          requireMedia(sample.visibleRect.width * sample.visibleRect.height <= this.limits.maxPixels, 'resource_limit');
          await consume(videoView(sample, () => this.checkBorrowed(sample), this.budget, stream.surfaceBytes), span); return 'found';
        });
        if (result !== 'continue') return result === 'found';
      }
      return false;
    }, signal);
  }
  videoRange(id: number, start: ExactRatio, end: ExactRatio, consume: ConsumeVideo, signal?: AbortSignal): Promise<number> {
    start = structuredClone(start); end = structuredClone(end);
    return this.run(async () => {
      const { track, info } = this.video(id);
      requireMedia(compare(start, end) < 0, 'invalid_request');
      const stream = await BudgetedVideoStream.open(track, lookupSeconds({ time: start, side: 'after' }, info.timescale), this.budget, this.limits.maxEncodedPacketBytes, this.limits.maxPixels, this.lifetime.signal);
      let count = 0;
      await this.consume(stream[Symbol.asyncIterator](), async sample => {
          const span = interval(sample.timestamp, sample.duration, info.timescale);
          if (compare(span.start, end) >= 0) return false;
          if (compare(span.end, start) <= 0) return true;
          requireMedia(sample.visibleRect.width * sample.visibleRect.height <= this.limits.maxPixels, 'resource_limit');
          count++; return (await consume(videoView(sample, () => this.checkBorrowed(sample), this.budget, stream.surfaceBytes), span)) !== false;
      });
      return count;
    }, signal);
  }
  audioRange(id: number, start: ExactRatio, end: ExactRatio, consume: ConsumeAudio, signal?: AbortSignal): Promise<number> {
    start = structuredClone(start); end = structuredClone(end);
    return this.run(async () => {
      const { track, info } = this.audio(id);
      requireMedia(compare(start, end) < 0, 'invalid_request');
      // Decoded audio lives on its own sample grid, which may differ from the
      // container's timescale. Blocks overlap the requested interval; Rust DSP
      // owns trimming, resampling, channel mapping and timeline placement.
      // An audio packet marked key does not certify a decoder-state restart:
      // AAC overlap, MP3 reservoirs and Opus preroll can depend on earlier data.
      // Reconstruct from the admitted stream start until the container adapter
      // supplies a proven roll-recovery point. Do not invent codec delay constants.
      if (this.audioCursor?.id !== id) {
        await this.audioCursor?.cursor.close();
        this.check();
        this.audioCursor = { id, cursor: new BrowserAudioCursor(track, Number(info.first.ticks) / info.timescale) };
      }
      return this.audioCursor.cursor.read(start, end, (sample, span) => this.borrow(sample, async () => {
          requireMedia(sample.sampleRate <= this.limits.maxAudioSampleRate
            && sample.numberOfChannels <= this.limits.maxAudioChannels
            && sample.numberOfFrames * sample.numberOfChannels * 4 <= this.limits.maxAudioBlockBytes, 'resource_limit');
          // The audited sink restores a sample-rate-aligned timestamp after
          // WebCodecs. Keep that grid; reducing it back to us would lose samples.
          return (await consume(audioView(sample, () => this.checkBorrowed(sample)), span)) !== false;
      }));
    }, signal);
  }
  /** Cancelling an operation disposes this source and every borrowed sample.
   * A replacement seek generation opens a new input over the same admitted Blob. */
  close(reason: BrowserMediaError = new BrowserMediaError('closed')): void {
    if (this.lifetime.signal.aborted) return;
    this.lifetime.abort(reason);
    this.detach?.(); this.detach = undefined;
    for (const sample of this.borrowed) sample.close();
    this.borrowed.clear();
    // A consumer may never finish its callback. Return the *upstream* iterator
    // directly rather than waiting for that callback to advance a for-await loop.
    for (const samples of this.iterators) void samples.return().catch(() => {});
    this.iterators.clear();
    void this.audioCursor?.cursor.close().catch(() => {}); this.audioCursor = undefined;
    this.input.dispose();
  }
}
