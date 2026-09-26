import type { HostRequest } from '../../contracts/src/generated/host-request.js';
import type { AssetDescriptor, AssetInfo, HostResponse, UploadInfo } from '../../contracts/src/generated/host-response.js';

/** Bound to one trusted host authority. Transport adapters validate wire data
 * against the generated contracts before returning these types. No implicit
 * credentials, files, endpoint, task queue or publication owner is created. */
export interface OperationPort {
  dispatch(request: HostRequest): Promise<HostResponse>;
  appendUpload(uploadId: string, offset: string, bytes: Uint8Array): Promise<UploadInfo>;
  readAssetRange(assetId: string, offset: string, length: number): Promise<Uint8Array>;
}

/** A host-owned immutable byte source; offsets never pass through Number. */
export interface ByteSource {
  readonly byteLength: bigint;
  readRange(offset: bigint, length: number): Promise<Uint8Array>;
}

export interface Sha256 {
  update(bytes: Uint8Array): void;
  finish(): string;
}
/** Host runtime crypto, e.g. native/WASM SHA256 or a Node streaming digest. */
export interface DigestFactory {
  sha256(): Sha256;
}

/** Private output, with backpressure. seal returns final stored bytes for
 * verification, not a hash of bytes merely offered to write. Publication and
 * transaction/fence checks always remain with the product's existing owner. */
export interface CandidateSink<Reference> {
  write(bytes: Uint8Array): Promise<void>;
  seal(): Promise<SealedCandidate<Reference>>;
  discard(): Promise<void>;
}
export interface SealedCandidate<Reference> {
  reference: Reference;
  source: ByteSource;
  /** Release verification reader handles; keep the sealed candidate bytes. */
  release(): Promise<void>;
}
export interface CandidateStore<Reference> {
  begin(asset: AssetInfo): Promise<CandidateSink<Reference>>;
}
export interface CopiedAsset<Reference> {
  asset: AssetInfo;
  reference: Reference;
}
export interface CallOptions {
  /** Stops local waiting only. It never calls jobs.cancel or cancels an upload. */
  signal?: AbortSignal;
}
export interface WaitOptions extends CallOptions {
  /** Finite wait budget, separate from durable business/job lifetime. */
  timeoutMs?: number;
}
export interface CopyOptions extends CallOptions {
  /** Pin from the selected immutable bundle/version, never an authority grant. */
  expectedDescriptor?: AssetDescriptor;
}
