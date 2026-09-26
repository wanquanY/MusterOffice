/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `SourceFillQuery`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * An explicit interpretation, not a certificate for any Office/WPS version.
 *
 * This interface was referenced by `SourceFillQuery`'s JSON-Schema
 * via the `definition` "FillProfile".
 */
export type FillProfile = "ms-oi29500-fills-2024-draft-v1";
/**
 * This interface was referenced by `SourceFillQuery`'s JSON-Schema
 * via the `definition` "FillTarget".
 */
export type FillTarget =
  | {
      kind: "object";
      nativeId: number;
    }
  | {
      kind: "line";
      nativeId: number;
    }
  | {
      kind: "picture";
      nativeId: number;
    }
  | {
      kind: "rootGroup";
    }
  | {
      kind: "background";
    };

export interface SourceFillQuery {
  expectedSourceSha256: Digest;
  profile: FillProfile;
  surface: string;
  targets: FillTarget[];
}
