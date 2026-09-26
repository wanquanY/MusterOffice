import type { AssetDescriptor, AssetInfo } from '../../contracts/src/generated/host-response.js';
import { ClientError, requireState } from './errors.js';

export function byteLength(value: string): bigint {
  requireState(typeof value === 'string' && /^(0|[1-9][0-9]{0,19})$/.test(value), 'Noncanonical byte length');
  const result = BigInt(value);
  requireState(result <= 18446744073709551615n, 'Byte length exceeds uint64');
  return result;
}
export function digest(value: string): void {
  requireState(typeof value === 'string' && /^[0-9a-f]{64}$/.test(value), 'Invalid SHA256');
}
export function descriptor(value: AssetDescriptor): bigint {
  digest(value.sha256);
  requireState(typeof value.mediaType === 'string' && value.mediaType.length > 0, 'Missing media type');
  return byteLength(value.byteLength);
}
export function sameDescriptor(a: AssetDescriptor, b: AssetDescriptor): void {
  descriptor(a); descriptor(b);
  requireState(a.byteLength === b.byteLength && a.sha256 === b.sha256 && a.mediaType === b.mediaType,
    'Asset declaration changed');
}
export function asset(value: AssetInfo, expectedId?: string): bigint {
  requireState(value.verification === 'bytesSha256' && value.id.length > 0, 'Asset is not byte-verified');
  if (expectedId !== undefined) requireState(value.id === expectedId, 'Asset identity differs');
  return descriptor(value.descriptor);
}
export function chunkLength(remaining: bigint, limit: number): number {
  return Number(remaining < BigInt(limit) ? remaining : BigInt(limit));
}
export function positiveBound(value: number, maximum: number, name: string): void {
  if (!Number.isSafeInteger(value) || value < 1 || value > maximum)
    throw new ClientError('INPUT_INVALID', `Invalid ${name}`);
}
