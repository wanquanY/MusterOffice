/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `SourceTextEdits`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;

export interface SourceTextEdits {
  edits: SourceTextEdit[];
  expectedSourceSha256: Digest;
}
/**
 * This interface was referenced by `SourceTextEdits`'s JSON-Schema
 * via the `definition` "SourceTextEdit".
 */
export interface SourceTextEdit {
  expectedText: string;
  replacement: string;
  target: SourceTextTarget;
}
/**
 * This interface was referenced by `SourceTextEdits`'s JSON-Schema
 * via the `definition` "SourceTextTarget".
 */
export interface SourceTextTarget {
  objectId: number;
  paragraph: number;
  part: string;
  run: number;
}
