import type { AssetInfo } from '../../contracts/src/generated/host-response.js';
import type { DigestFactory, SealedCandidate } from './ports.js';
import { checkAbort, ClientError, requireState } from './errors.js';
import { chunkLength, digest } from './scalars.js';

/** Verify actual private storage and release its reader on every path. Private
 * reads settle before release/discard so late I/O cannot race cleanup. */
export async function verifySealed<Reference>(sealed: SealedCandidate<Reference>, info: AssetInfo,
  size: bigint, chunkBytes: number, digests: DigestFactory, signal?: AbortSignal): Promise<void> {
  let failed = false, failure: unknown;
  try {
    checkAbort(signal);
    requireState(sealed.source.byteLength === size, 'Sealed candidate length differs');
    const hash = digests.sha256();
    for (let offset = 0n; offset < size;) {
      checkAbort(signal);
      const length = chunkLength(size - offset, chunkBytes);
      const bytes = await sealed.source.readRange(offset, length);
      requireState(bytes instanceof Uint8Array && bytes.byteLength === length, 'Sealed candidate range is incomplete');
      checkAbort(signal);
      hash.update(bytes);
      offset += BigInt(length);
    }
    const actual = hash.finish(); digest(actual);
    if (actual !== info.descriptor.sha256) throw new ClientError('INTEGRITY', 'Final stored asset SHA256 differs');
    checkAbort(signal);
  } catch (error) { failed = true; failure = error; }
  try { await sealed.release(); }
  catch (release) {
    if (failed) throw new AggregateError([failure, release], 'Candidate verification and reader release failed');
    throw release;
  }
  if (failed) throw failure;
}
