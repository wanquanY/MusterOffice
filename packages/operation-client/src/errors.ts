import type { Failure } from '../../contracts/src/generated/host-response.js';

export type ClientErrorCode = 'WAIT_ABORTED' | 'WAIT_TIMEOUT' | 'HOST_FAILURE' |
  'HOST_PROTOCOL' | 'INPUT_INVALID' | 'INTEGRITY' | 'LIMIT_EXCEEDED' | 'UPLOAD_BUSY';
export class ClientError extends Error {
  constructor(readonly code: ClientErrorCode, message: string,
    readonly recovery: Readonly<{ jobId?: string; requestId?: string; uploadId?: string }> = {},
    readonly failure?: Failure) {
    super(message);
    this.name = 'MusterOfficeClientError';
  }
}
export function requireState(condition: unknown, message: string): asserts condition {
  if (!condition) throw new ClientError('HOST_PROTOCOL', message);
}
export function checkAbort(signal?: AbortSignal): void {
  if (signal?.aborted) throw new ClientError('WAIT_ABORTED', 'Local wait aborted; host work may still complete');
}

/** Observe late fulfillment/rejection after abort; never resend a mutation or
 * turn transport uncertainty into a fabricated durable cancellation. */
export function waiting<T>(work: Promise<T>, signal?: AbortSignal): Promise<T> {
  if (!signal) return work;
  return new Promise<T>((resolve, reject) => {
    const abort = () => { cleanup(); reject(new ClientError('WAIT_ABORTED', 'Local wait aborted; host work may still complete')); };
    const cleanup = () => signal.removeEventListener('abort', abort);
    work.then(value => { cleanup(); resolve(value); }, error => { cleanup(); reject(error); });
    signal.addEventListener('abort', abort, { once: true });
    if (signal.aborted) abort();
  });
}

export function delay(ms: number, signal?: AbortSignal): Promise<void> {
  checkAbort(signal);
  return new Promise<void>((resolve, reject) => {
    const cleanup = () => { clearTimeout(timer); signal?.removeEventListener('abort', abort); };
    const abort = () => { cleanup(); reject(new ClientError('WAIT_ABORTED', 'Local wait aborted; host work may still complete')); };
    const timer = setTimeout(() => { cleanup(); resolve(); }, ms);
    signal?.addEventListener('abort', abort, { once: true });
    if (signal?.aborted) abort();
  });
}
