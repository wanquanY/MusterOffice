import { type InputVideoTrack, type VideoSample, type EncodedPacket } from 'mediabunny';
import { requireMedia } from './contract.js';
import { openPlatformVideo, videoCapacity, type VideoCapacity } from './mediabunny-platform.js';
import { BrowserVideoBudget } from './video-budget.js';
import { AdmittedVideoPackets } from './video-packets.js';

/** A lease is reserved BEFORE configuring a decoder. Every submitted access unit
 * retains an output credit through decode, reorder, queued output and consumption.
 * decodeQueueSize alone is insufficient: it omits codec-internal pending output. */
export class BudgetedVideoStream implements AsyncIterable<VideoSample> {
  private decoder: Awaited<ReturnType<typeof openPlatformVideo>> | undefined;
  private readonly ready: VideoSample[] = [];
  private readonly waiters = new Set<() => void>();
  private pending = 0;
  private reserved = 0;
  private closed = false;
  private done = false;
  private failure: unknown;
  private release: (() => void) | undefined;
  private active: VideoSample | undefined;
  private detach: (() => void) | undefined;
  private tasks = 1;
  private readonly packetLeases = new Map<string, (() => void)[]>();
  private constructor(private readonly capacity: number, readonly surfaceBytes: number, private readonly codec: VideoCapacity,
    private readonly maxPacketBytes: number, private readonly timescale: number, private readonly budget: BrowserVideoBudget) {}
  static async open(track: InputVideoTrack, timestamp: number, budget: BrowserVideoBudget, maxPacketBytes: number, maxPixels: number, signal: AbortSignal): Promise<BudgetedVideoStream> {
    signal.throwIfAborted();
    const info = await videoCapacity(track);
    requireMedia(info.pixels <= maxPixels, 'resource_limit');
    signal.throwIfAborted();
    // Reference storage plus a full codec-dependent reorder window, one current
    // consumer and one next-frame slot. This is not a universal N-frame constant.
    const slots = Math.max(info.references, info.reorder) + 2;
    const stream = new BudgetedVideoStream(slots, info.surfaceBytes, info, maxPacketBytes, await track.getTimeResolution(), budget);
    signal.throwIfAborted();
    stream.release = budget.reserve((info.references + slots) * info.surfaceBytes + maxPacketBytes);
    const abort = () => stream.close(signal.reason);
    signal.addEventListener('abort', abort, { once: true });
    stream.detach = () => signal.removeEventListener('abort', abort);
    if (signal.aborted) abort();
    try {
      const packets = new AdmittedVideoPackets(track, maxPacketBytes, stream.timescale);
      const first = await packets.first(timestamp);
      signal.throwIfAborted();
      requireMedia(first, 'unsupported_media');
      stream.decoder = await openPlatformVideo(track, sample => stream.output(sample), error => stream.close(error));
      if (stream.closed) { stream.decoder.close(); signal.throwIfAborted(); throw stream.failure; }
      stream.tasks++;
      void stream.pump(packets, first).catch(error => stream.close(error)).finally(() => stream.taskEnded());
      return stream;
    } catch (error) { stream.close(error); throw error; }
    finally { stream.taskEnded(); }
  }
  private wake(): void { for (const resolve of this.waiters) resolve(); this.waiters.clear(); }
  private changed(): Promise<void> { return new Promise(resolve => this.waiters.add(resolve)); }
  private output(sample: VideoSample): void {
    if (this.closed) { sample.close(); return; }
    try {
      requireMedia(this.pending > 0, 'unsupported_media');
      requireMedia(sample.allocationSize() <= this.surfaceBytes, 'resource_limit');
      const key = this.packetKey(sample.timestamp), leases = this.packetLeases.get(key);
      requireMedia(leases?.length, 'invalid_timing');
      leases.shift()!(); if (!leases.length) this.packetLeases.delete(key);
      this.pending--; this.ready.push(sample); this.wake();
    } catch (error) { sample.close(); this.close(error); }
  }
  private async pump(packets: AdmittedVideoPackets, initial: EncodedPacket): Promise<void> {
    let packet: EncodedPacket | null = initial;
    while (packet && !this.closed) {
      while (this.reserved >= this.capacity && !this.closed) await this.changed();
      if (this.closed) return;
      requireMedia(packet.byteLength > 0 && packet.byteLength <= this.maxPacketBytes, 'resource_limit');
      requireMedia(!packet.sideData.alpha, 'unsupported_codec');
      this.codec.validatePacket(packet.data);
      // Both the source packet and WebCodecs' encoded copy can be in flight.
      // Unlike decodeQueueSize, this charge survives internal codec buffering.
      const key = this.packetKey(packet.timestamp), release = this.budget.reserve(packet.byteLength * 2);
      const leases = this.packetLeases.get(key) ?? []; leases.push(release); this.packetLeases.set(key, leases);
      this.pending++; this.reserved++;
      this.decoder!.decode(packet);
      packet = await packets.next(packet);
    }
    if (!this.closed) {
      await this.decoder!.flush();
      if (this.closed) return;
      // Non-picture access units may yield no output. Their reservations remain
      // live until flush proves there can be no more delayed frames.
      this.reserved -= this.pending; this.pending = 0;
      this.releasePackets();
      this.done = true; this.wake();
    }
  }
  async *[Symbol.asyncIterator](): AsyncGenerator<VideoSample, void, unknown> {
    try {
      for (;;) {
        if (this.failure) throw this.failure;
        if (this.closed) return;
        const sample = this.ready.shift();
        if (sample) {
          this.active = sample;
          try { yield sample; }
          finally {
            sample.close(); this.active = undefined;
            if (!this.closed) this.reserved--;
            this.wake();
          }
        } else if (this.done) return;
        else await this.changed();
      }
    } finally { this.close(); }
  }
  close(error?: unknown): void {
    if (this.closed) return;
    this.closed = true;
    this.failure = error;
    this.detach?.(); this.detach = undefined;
    try { this.decoder?.close(); } catch (error) { this.failure ??= error; } finally { this.decoder = undefined; }
    this.active?.close(); this.active = undefined;
    for (const sample of this.ready) sample.close();
    this.ready.length = 0; this.pending = 0; this.reserved = 0;
    this.releasePackets();
    this.releaseIfClosed();
    this.wake();
  }
  private taskEnded(): void { this.tasks--; this.releaseIfClosed(); }
  private packetKey(timestamp: number): string {
    const ticks = Math.round(timestamp * this.timescale);
    requireMedia(Number.isSafeInteger(ticks), 'invalid_timing'); return String(ticks);
  }
  private releasePackets(): void {
    for (const leases of this.packetLeases.values()) for (const release of leases) release();
    this.packetLeases.clear();
  }
  private releaseIfClosed(): void {
    if (this.closed && this.tasks === 0) { this.release?.(); this.release = undefined; }
  }
}
