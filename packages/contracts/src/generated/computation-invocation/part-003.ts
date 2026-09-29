/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */
import type { ByteLength, Digest, Stroke, TypefaceMappingPolicy } from './part-001.js';
import type { Document } from './part-002.js';

/**
 * Explicit resource bundle bindings, not system font names or legal permissions.
 *
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "CascadeFont".
 */
export interface CascadeFont {
  byteLength: ByteLength;
  expectedSha256: Digest;
  faceIndex: number;
  offset: ByteLength;
}

/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "ManifestTypeface".
 */
export interface ManifestTypeface {
  bold?: ManifestInstance | null;
  boldItalic?: ManifestInstance | null;
  italic?: ManifestInstance | null;
  policy: TypefaceMappingPolicy;
  /**
   * Explicit instance selection; no synthesized bold/slant or slot fallback.
   */
  regular?: ManifestInstance | null;
  typeface: string;
}

/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "ManifestInstance".
 */
export interface ManifestInstance {
  face: number;
  /**
   * All axes are validated, including instances unused by this paragraph.
   */
  variations: ShapeVariation[];
}

/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "ShapeVariation".
 */
export interface ShapeVariation {
  tag: string;
  /**
   * Exact requested OpenType 16.16 design coordinate.
   */
  value1616: number;
}

/**
 * Expected identity, never an executable path or permission grant.
 */
export interface RendererIdentity {
  implementationSha256: Digest;
  profile: string;
}

/**
 * Storage envelope owned by an authorized host. A digest is not an access token.
 *
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "SnapshotRecord".
 */
export interface SnapshotRecord {
  document: Document;
  revision: Digest;
  semanticDigest: Digest;
}

/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "RendererIdentity".
 */
export interface RendererIdentity1 {
  implementationSha256: Digest;
  profile: string;
}

/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "TableCellBorders".
 */
export interface TableCellBorders1 {
  bottom?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: Stroke;
      };
  bottomLeftToTopRight?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: Stroke;
      };
  left?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: Stroke;
      };
  right?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: Stroke;
      };
  top?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: Stroke;
      };
  topLeftToBottomRight?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: Stroke;
      };
}
