/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `FrameRequest`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * Signed int64 ticks. Range requires semantic validation.
 *
 * This interface was referenced by `FrameRequest`'s JSON-Schema
 * via the `definition` "Ticks".
 */
export type Ticks = string;

export interface FrameRequest {
  expectedDigest: Digest;
  frame: Ticks;
}
