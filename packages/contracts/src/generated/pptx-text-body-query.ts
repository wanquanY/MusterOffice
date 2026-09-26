/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `SourceTextBodyQuery`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * This interface was referenced by `SourceTextBodyQuery`'s JSON-Schema
 * via the `definition` "TextBodyProfile".
 */
export type TextBodyProfile = "drawingml-body-inheritance-draft-v1";

export interface SourceTextBodyQuery {
  expectedSourceSha256: Digest;
  objects: number[];
  profile: TextBodyProfile;
  surface: string;
}
