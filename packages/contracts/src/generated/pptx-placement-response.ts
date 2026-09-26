/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type PptxPlacementResponse =
  | {
      placements: SourcePlacements;
      status: "evaluated";
    }
  | {
      error: PptxFailure;
      status: "error";
    };
export type SourcePlacementOutcome =
  | {
      placement: NativePlacement;
      status: "resolved";
    }
  | {
      reason: PlacementUnresolved;
      status: "unresolved";
    };
/**
 * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
 */
export type FixedQ32 = string;
/**
 * Signed int64 EMU. 1 point = 12700 EMU. Range requires semantic validation.
 */
export type Emu = string;
export type TransformValueSource =
  | {
      kind: "declaration";
      object: SourceObjectRef;
    }
  | {
      kind: "default";
      object: SourceObjectRef;
    };
export type PlacementCause =
  | {
      kind: "missingOrigin";
    }
  | {
      kind: "missingSize";
    }
  | {
      kind: "inheritance";
      status: SourcePlaceholderMatch;
    }
  | {
      kind: "retainedTransform";
      sourceOrdinal: number;
    }
  | {
      field: string;
      kind: "invalidCoordinate";
    }
  | {
      kind: "numericRange";
    };
export type SourcePlaceholderMatch =
  | {
      status: "notPlaceholder";
    }
  | {
      status: "master";
    }
  | {
      rule: PlaceholderMatchRule;
      status: "matched";
      target: SourceObjectRef;
    }
  | {
      status: "unmatched";
    }
  | {
      status: "detached";
    }
  | {
      candidates: number;
      part: string;
      status: "ambiguous";
    }
  | {
      status: "unsupportedContext";
    };
export type PlaceholderMatchRule = "slideIndex" | "masterType";
export type SourcePlacementProfile = "drawingml-source-sector-scale-q96-v1-draft";
export type Digest = string;
export type PptxFailureCode =
  | "INPUT_INVALID"
  | "SOURCE_CONFLICT"
  | "PRESERVATION_CONFLICT"
  | "MAPPING_NOT_IMPLEMENTED"
  | "RESOURCE_REQUIRED"
  | "LIMIT_EXCEEDED"
  | "CANCELLED"
  | "READ_FAILED";

export interface SourcePlacements {
  objects: NativeObjectPlacement[];
  profile: SourcePlacementProfile;
  /**
   * Presentation spTree transform declarations are retained but do not move
   * page contents in this profile. Nested grpSp transforms do participate.
   */
  rootTransformIgnored: boolean;
  sourceSha256: Digest;
  surface: string;
  uniqueAngles: number;
}
export interface NativeObjectPlacement {
  depth: number;
  nativeId: number;
  outcome: SourcePlacementOutcome;
  parentGroup?: number | null;
}
export interface NativePlacement {
  affine: Affine;
  anchor: Point21;
  sourceOrigin: Point;
  sourceSize: Size;
  transform: ResolvedNativeTransform;
  uncertainty: AffineUncertainty;
}
/**
 * Already in surface coordinates; subtract anchor, then apply once.
 */
export interface Affine {
  /**
   * Dimensionless Q32 in row-major order: xx, xy, yx, yy.
   *
   * @minItems 4
   * @maxItems 4
   */
  linear: [FixedQ32, FixedQ32, FixedQ32, FixedQ32];
  translation: Point2;
}
/**
 * Q32 in the same coordinate unit as the input point.
 */
export interface Point2 {
  x: FixedQ32;
  y: FixedQ32;
}
export interface Point21 {
  x: FixedQ32;
  y: FixedQ32;
}
export interface Point {
  x: Emu;
  y: Emu;
}
/**
 * Effective coordinate viewport. A zero/missing group child extent uses
 * the corresponding target extent, producing unit scale on that axis.
 */
export interface Size {
  height: Emu;
  width: Emu;
}
export interface ResolvedNativeTransform {
  childOrigin?: TransformValue | null;
  childSize?: TransformValue2 | null;
  flipHorizontal: TransformValue4;
  flipVertical: TransformValue4;
  /**
   * Office ignores a graphic frame's own orientation attributes; parent
   * group orientation still applies. The original values remain above.
   */
  graphicFrameOrientationIgnored: boolean;
  origin: TransformValue;
  rotation: TransformValue3;
  size: TransformValue2;
}
export interface TransformValue {
  source: TransformValueSource;
  value: Point;
}
export interface SourceObjectRef {
  nativeId: number;
  part: string;
}
export interface TransformValue2 {
  source: TransformValueSource;
  value: Size1;
}
export interface Size1 {
  height: Emu;
  width: Emu;
}
export interface TransformValue4 {
  source: TransformValueSource;
  value: boolean;
}
export interface TransformValue3 {
  source: TransformValueSource;
  value: number;
}
export interface AffineUncertainty {
  /**
   * Nonnegative outward bounds, raw dimensionless Q32.
   *
   * @minItems 4
   * @maxItems 4
   */
  linear: [FixedQ32, FixedQ32, FixedQ32, FixedQ32];
  translation: Point22;
}
/**
 * Nonnegative outward bounds, raw Q32 EMU.
 */
export interface Point22 {
  x: FixedQ32;
  y: FixedQ32;
}
export interface PlacementUnresolved {
  cause: PlacementCause;
  object: SourceObjectRef;
}
export interface PptxFailure {
  code: PptxFailureCode;
  message: string;
}
