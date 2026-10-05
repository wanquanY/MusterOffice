/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * Signed int64 ticks. Range requires semantic validation.
 *
 * This interface was referenced by `PresentationWindow`'s JSON-Schema
 * via the `definition` "Ticks".
 */
export type Ticks = string;
/**
 * This interface was referenced by `PresentationWindow`'s JSON-Schema
 * via the `definition` "Timescale".
 */
export type Timescale = number;
/**
 * This interface was referenced by `PresentationWindow`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * This interface was referenced by `PresentationWindow`'s JSON-Schema
 * via the `definition` "ObjectId".
 */
export type ObjectId = string;

/**
 * Host-supplied bounded presentation-order metadata, independent of packet decode order.
 * A window does not claim that the whole source was indexed; outside-window requires another read.
 */
export interface PresentationWindow {
  coverage: TimeRange;
  resourceDigest: Digest;
  samples: PresentationSample[];
  stream: ObjectId;
}
/**
 * Half-open [start,end). Source ranges may begin before zero; timeline placement may not.
 *
 * This interface was referenced by `PresentationWindow`'s JSON-Schema
 * via the `definition` "TimeRange".
 */
export interface TimeRange {
  end: RationalTime;
  start: RationalTime;
}
/**
 * Exact wire representation. Equality compares author values; compare_time compares instants.
 *
 * This interface was referenced by `PresentationWindow`'s JSON-Schema
 * via the `definition` "RationalTime".
 */
export interface RationalTime {
  ticks: Ticks;
  timescale: Timescale;
}
/**
 * This interface was referenced by `PresentationWindow`'s JSON-Schema
 * via the `definition` "PresentationSample".
 */
export interface PresentationSample {
  /**
   * Opaque identity assigned by the host in this window, not an inferred CFR frame number.
   */
  id: number;
  interval: TimeRange;
}
