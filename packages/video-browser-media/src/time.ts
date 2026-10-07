import { requireMedia, type ExactRatio, type RationalTime, type SampleInterval, type TimeSample } from './contract.js';

function integer(value: string, signed: boolean): bigint {
  requireMedia(typeof value === 'string' && value.length <= 1235
    && (signed ? /^(0|-[1-9][0-9]*|[1-9][0-9]*)$/ : /^[1-9][0-9]*$/).test(value), 'invalid_request');
  const result = BigInt(value);
  requireMedia(result.toString(2).replace('-', '').length <= 4096, 'resource_limit');
  return result;
}
export function fraction(value: ExactRatio): { n: bigint; d: bigint } {
  return { n: integer(value.numerator, true), d: integer(value.denominator, false) };
}
export function compare(a: ExactRatio, b: ExactRatio): number {
  const x = fraction(a), y = fraction(b), result = x.n * y.d - y.n * x.d;
  return result < 0n ? -1 : result > 0n ? 1 : 0;
}
export function ratio(value: RationalTime): ExactRatio {
  return { numerator: value.ticks, denominator: String(value.timescale) };
}
export function validateScale(value: number): void {
  requireMedia(Number.isSafeInteger(value) && value > 0 && value <= 1_000_000_000, 'invalid_timing');
}
/** Recover the container's integer ticks after its seconds-based API boundary.
 * Reject values which cannot be represented safely; never accumulate rounded us. */
export function fromSeconds(seconds: number, timescale: number): RationalTime {
  validateScale(timescale);
  const scaled = seconds * timescale, ticks = Math.round(scaled);
  requireMedia(Number.isFinite(seconds) && Number.isSafeInteger(ticks)
    && Math.abs(scaled - ticks) < 0.01, 'invalid_timing');
  return { ticks: String(ticks === 0 ? 0 : ticks), timescale };
}
export function interval(timestamp: number, duration: number, timescale: number): SampleInterval {
  requireMedia(duration > 0, 'invalid_timing');
  const start = fromSeconds(timestamp, timescale), span = fromSeconds(duration, timescale);
  requireMedia(BigInt(span.ticks) > 0n, 'invalid_timing');
  return { start: ratio(start), end: { numerator: String(BigInt(start.ticks) + BigInt(span.ticks)), denominator: String(timescale) } };
}
/** Exact interval lookup on the container grid. 'before' uses a preceding tick
 * only when the requested instant lands exactly on that grid. No epsilon. */
export function lookupSeconds(sample: TimeSample, timescale: number): number {
  validateScale(timescale);
  requireMedia(sample.side === 'before' || sample.side === 'after', 'invalid_request');
  const { n, d } = fraction(sample.time), scaled = n * BigInt(timescale);
  let ticks = scaled / d;
  if (scaled % d < 0n) ticks--;
  if (sample.side === 'before' && scaled % d === 0n) ticks--;
  requireMedia(ticks >= -BigInt(Number.MAX_SAFE_INTEGER) && ticks <= BigInt(Number.MAX_SAFE_INTEGER), 'resource_limit');
  return Number(ticks) / timescale;
}
export function contains(range: SampleInterval, sample: TimeSample): boolean {
  const start = compare(sample.time, range.start), end = compare(sample.time, range.end);
  return sample.side === 'before' ? start > 0 && end <= 0 : start >= 0 && end < 0;
}
