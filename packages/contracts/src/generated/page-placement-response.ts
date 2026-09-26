/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type PagePlacementResponse =
  | {
      result: PagePlacements;
      status: "evaluated";
    }
  | {
      error: PlacementFailure;
      status: "error";
    };
export type Digest = string;
/**
 * Signed int64 EMU. 1 point = 12700 EMU. Range requires semantic validation.
 */
export type Emu = string;
export type SlideId = string;
export type ContainerId =
  | {
      id: SlideId;
      kind: "slide";
    }
  | {
      id: MasterId;
      kind: "master";
    }
  | {
      id: LayoutId;
      kind: "layout";
    }
  | {
      id: ObjectId;
      kind: "group";
    };
export type MasterId = string;
export type LayoutId = string;
export type ObjectId = string;
/**
 * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
 */
export type FixedQ32 = string;
export type PlacementFailureCode = "INPUT_INVALID" | "LIMIT_EXCEEDED" | "COORDINATE_RANGE" | "CANCELLED";
export type ValidationCode =
  | "IDENTITY_MISMATCH"
  | "MISSING_REFERENCE"
  | "DUPLICATE_IDENTITY"
  | "INVALID_VALUE"
  | "OWNERSHIP_CONFLICT"
  | "REFERENCE_CYCLE"
  | "LIMIT_EXCEEDED";

export interface PagePlacements {
  documentSha256: Digest;
  hidden: boolean;
  pageSize: Size;
  profile: string;
  slide: SlideId;
  /**
   * Master, layout, slide contexts; not a resolved paint or placeholder list.
   */
  surfaces: PlacementSurface[];
  uniqueAngles: number;
}
export interface Size {
  height: Emu;
  width: Emu;
}
export interface PlacementSurface {
  container: ContainerId;
  /**
   * Stable author order, groups followed by their descendants. A group entry
   * is a coordinate-space record, not a paint instruction.
   */
  objects: ObjectPlacement[];
}
export interface ObjectPlacement {
  affine: Affine;
  anchor: Point1;
  depth: number;
  object: ObjectId;
  parent: ContainerId;
  sourceSize: Size1;
  uncertainty: AffineUncertainty;
}
/**
 * Already evaluated into page space; do not apply parent placements again.
 */
export interface Affine {
  /**
   * Dimensionless Q32 in row-major order: xx, xy, yx, yy.
   *
   * @minItems 4
   * @maxItems 4
   */
  linear: [FixedQ32, FixedQ32, FixedQ32, FixedQ32];
  translation: Point;
}
/**
 * Q32 in the same coordinate unit as the input point.
 */
export interface Point {
  x: FixedQ32;
  y: FixedQ32;
}
/**
 * Subtract this exact source center before applying the evaluated affine.
 */
export interface Point1 {
  x: FixedQ32;
  y: FixedQ32;
}
/**
 * Shape/picture/connector bounds are local 0..size; custom paths and groups
 * use their own viewport. The current author model has no child offset.
 */
export interface Size1 {
  height: Emu;
  width: Emu;
}
/**
 * Covers author scale/angle computation and final Q32 quantization. It is
 * additional to mo-render's bounds for an already-quantized affine graph.
 */
export interface AffineUncertainty {
  /**
   * Nonnegative outward bounds, raw dimensionless Q32.
   *
   * @minItems 4
   * @maxItems 4
   */
  linear: [FixedQ32, FixedQ32, FixedQ32, FixedQ32];
  translation: Point2;
}
/**
 * Nonnegative outward bounds, raw Q32 EMU.
 */
export interface Point2 {
  x: FixedQ32;
  y: FixedQ32;
}
export interface PlacementFailure {
  code: PlacementFailureCode;
  message: string;
  report?: ValidationReport | null;
}
export interface ValidationReport {
  issues: ValidationIssue[];
  truncated: boolean;
}
export interface ValidationIssue {
  code: ValidationCode;
  message: string;
  path: string;
}
