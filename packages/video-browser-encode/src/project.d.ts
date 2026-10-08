import type { EncodeLimits, EncodeRuntime, EncodeSink, EncodeStats } from './index.mjs';
export interface VideoExportWork {
  kind:'video'; index:number; keyFrame:boolean; pts:string; duration:string; timescale:number;
}
export interface AudioExportWork { kind:'audio'; firstSample:string; frames:number; }
export interface CodecTransport { timestampUs:string; durationUs:string; }
export interface ProjectExport {
  info():string; next():string; accept(work:string):void; progress():string;
}
export interface ProjectEncodeRequest {
  /** Dedicated, immutable ProjectExport instance. Caller frees it after this call. */
  plan:ProjectExport; runtime:EncodeRuntime; sink:EncodeSink; limits:EncodeLimits;
  signal?:AbortSignal; codec?:string;
  budgets?:{ maxFramesInFlight?:number; maxVideoInputBytes?:number; maxCodecQueue?:number; maxBufferedPcmBytes?:number; stallTimeoutMs?:number };
  /** Ownership of the returned I420 frame transfers; it is always closed. */
  createFrame(work:VideoExportWork, transport:CodecTransport, signal:AbortSignal):Promise<VideoFrame>;
  createPcm(work:AudioExportWork, signal:AbortSignal):Promise<Float32Array>;
  onProgress?(progress:{ videoFrames:number; audioSamples:string }):void;
}
export function encodeProject(request:ProjectEncodeRequest):Promise<Readonly<{
  stats:EncodeStats; documentDigest:string; peakFramesInFlight:number; peakVideoInputBytes:number; peakBufferedPcmBytes:number;
}>>;
