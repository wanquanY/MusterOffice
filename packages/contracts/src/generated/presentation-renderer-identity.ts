/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `RendererIdentity`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;

export interface RendererIdentity {
  implementationSha256: Digest;
  profile: string;
}
