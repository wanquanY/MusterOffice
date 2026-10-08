/** Exact integers are never represented by non-integral floating point seconds. */
export type ExactInteger = string | bigint | number;
export interface EncodeConfig {
  width:number; height:number; videoTimeScale:number; frameRateNumerator:number; frameRateDenominator:number;
  videoDuration:ExactInteger; videoFrames:ExactInteger; videoBitRate:ExactInteger; gopFrames:number;
  audioSampleRate:number; audioChannels:number; audioBitRate:number; audioSamples:ExactInteger;
}
export interface EncodeLimits {
  maxFileBytes:ExactInteger; maxWriteBytes:ExactInteger; maxPendingBytes:ExactInteger; maxFrameBytes:ExactInteger;
  maxPixels:number; maxPackets:number; maxPacketBytes:number; maxPendingPackets:number; maxSteps:number; maxWriteCalls:number;
}
export interface EncodeSink {
  /** Borrowed bytes must not be retained. Return a positive short/full write. */
  writeAt(offset:bigint, bytes:Uint8Array):number;
  cancelled():boolean;
}
export interface EncodeStats {
  byteLength:string; writeBytes:string; videoFrames:string; audioSamples:string;
  videoPackets:string; audioPackets:string; peakPendingBytes:string;
}
export interface EncodeSession {
  video(pts:ExactInteger, duration:ExactInteger, bytes:Uint8Array, key:boolean):void;
  audio(firstSample:ExactInteger, interleavedPcm:Float32Array):void;
  /** Candidate only. Does not flush host storage, verify or publish a file. */
  finish():EncodeStats;
  close():void;
}
export interface EncodeRuntime {
  open(config:EncodeConfig, limits:EncodeLimits, sink:EncodeSink, avcDescription:Uint8Array):EncodeSession;
}
export function createEncodeRuntime(module:WebAssembly.Module):Promise<EncodeRuntime>;
