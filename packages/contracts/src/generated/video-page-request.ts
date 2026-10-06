/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `PageRequest`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * This interface was referenced by `PageRequest`'s JSON-Schema
 * via the `definition` "ListQuery".
 */
export type ListQuery =
  | {
      kind: "sequences";
    }
  | {
      kind: "tracks";
      sequence: ObjectId;
    }
  | {
      kind: "clips";
      range?: TimeRange | null;
      track: ObjectId;
    }
  | {
      kind: "resources";
    }
  | {
      clip?: ObjectId | null;
      kind: "linkGroups";
    };
/**
 * This interface was referenced by `PageRequest`'s JSON-Schema
 * via the `definition` "ObjectId".
 */
export type ObjectId = string;
/**
 * Signed int64 ticks. Range requires semantic validation.
 *
 * This interface was referenced by `PageRequest`'s JSON-Schema
 * via the `definition` "Ticks".
 */
export type Ticks = string;
/**
 * This interface was referenced by `PageRequest`'s JSON-Schema
 * via the `definition` "Timescale".
 */
export type Timescale = number;

export interface PageRequest {
  cursor?: PageCursor | null;
  limit: number;
  query: ListQuery;
}
/**
 * This interface was referenced by `PageRequest`'s JSON-Schema
 * via the `definition` "PageCursor".
 */
export interface PageCursor {
  documentDigest: Digest;
  offset: number;
  queryDigest: Digest;
}
/**
 * Half-open [start,end). Source ranges may begin before zero; timeline placement may not.
 *
 * This interface was referenced by `PageRequest`'s JSON-Schema
 * via the `definition` "TimeRange".
 */
export interface TimeRange {
  end: RationalTime;
  start: RationalTime;
}
/**
 * Exact wire representation. Equality compares author values; compare_time compares instants.
 *
 * This interface was referenced by `PageRequest`'s JSON-Schema
 * via the `definition` "RationalTime".
 */
export interface RationalTime {
  ticks: Ticks;
  timescale: Timescale;
}
