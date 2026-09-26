/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `FontRequest`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;

export interface FontRequest {
  characters: FontCharacter[];
  expectedSha256: Digest;
  faceIndex: number;
}
/**
 * This interface was referenced by `FontRequest`'s JSON-Schema
 * via the `definition` "FontCharacter".
 */
export interface FontCharacter {
  codepoint: number;
  variationSelector?: number | null;
}
