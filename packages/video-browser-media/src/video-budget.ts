import { requireMedia } from './contract.js';

/** Admission budget shared by a host's source decoders and owned frame leases.
 * Charges include conservative surface weights and codec reference reservations;
 * they are not a measurement of opaque browser/driver allocations. */
export class BrowserVideoBudget {
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
export function videoSurfaceBytes(width: number, height: number, bytesPerPixel = 8): number {
  requireMedia(Number.isSafeInteger(width) && width > 0 && Number.isSafeInteger(height) && height > 0, 'unsupported_media');
  requireMedia(bytesPerPixel === 4 || bytesPerPixel === 8, 'invalid_request');
  const bytes = Math.ceil(width / 64) * 64 * Math.ceil(height / 64) * 64 * bytesPerPixel;
  requireMedia(Number.isSafeInteger(bytes), 'resource_limit');
  return bytes;
}
