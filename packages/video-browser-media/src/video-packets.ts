import { EncodedPacketSink, type EncodedPacket, type InputVideoTrack } from 'mediabunny';
import { requireMedia } from './contract.js';

/** Packet source owns container addressing. The bounded decoder owns allocation
 * and lifecycle; it must not infer a demuxer's timestamp convention. */
export interface VideoPackets {
  first(timestamp: number): Promise<EncodedPacket | null>;
  next(previous: EncodedPacket): Promise<EncodedPacket | null>;
}

/** Admit the declared packet size before requesting compressed bytes. Key flags
 * from a container are candidates, not proof; verify only after bounded loading. */
export class AdmittedVideoPackets implements VideoPackets {
  private readonly sink: EncodedPacketSink;
  constructor(private readonly track: InputVideoTrack, private readonly maxBytes: number, private readonly timescale: number) {
    this.sink = new EncodedPacketSink(track);
  }
  private admit(packet: EncodedPacket): void {
    requireMedia(Number.isSafeInteger(packet.byteLength) && packet.byteLength > 0 && packet.byteLength <= this.maxBytes, 'resource_limit');
    requireMedia(!packet.sideData.alpha && !packet.sideData.alphaByteLength, 'unsupported_codec');
  }
  private async load(metadata: EncodedPacket, read: () => Promise<EncodedPacket | null>): Promise<EncodedPacket> {
    this.admit(metadata);
    const packet = await read();
    requireMedia(packet && !packet.isMetadataOnly && packet.sequenceNumber === metadata.sequenceNumber
      && packet.byteLength === metadata.byteLength && packet.timestamp === metadata.timestamp && packet.duration === metadata.duration, 'unsupported_media');
    this.admit(packet);
    const type = await this.track.determinePacketType(packet);
    requireMedia(type, 'unsupported_codec');
    return type === packet.type ? packet : packet.clone({ type });
  }
  async first(timestamp: number): Promise<EncodedPacket | null> {
    let cursor = timestamp;
    for (;;) {
      const metadata = await this.sink.getKeyPacket(cursor, { metadataOnly: true });
      if (!metadata) break;
      const packet = await this.load(metadata, () => this.sink.getKeyPacket(cursor));
      if (packet.type === 'key') return packet;
      const previous = metadata.timestamp - 1 / this.timescale;
      requireMedia(previous < cursor, 'invalid_timing'); cursor = previous;
    }
    let metadata = await this.sink.getFirstKeyPacket({ metadataOnly: true });
    let read = () => this.sink.getFirstKeyPacket();
    while (metadata) {
      const packet = await this.load(metadata, read);
      if (packet.type === 'key') return packet;
      metadata = await this.sink.getNextKeyPacket(packet, { metadataOnly: true });
      read = () => this.sink.getNextKeyPacket(packet);
    }
    return null;
  }
  async next(previous: EncodedPacket): Promise<EncodedPacket | null> {
    const metadata = await this.sink.getNextPacket(previous, { metadataOnly: true });
    return metadata ? this.load(metadata, () => this.sink.getNextPacket(previous)) : null;
  }
}
