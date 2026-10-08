import { requireMedia } from './contract.js';

/** Existing consumers share the SAME ledger/constructor as new audio users. */
export { BrowserMediaBudget as BrowserVideoBudget } from './media-budget.js';
export function videoSurfaceBytes(width: number, height: number, bytesPerPixel = 8): number {
  requireMedia(Number.isSafeInteger(width) && width > 0 && Number.isSafeInteger(height) && height > 0, 'unsupported_media');
  requireMedia(bytesPerPixel === 4 || bytesPerPixel === 8, 'invalid_request');
  const bytes = Math.ceil(width / 64) * 64 * Math.ceil(height / 64) * 64 * bytesPerPixel;
  requireMedia(Number.isSafeInteger(bytes), 'resource_limit');
  return bytes;
}
