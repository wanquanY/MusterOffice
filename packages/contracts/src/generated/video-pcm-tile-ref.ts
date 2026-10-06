/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * Canonical uint64 byte length. Range requires semantic validation.
 *
 * This interface was referenced by `PcmTileRef`'s JSON-Schema
 * via the `definition` "ByteLength".
 */
export type ByteLength = string;
/**
 * Signed int64 ticks. Range requires semantic validation.
 *
 * This interface was referenced by `PcmTileRef`'s JSON-Schema
 * via the `definition` "Ticks".
 */
export type Ticks = string;
/**
 * This interface was referenced by `PcmTileRef`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;

export interface PcmTileRef {
  byteLength: ByteLength;
  firstSample: Ticks;
  frames: number;
  index: number;
  sha256: Digest;
  source: Digest;
}
