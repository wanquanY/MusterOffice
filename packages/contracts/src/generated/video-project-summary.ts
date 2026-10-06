/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * Signed int64 ticks. Range requires semantic validation.
 *
 * This interface was referenced by `ProjectSummary`'s JSON-Schema
 * via the `definition` "Ticks".
 */
export type Ticks = string;
/**
 * This interface was referenced by `ProjectSummary`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * This interface was referenced by `ProjectSummary`'s JSON-Schema
 * via the `definition` "DocumentId".
 */
export type DocumentId = string;
/**
 * This interface was referenced by `ProjectSummary`'s JSON-Schema
 * via the `definition` "ObjectId".
 */
export type ObjectId = string;

export interface ProjectSummary {
  audioSampleCount: Ticks;
  audioSampleRate: number;
  canvas: Canvas;
  clipCount: number;
  documentDigest: Digest;
  documentId: DocumentId;
  duration: ExactRatio;
  frameCount: Ticks;
  frameRate: FrameRate;
  linkGroupCount: number;
  resourceCount: number;
  rootSequence: ObjectId;
  sequenceCount: number;
  title: string;
  trackCount: number;
}
/**
 * This interface was referenced by `ProjectSummary`'s JSON-Schema
 * via the `definition` "Canvas".
 */
export interface Canvas {
  height: number;
  width: number;
}
/**
 * Exact computed fraction. Decimal strings are bounded to 4096 bits and canonicalized by the kernel; each computation can impose a smaller budget.
 *
 * This interface was referenced by `ProjectSummary`'s JSON-Schema
 * via the `definition` "ExactRatio".
 */
export interface ExactRatio {
  denominator: string;
  numerator: string;
}
/**
 * This interface was referenced by `ProjectSummary`'s JSON-Schema
 * via the `definition` "FrameRate".
 */
export interface FrameRate {
  denominator: number;
  numerator: number;
}
