import { BrowserMediaError, requireMedia, type BrowserMediaErrorCode } from './contract.js';
import type { BrowserPcmCompletion, BrowserPcmDecode } from './pcm-contract.js';

export function pcmCall<T>(run: () => T): T {
  try { return run(); } catch (error) {
    if (error instanceof BrowserMediaError) throw error;
    let code: unknown;
    try { const text = String(error); if (text.length <= 16384) code = JSON.parse(text).code; } catch { /* Invalid ABI errors fail closed. */ }
    const supported: readonly BrowserMediaErrorCode[] = ['invalid_request', 'resource_limit', 'cancelled', 'pending_source', 'unsupported_media', 'invalid_timing'];
    throw new BrowserMediaError(supported.includes(code as BrowserMediaErrorCode) ? code as BrowserMediaErrorCode : 'invalid_request');
  }
}
export function completion(core: BrowserPcmDecode, release: () => void,
  metadata: Omit<BrowserPcmCompletion, 'seal' | 'close'>): BrowserPcmCompletion {
  let closed = false;
  return Object.freeze({ ...metadata,
    seal(reference: string) {
      requireMedia(!closed, 'closed'); requireMedia(typeof reference === 'string' && reference.length <= 4096, 'resource_limit');
      return pcmCall(() => core.seal(reference));
    },
    close() { if (!closed) { closed = true; try { core.free(); } finally { release(); } } },
  });
}
