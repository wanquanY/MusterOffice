/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `CreateProject`'s JSON-Schema
 * via the `definition` "DocumentId".
 */
export type DocumentId = string;
/**
 * This interface was referenced by `CreateProject`'s JSON-Schema
 * via the `definition` "ObjectId".
 */
export type ObjectId = string;

export interface CreateProject {
  audioSampleRate: number;
  canvas: Canvas;
  documentId: DocumentId;
  frameRate: FrameRate;
  rootSequence: ObjectId;
  title: string;
}
/**
 * This interface was referenced by `CreateProject`'s JSON-Schema
 * via the `definition` "Canvas".
 */
export interface Canvas {
  height: number;
  width: number;
}
/**
 * This interface was referenced by `CreateProject`'s JSON-Schema
 * via the `definition` "FrameRate".
 */
export interface FrameRate {
  denominator: number;
  numerator: number;
}
