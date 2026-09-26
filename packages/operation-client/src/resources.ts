import type { UploadRequest } from '../../contracts/src/generated/host-request.js';
import type { AssetInfo, UploadInfo } from '../../contracts/src/generated/host-response.js';
import { OfficeClient } from './client.js';
import type { ByteSource, CallOptions, CopyOptions, CandidateStore, CopiedAsset, DigestFactory } from './ports.js';
import { checkAbort, ClientError, requireState, waiting } from './errors.js';
import { asset, byteLength, chunkLength, descriptor, positiveBound, sameDescriptor } from './scalars.js';
import { verifySealed } from './verification.js';

export interface ResourceLimits {
  /** Client memory bound for one binary chunk, not a replacement host quota. */
  chunkBytes?: number;
  maxAssetBytes?: bigint;
}

/** Sequential binary transfer, with no background queue and no publication. */
export class ResourceClient {
  readonly chunkBytes: number;
  readonly maxAssetBytes: bigint;
  constructor(readonly client: OfficeClient, limits: ResourceLimits = {}) {
    this.chunkBytes = limits.chunkBytes ?? 262_144;
    this.maxAssetBytes = limits.maxAssetBytes ?? 1_073_741_824n;
    positiveBound(this.chunkBytes, 16 * 1024 * 1024, 'binary chunk budget');
    if (this.maxAssetBytes < 0n || this.maxAssetBytes > 18446744073709551615n)
      throw new ClientError('INPUT_INVALID', 'Invalid asset byte budget');
  }

  private budget(length: bigint): void {
    if (length > this.maxAssetBytes) throw new ClientError('LIMIT_EXCEEDED', 'Client asset byte budget');
  }

  /** Retry with the SAME requestId and immutable source. The host tells us the
   * committed offset. A lost append acknowledgement never implies lost bytes. */
  async upload(request: UploadRequest, source: ByteSource, options: CallOptions = {}): Promise<AssetInfo> {
    // Snapshot the small identity before awaiting; data is never serialized here.
    request = { requestId: request.requestId, descriptor: { ...request.descriptor } };
    const size = descriptor(request.descriptor);
    this.budget(size);
    if (source.byteLength !== size) throw new ClientError('INPUT_INVALID', 'Source length differs from upload declaration');
    let identity: string | undefined;
    try {
      let upload = (await this.client.result({ operation: 'beginUpload', request }, 'upload', options)).upload;
      identity = upload.id;
      const validate = (value: UploadInfo): bigint => {
        requireState(value.id === identity && value.requestId === request.requestId, 'Upload identity changed');
        sameDescriptor(value.descriptor, request.descriptor);
        const received = byteLength(value.receivedBytes);
        requireState(received <= size, 'Upload exceeds its declaration');
        positiveBound(value.chunkBytes, this.chunkBytes, 'host upload chunk');
        requireState(received === size || received % BigInt(value.chunkBytes) === 0n, 'Unaligned committed upload offset');
        if (value.error) throw new ClientError('HOST_FAILURE', value.error.message,
          { requestId: request.requestId, uploadId: identity }, value.error);
        return received;
      };
      let offset = validate(upload);
      while (upload.state === 'uploading' && offset < size) {
        checkAbort(options.signal);
        const length = chunkLength(size - offset, upload.chunkBytes);
        const bytes = await waiting(source.readRange(offset, length), options.signal);
        requireState(bytes instanceof Uint8Array && bytes.byteLength === length, 'Source returned an incomplete chunk');
        checkAbort(options.signal);
        // Own this one chunk until append settles; do not retain source buffers.
        const next = await waiting(this.client.port.appendUpload(identity, offset.toString(), bytes.slice()), options.signal);
        const received = validate(next);
        requireState(received >= offset + BigInt(length), 'Append did not acknowledge the complete chunk');
        offset = received; upload = next;
      }
      if (upload.state === 'uploading') {
        upload = (await this.client.result({ operation: 'sealUpload', uploadId: identity }, 'upload', options)).upload;
        validate(upload);
      }
      if (upload.state === 'verifying') throw new ClientError('UPLOAD_BUSY', 'Host is verifying upload; retry with the same requestId',
        { requestId: request.requestId, uploadId: identity });
      requireState(upload.state === 'sealed' && upload.asset, 'Upload did not produce a sealed asset');
      requireState(byteLength(upload.receivedBytes) === size, 'Sealed upload length differs');
      asset(upload.asset); sameDescriptor(upload.asset.descriptor, request.descriptor);
      checkAbort(options.signal);
      return upload.asset;
    } catch (error) {
      if (error instanceof ClientError && error.code === 'WAIT_ABORTED')
        throw new ClientError(error.code, error.message,
          identity === undefined ? { requestId: request.requestId } : { requestId: request.requestId, uploadId: identity });
      throw error;
    }
  }

  /** Download to a private host sink, seal, then hash the FINAL stored bytes.
   * Return only a verified candidate reference. The host alone may commit it. */
  async copyAsset<Reference>(assetId: string, store: CandidateStore<Reference>,
    digests: DigestFactory, options: CopyOptions = {}): Promise<CopiedAsset<Reference>> {
    const expected = options.expectedDescriptor === undefined ? undefined : { ...options.expectedDescriptor };
    const received = (await this.client.result({ operation: 'readAsset', assetId }, 'asset', options)).asset;
    const info = { id: received.id, verification: received.verification, descriptor: { ...received.descriptor } };
    const size = asset(info, assetId);
    if (expected) sameDescriptor(info.descriptor, expected);
    this.budget(size);
    checkAbort(options.signal);
    // Private writes/seal are awaited to completion before discard, even after
    // local abort. Dropping their waiter could race cleanup against a late write.
    const sink = await store.begin(info);
    try {
      for (let offset = 0n; offset < size;) {
        checkAbort(options.signal);
        const length = chunkLength(size - offset, this.chunkBytes);
        const bytes = await waiting(this.client.port.readAssetRange(assetId, offset.toString(), length), options.signal);
        requireState(bytes instanceof Uint8Array && bytes.byteLength === length, 'Host returned an incomplete asset range');
        checkAbort(options.signal);
        await sink.write(bytes);
        offset += BigInt(length);
      }
      checkAbort(options.signal);
      const sealed = await sink.seal();
      await verifySealed(sealed, info, size, this.chunkBytes, digests, options.signal);
      checkAbort(options.signal);
      return { asset: info, reference: sealed.reference };
    } catch (error) {
      try { await sink.discard(); }
      catch (cleanup) { throw new AggregateError([error, cleanup], 'Candidate transfer failed and cleanup failed'); }
      throw error;
    }
  }
}
