/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * Signed int64 EMU. 1 point = 12700 EMU. Range requires semantic validation.
 *
 * This interface was referenced by `SourceTransformEdits`'s JSON-Schema
 * via the `definition` "Emu".
 */
export type Emu = string;
/**
 * This interface was referenced by `SourceTransformEdits`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;

export interface SourceTransformEdits {
  edits: SourceTransformEdit[];
  expectedSourceSha256: Digest;
}
/**
 * This interface was referenced by `SourceTransformEdits`'s JSON-Schema
 * via the `definition` "SourceTransformEdit".
 */
export interface SourceTransformEdit {
  expected: SourceTransformValues;
  replacement: SourceTransformValues;
  target: SourceObjectRef;
}
/**
 * Direct native declarations, never resolved placement or inherited values.
 * Angles use native 1/60000 degree units; signed/multi-turn values are retained.
 *
 * This interface was referenced by `SourceTransformEdits`'s JSON-Schema
 * via the `definition` "SourceTransformValues".
 */
export interface SourceTransformValues {
  childOrigin?: Point | null;
  childSize?: Size | null;
  flipHorizontal?: boolean | null;
  flipVertical?: boolean | null;
  origin?: Point | null;
  rotation?: number | null;
  size?: Size | null;
}
/**
 * This interface was referenced by `SourceTransformEdits`'s JSON-Schema
 * via the `definition` "Point".
 */
export interface Point {
  x: Emu;
  y: Emu;
}
/**
 * This interface was referenced by `SourceTransformEdits`'s JSON-Schema
 * via the `definition` "Size".
 */
export interface Size {
  height: Emu;
  width: Emu;
}
/**
 * Immutable source part and native object ID, including the shape-tree ID.
 */
export interface SourceObjectRef {
  nativeId: number;
  part: string;
}
/**
 * This interface was referenced by `SourceTransformEdits`'s JSON-Schema
 * via the `definition` "SourceObjectRef".
 */
export interface SourceObjectRef1 {
  nativeId: number;
  part: string;
}
