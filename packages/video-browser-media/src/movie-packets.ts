import { EncodedPacket, type InputVideoTrack } from 'mediabunny';
import { iterateAvcNalUnits, extractNalUnitTypeForAvc } from '@musteroffice/mediabunny-codec-data';
import { requireMedia } from './contract.js';
import { BrowserMovieIndex, integer, type RawVideoPacket, type RawVideoTrack } from './movie-index.js';
import type { MovieVideoSelection } from './movie-contract.js';
import type { VideoPackets } from './video-packets.js';
import { avcCapacity, type VideoCapacity } from './mediabunny-platform.js';
type Selection = Extract<MovieVideoSelection, {type: 'sample'}>;

/** Decode-order raw access units. Container sync flags and recovery SEI are not
 * closed-GOP proof. Start at a verified IDR, including when this requires going
 * earlier than Rust's candidate. No movie offset ever enters these timestamps. */
export class IndexedVideoPackets implements VideoPackets {
  private page: RawVideoPacket[] = [];
  private start = -1;
  private constructor(private readonly blob: Blob, private readonly index: BrowserMovieIndex, private readonly track: RawVideoTrack,
    private readonly selection: Selection, private readonly config: VideoDecoderConfig, private readonly codec: VideoCapacity,
    private readonly maxBytes: number, private readonly signal: AbortSignal) {}
  static async open(blob: Blob, index: BrowserMovieIndex, input: InputVideoTrack, selection: Selection, maxBytes: number, signal: AbortSignal): Promise<IndexedVideoPackets> {
    const track = index.track(input.id), config = await input.getDecoderConfig(); signal.throwIfAborted();
    requireMedia(track.descriptions === 1 && track.timescale === await input.getTimeResolution(), 'unsupported_media');
    requireMedia(await input.getCodec() === 'avc' && config, 'unsupported_codec');
    return new IndexedVideoPackets(blob, index, track, selection, config, avcCapacity(config), maxBytes, signal);
  }
  private metadata(ordinal: number): RawVideoPacket {
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
    const timestamp = this.seconds(integer(meta.pts)), duration = this.seconds(BigInt(meta.decodeDuration));
    const bytes = new Uint8Array(await this.blob.slice(Number(offset), Number(offset + length)).arrayBuffer());
    this.signal.throwIfAborted(); requireMedia(bytes.byteLength === Number(length), 'unsupported_media');
    this.codec.validatePacket(bytes);
    let idr = false, dependent = false;
    for (const nal of iterateAvcNalUnits(bytes, this.config)) {
      const kind = extractNalUnitTypeForAvc(bytes[nal.offset]!);
      idr ||= kind === 5; dependent ||= kind >= 1 && kind <= 4;
    }
    requireMedia(!(idr && dependent), 'unsupported_codec');
    return new EncodedPacket(bytes, idr ? 'key' : 'delta', timestamp, duration, ordinal);
  }
  async first(): Promise<EncodedPacket | null> {
    const target = this.metadata(this.selection.sample);
    requireMedia(target.pts === this.selection.rawStart && target.dts === this.selection.decodeStart, 'invalid_timing');
    for (let at = this.selection.access?.sample ?? this.selection.sample; at >= 0; at--) {
      const meta = this.metadata(at);
      if (!meta.syncSample || integer(meta.pts) > integer(this.selection.rawStart)) continue;
      const packet = await this.load(at);
      if (packet.type === 'key') return packet;
    }
    return null;
  }
  async next(previous: EncodedPacket): Promise<EncodedPacket | null> {
    this.signal.throwIfAborted();
    const ordinal = previous.sequenceNumber + 1;
    return ordinal === this.track.samples ? null : this.load(ordinal);
  }
}
