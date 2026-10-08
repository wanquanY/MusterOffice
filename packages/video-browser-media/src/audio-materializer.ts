import type { InputAudioTrack } from 'mediabunny';
import { BrowserMediaError, requireMedia, type BrowserMediaLimits } from './contract.js';
import type { BrowserPcmRuntime, BrowserPcmOrigin, BrowserPcmDecode, BrowserPcmCompletion, WritePcmTile } from './pcm-contract.js';
import { BrowserMovieIndex, integer, type RawMovieTrack, type RawMoviePacket } from './movie-index.js';
import { BrowserMediaBudget } from './media-budget.js';
import { aacCapacity, type AacCapacity } from './audio-platform.js';
import { completion, pcmCall } from './pcm-completion.js';

const PROFILE = 'musteroffice.browser.raw-aac-pcm/1-draft';
const PCM_WORKING = 4 << 20, MOVIE_VALIDATION = 12 << 20;
interface Pending { timestamp: number; delivered: number; release(): void; }

/** One fresh, continuous decode to EOF. No random AAC key-packet restart and
 * no intermediate flush. Storage is a bounded synchronous host capability;
 * durable commit is separate and is never performed by this media adapter. */
export class BrowserAudioMaterializer {
  private decoder: AudioDecoder | undefined;
  private core: BrowserPcmDecode | undefined;
  private codec: AacCapacity | undefined;
  private readonly pending: Pending[] = [];
  private readonly waiters = new Set<() => void>();
  private closed = false;
  private failure: unknown;
  private hasFailure = false;
  private callbackFailure: unknown;
  private callbackFailed = false;
  private blocks = 0;
  private tiles = 0;
  private constructor(private readonly budget: BrowserMediaBudget, private readonly limits: BrowserMediaLimits,
    private readonly signal: AbortSignal, private readonly write: WritePcmTile) {}
  static async run(blob: Blob, index: BrowserMovieIndex, input: InputAudioTrack, runtime: BrowserPcmRuntime,
    origin: BrowserPcmOrigin, write: WritePcmTile, budget: BrowserMediaBudget, limits: BrowserMediaLimits, signal: AbortSignal): Promise<BrowserPcmCompletion> {
    const owner = new BrowserAudioMaterializer(budget, limits, signal, write);
    const abort = () => owner.close(signal.reason);
    signal.addEventListener('abort', abort, { once: true });
    if (signal.aborted) abort();
    let result: BrowserPcmCompletion | undefined;
    try {
      result = await owner.decode(blob, index, input, runtime, origin);
      signal.throwIfAborted();
      if (owner.hasFailure) throw owner.failure;
      return result;
    } catch (error) { result?.close(); throw error; }
    finally { signal.removeEventListener('abort', abort); owner.close(); }
  }
  private check(): void {
    this.signal.throwIfAborted();
    if (this.hasFailure) throw this.failure;
    requireMedia(!this.closed, 'closed');
  }
  private wake(): void { for (const resolve of this.waiters) resolve(); this.waiters.clear(); }
  private changed(): Promise<void> { return new Promise(resolve => this.waiters.add(resolve)); }
  private close(error?: unknown): void {
    if (this.closed) return;
    this.closed = true;
    if (arguments.length) { this.hasFailure = true; this.failure = error; }
    try { if (this.decoder?.state !== 'closed') this.decoder?.close(); }
    catch (error) { if (!this.hasFailure) { this.hasFailure = true; this.failure = error; } }
    finally {
      this.decoder = undefined;
      for (const pending of this.pending) pending.release(); this.pending.length = 0;
      this.wake();
    }
    // Do not free Rust here: an abort may reenter from the synchronous write
    // callback while Rust is borrowed. The running call's finally owns cleanup.
  }
  private readonly writeTile: WritePcmTile = (reference, bytes) => {
    this.check();
    requireMedia(typeof reference === 'string' && reference.length <= 4096 && bytes instanceof Uint8Array
      && bytes.length > 0 && bytes.length <= 4096 * this.codec!.channels * 4, 'resource_limit');
    try {
      const result: unknown = this.write(reference, bytes);
      requireMedia(result === true, 'invalid_request'); this.check(); this.tiles++; return true;
    } catch (error) { this.callbackFailed = true; this.callbackFailure = error; throw error; }
  };
  private output(data: AudioData): void {
    try {
      this.check(); const codec = this.codec!;
      requireMedia(data.sampleRate === codec.sampleRate && data.numberOfChannels === codec.channels, 'unsupported_codec');
      requireMedia(Number.isSafeInteger(data.timestamp) && data.numberOfFrames > 0
        && data.numberOfFrames <= codec.frameSamples, 'resource_limit');
      // Match an actual output to an admitted packet. Keep the current packet's
      // encoded lease until a later packet produces output (or successful EOF).
      // Partial priming trim must not prematurely acknowledge its own packet.
      let at = -1;
      for (let i = 0; i < this.pending.length; i++) if (this.pending[i]!.timestamp <= data.timestamp + 1) at = i;
      requireMedia(at >= 0, 'invalid_timing');
      const packet = this.pending[at]!;
      requireMedia(data.timestamp < packet.timestamp + Math.ceil(codec.frameSamples * 1e6 / codec.sampleRate) + 1
        && packet.delivered + data.numberOfFrames <= codec.frameSamples, 'invalid_timing');
      packet.delivered += data.numberOfFrames;
      const pcm = new Float32Array(data.numberOfFrames * codec.channels);
      data.copyTo(pcm, { planeIndex: 0, format: 'f32' });
      // Explicit LE bytes; typed-array native endianness is not a wire promise.
      const bytes = new Uint8Array(pcm.byteLength), view = new DataView(bytes.buffer);
      for (let i = 0; i < pcm.length; i++) view.setFloat32(i * 4, pcm[i]!, true);
      pcmCall(() => this.core!.push(BigInt(data.timestamp), data.numberOfFrames, codec.sampleRate, codec.channels, bytes, this.writeTile));
      this.check(); this.blocks++;
      for (let i = 0; i < at; i++) this.pending.shift()!.release();
    } catch (error) { this.close(this.callbackFailed ? this.callbackFailure : error); }
    finally { data.close(); this.wake(); }
  }
  private async decode(blob: Blob, index: BrowserMovieIndex, input: InputAudioTrack,
    runtime: BrowserPcmRuntime, origin: BrowserPcmOrigin): Promise<BrowserPcmCompletion> {
    let releaseCompute: (() => void) | undefined, releaseCodec: (() => void) | undefined, releaseValidation: (() => void) | undefined;
    let transferred = false;
    try {
      this.check();
      requireMedia(runtime.profile === 'musteroffice.pcm-worker/1-draft' && typeof this.write === 'function', 'invalid_request');
      requireMedia(typeof AudioDecoder === 'function' && typeof EncodedAudioChunk === 'function', 'unsupported_codec');
      for (const digest of [origin.content, origin.decoderProfile]) requireMedia(typeof digest === 'string' && /^[a-f0-9]{64}$/.test(digest), 'invalid_request');
      const track = index.track(input.id, 'audio');
      requireMedia(track.samples > 0 && track.descriptions === 1 && track.timescale === await input.getTimeResolution(), 'unsupported_media');
      this.check(); const codec = await aacCapacity(input, this.limits); this.check(); this.codec = codec;
      const clock = JSON.stringify(index.audioClock(input.id)); requireMedia(clock.length <= 1 << 20, 'resource_limit');
      requireMedia(this.limits.maxPendingAudioPackets >= 2 && this.limits.maxPendingAudioPackets <= 256, 'invalid_request');
      // All explicit PCM copies/carry and codec output slots are admitted before
      // creating/configuring a decoder. An owned completion keeps the compute
      // lease; decoder/queue reservations retire after actual pending I/O exits.
      releaseCompute = this.budget.reserve(PCM_WORKING + clock.length * 2 + 32768);
      releaseValidation = this.budget.reserve(MOVIE_VALIDATION);
      releaseCodec = this.budget.reserve(codec.decoderWeight + this.limits.maxPendingAudioPackets * codec.frameBytes * 3 + codec.description.length * 2);
      let page = index.packets(track, 0), start = 0;
      const profile = new TextEncoder().encode(JSON.stringify([PROFILE, origin.decoderProfile,
        codec.config.codec, Array.from(codec.description), codec.sampleRate, codec.channels, codec.frameSamples]));
      const hash = new Uint8Array(await crypto.subtle.digest('SHA-256', profile)); this.check();
      const decoderProfile = Array.from(hash, byte => byte.toString(16).padStart(2, '0')).join('');
      const options = JSON.stringify({ resource: origin.resource, content: origin.content, stream: origin.stream, decoderProfile,
        trackId: track.id, rawOrigin: { numerator: page[0]!.pts, denominator: String(track.timescale) },
        sampleRate: codec.sampleRate, channels: codec.channels });
      requireMedia(options.length <= 16384, 'resource_limit');
      this.core = pcmCall(() => runtime.PcmDecode.open(options));
      const support = await AudioDecoder.isConfigSupported(codec.config); this.check();
      requireMedia(support.supported, 'unsupported_codec');
      let lastTimestamp = -1;
      for (let ordinal = 0; ordinal < track.samples; ordinal++) {
        this.check();
        while (this.pending.length >= this.limits.maxPendingAudioPackets) { await this.changed(); this.check(); }
        if (ordinal >= start + page.length) { start = ordinal; page = index.packets(track, start); }
        const timestamp = await this.submit(blob, track, page[ordinal - start]!, lastTimestamp); lastTimestamp = timestamp;
      }
      // Only EOF drains the codec. Intermediate flush changes AAC overlap state
      // on actual browsers, even without configure/reset.
      await this.decoder!.flush(); this.check();
      const record = pcmCall(() => this.core!.finish(this.writeTile)); this.check();
      requireMedia(typeof record === 'string' && record.length <= 16384, 'resource_limit');
      let movieInfo: string;
      try {
        const movie = pcmCall(() => runtime.MoviePcm.open(options, record, clock, () => undefined, () => { throw new BrowserMediaError('pending_source'); }));
        try { movieInfo = pcmCall(() => movie.info()); requireMedia(typeof movieInfo === 'string' && movieInfo.length <= 16384, 'resource_limit'); }
        finally { movie.free(); }
      } finally { releaseValidation(); releaseValidation = undefined; }
      this.check();
      const result = completion(this.core, releaseCompute, { options, clock, record, movieInfo,
        decodedPackets: track.samples, decodedBlocks: this.blocks, writtenTiles: this.tiles });
      transferred = true; this.core = undefined; releaseCompute = undefined;
      return result;
    } catch (error) {
      if (this.signal.aborted) throw this.signal.reason;
      if (this.callbackFailed) throw this.callbackFailure;
      if (this.hasFailure) throw this.failure;
      throw error;
    } finally {
      this.close();
      try { if (!transferred) this.core?.free(); } finally { this.core = undefined; releaseCodec?.(); releaseValidation?.(); releaseCompute?.(); }
    }
  }
  private async submit(blob: Blob, track: RawMovieTrack, packet: RawMoviePacket, previous: number): Promise<number> {
    requireMedia(packet.description === 0 && packet.pts === packet.dts, 'unsupported_media');
    const offset = integer(packet.bytes.offset), length = integer(packet.bytes.length);
    requireMedia(length <= BigInt(this.limits.maxEncodedPacketBytes) && offset + length <= BigInt(blob.size), 'resource_limit');
    const ticks = pcmCall(() => this.core!.packetTimestamp(integer(packet.pts), track.timescale)), timestamp = Number(ticks);
    requireMedia(typeof ticks === 'bigint' && Number.isSafeInteger(timestamp) && BigInt(timestamp) === ticks
      && timestamp > previous && timestamp <= this.limits.maxDurationSeconds * 1e6, 'invalid_timing');
    // Charge the source ArrayBuffer and browser's encoded copy BEFORE Blob I/O.
    // Reading lease is deliberately outside pending[] so close cannot release
    // it while arrayBuffer() is still in flight.
    let release: (() => void) | undefined = this.budget.reserve(Number(length) * 2);
    try {
      const bytes = await blob.slice(Number(offset), Number(offset + length)).arrayBuffer(); this.check();
      requireMedia(bytes.byteLength === Number(length), 'unsupported_media');
      if (!this.decoder) {
        this.decoder = new AudioDecoder({ output: data => this.output(data), error: () => this.close(new BrowserMediaError('unsupported_media')) });
        this.decoder.configure(this.codec!.config);
      }
      this.pending.push({ timestamp, delivered: 0, release }); release = undefined;
      this.decoder!.decode(new EncodedAudioChunk({ type: 'key', timestamp, data: bytes }));
      this.check(); return timestamp;
    } finally { release?.(); }
  }
}
