import { EncodedPacket, type InputVideoTrack } from 'mediabunny';
import { requireMedia } from './contract.js';
import { BrowserMovieIndex, integer, type RawMoviePacket, type RawMovieTrack } from './movie-index.js';
import type { MovieVideoSelection } from './movie-contract.js';
import type { VideoPackets } from './video-packets.js';
import { avcCapacity, type VideoCapacity } from './mediabunny-platform.js';
import type { BrowserVideoBudget } from './video-budget.js';
type Selection = Extract<MovieVideoSelection, {type: 'sample'}>;

/** Decode-order raw access units. Container sync flags and recovery SEI are not
 * closed-GOP proof. Start at a verified IDR, including when this requires going
 * earlier than Rust's candidate. No movie offset ever enters these timestamps. */
export class IndexedVideoPackets implements VideoPackets {
  private page: RawMoviePacket[] = [];
  private start = -1;
  private constructor(private readonly blob: Blob, private readonly index: BrowserMovieIndex, private readonly track: RawMovieTrack,
    private readonly selection: Selection, private readonly codec: VideoCapacity,
    private readonly maxBytes: number, private readonly signal: AbortSignal, private readonly budget: BrowserVideoBudget) {}
  static async open(blob: Blob, index: BrowserMovieIndex, input: InputVideoTrack, selection: Selection, maxBytes: number, signal: AbortSignal, budget: BrowserVideoBudget): Promise<IndexedVideoPackets> {
    const track = index.track(input.id), config = await input.getDecoderConfig(); signal.throwIfAborted();
    requireMedia(track.descriptions === 1 && track.timescale === await input.getTimeResolution(), 'unsupported_media');
    requireMedia(await input.getCodec() === 'avc' && config, 'unsupported_codec');
    return new IndexedVideoPackets(blob, index, track, selection, avcCapacity(config), maxBytes, signal, budget);
  }
  private metadata(ordinal: number): RawMoviePacket {
    this.signal.throwIfAborted();
    requireMedia(Number.isInteger(ordinal) && ordinal >= 0 && ordinal < this.track.samples, 'invalid_request');
    if (ordinal < this.start || ordinal >= this.start + this.page.length) {
      this.start = Math.floor(ordinal / 128) * 128; this.page = this.index.packets(this.track, this.start);
    }
    return this.page[ordinal - this.start]!;
  }
  private seconds(ticks: bigint): number {
    const value = Number(ticks), seconds = value / this.track.timescale;
    // WebCodecs' microsecond clock must distinguish the supplied container tick.
    requireMedia(Number.isSafeInteger(value) && Number.isSafeInteger(Math.round(seconds * 1e6))
      && BigInt(Math.round(seconds * this.track.timescale)) === ticks
      && BigInt(Math.round(Math.round(seconds * 1e6) / 1e6 * this.track.timescale)) === ticks, 'invalid_timing');
    return seconds;
  }
  private async load(ordinal: number): Promise<EncodedPacket> {
    const meta = this.metadata(ordinal), offset = integer(meta.bytes.offset), length = integer(meta.bytes.length);
    requireMedia(meta.description === 0, 'unsupported_media');
    requireMedia(length <= BigInt(this.maxBytes) && offset + length <= BigInt(this.blob.size), 'resource_limit');
    await this.index.scanAvc(this.track.id, ordinal, this.signal);
    const interpretation = this.index.avcSample(this.track.id, ordinal);
    const timestamp = this.seconds(integer(meta.pts)), duration = this.seconds(BigInt(meta.decodeDuration));
    const bytes = new Uint8Array(await this.blob.slice(Number(offset), Number(offset + length)).arrayBuffer());
    this.signal.throwIfAborted(); requireMedia(bytes.byteLength === Number(length), 'unsupported_media');
    this.codec.validatePacket(bytes);
    return new EncodedPacket(bytes, interpretation.picture?.idr ? 'key' : 'delta', timestamp, duration, ordinal);
  }
  async first(): Promise<EncodedPacket | null> {
    const target = this.metadata(this.selection.sample);
    requireMedia(target.pts === this.selection.rawStart && target.dts === this.selection.decodeStart, 'invalid_timing');
    for (let at = this.selection.access?.sample ?? this.selection.sample; at >= 0; at--) {
      const meta = this.metadata(at);
      if (!meta.syncSample || integer(meta.pts) > integer(this.selection.rawStart)) continue;
      const packet = await this.load(at);
      if (packet.type === 'key') {
        // Reserve the bounded ABI result before WASM copies it to JS. The
        // stream's source slot holds packet; construction needs one extra copy.
        const releasePrefix = this.budget.reserve((1 << 20) + 1152);
        try {
          const prefix = this.index.avcBootstrap(this.track.id, at);
          const length = prefix.length + packet.data.length;
          requireMedia(length <= this.maxBytes, 'resource_limit');
          const releaseCopy = this.budget.reserve(length);
          try {
            const bytes = new Uint8Array(length); bytes.set(prefix); bytes.set(packet.data, prefix.length);
            this.codec.validatePacket(bytes);
            return new EncodedPacket(bytes, 'key', packet.timestamp, packet.duration, packet.sequenceNumber);
          } finally { releaseCopy(); }
        } finally { releasePrefix(); }
      }
    }
    return null;
  }
  async next(previous: EncodedPacket): Promise<EncodedPacket | null> {
    this.signal.throwIfAborted();
    const ordinal = previous.sequenceNumber + 1;
    return ordinal === this.track.samples ? null : this.load(ordinal);
  }
}
