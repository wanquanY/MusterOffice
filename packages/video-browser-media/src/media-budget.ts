import { requireMedia } from './contract.js';

/** Shared application admission for audio/video operations and owned leases.
 * Weights include explicit buffers and conservative codec reservations. They
 * are not a measurement or hard bound on browser/driver/linear-memory usage. */
export class BrowserMediaBudget {
  private bytes = 0;
  private peak = 0;
  constructor(readonly capacityBytes: number = 512 * 1024 ** 2) {
    requireMedia(Number.isSafeInteger(capacityBytes) && capacityBytes > 0, 'invalid_request');
  }
  get usedBytes(): number { return this.bytes; }
  get peakBytes(): number { return this.peak; }
  reserve(bytes: number): () => void {
    requireMedia(Number.isSafeInteger(bytes) && bytes > 0 && bytes <= this.capacityBytes - this.bytes, 'resource_limit');
    this.bytes += bytes; this.peak = Math.max(this.peak, this.bytes);
    let released = false;
    return () => { if (!released) { released = true; this.bytes -= bytes; } };
  }
}
