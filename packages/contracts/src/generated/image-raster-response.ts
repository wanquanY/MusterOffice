/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type ImageRasterResponse =
  | {
      info: ImageRasterInfo;
      status: "rendered";
    }
  | {
      error: RasterFailure;
      status: "error";
    };
/**
 * Canonical uint64 byte length. Range requires semantic validation.
 */
export type ByteLength = string;
export type Digest = string;
/**
 * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
 */
export type FixedQ32 = string;
export type RasterFailureCode =
  | "INPUT_INVALID"
  | "LIMIT_EXCEEDED"
  | "COORDINATE_RANGE"
  | "PRECISION_EXCEEDED"
  | "CANCELLED"
  | "COMPONENT_FAILURE"
  | "COMPONENT_INVALID"
  | "HOST_FAILURE";

export interface ImageRasterInfo {
  images: ImageWork;
  raster: RasterInfo;
  resourcesSha256: Digest;
}
export interface ImageWork {
  brushes: number;
  /**
   * Bound for upstream uncertainty and affine/domain quantization, in Q32
   * device pixels. Domain repetition is bounded over the viewport. Excludes
   * inverse/shader arithmetic, filter output changes and sample coverage.
   */
  coordinateErrorBound: string;
  draws: number;
  resourceBytes: number;
  resources: number;
}
export interface RasterInfo {
  byteLength: ByteLength;
  /**
   * SHA-256 of the little-endian device batch; use together with profile.
   */
  frameSha256: string;
  height: number;
  profile: string;
  sha256: Digest;
  width: number;
  work: RasterWork;
}
export interface RasterWork {
  clips?: ClipWork | null;
  commands: number;
  compositing?: CompositeWork | null;
  /**
   * Conservative maximum error of the transformed control coordinates,
   * measured in raw Q32 pixels. Not a raster coverage error bound.
   */
  coordinateErrorBound: string;
  drawnCommands: number;
  draws: number;
  /**
   * Only present for V12 ellipse fields. Geometry and solver errors are separate.
   */
  ellipticGradients?: EllipticGradientWork | null;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  gradientCoordinateErrorBound: string;
  gradientDraws: number;
  gradientStops: number;
  /**
   * Maximum stop/color conversion or linear/rectangular field error.
   * Elliptic parameter and root bounds are reported separately.
   */
  gradientValueErrorBound: number;
  gradients: number;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  miterLimitErrorBound: string;
  opacityGroups?: OpacityGroupWork | null;
  paths: number;
  strokeDraws: number;
  strokeStyles: number;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  strokeWidthErrorBound: string;
}
export interface ClipWork {
  /**
   * Actual pushes when preserving common ancestors between consecutive draws.
   */
  applications: number;
  appliedCommands: number;
  maximumDepth: number;
  nodes: number;
  /**
   * All supplied clip path commands checked during device placement.
   */
  placementCommands: number;
}
export interface CompositeWork {
  /**
   * Exact packed pixel bytes copied for all snapshots; not process RSS.
   */
  capturedBytes: number;
  captures: number;
  snapshotDraws: number;
  sourceDraws: number;
}
/**
 * Maximum across elliptic fields; excludes pixel coverage and shader inverse arithmetic.
 */
export interface EllipticGradientWork {
  /**
   * Conservative device Q32 displacement of corresponding ellipse-family
   * points from parameter/plane conversion, including the tile phase budget.
   */
  coordinateErrorBound: string;
  /**
   * Q32 width of a successful solver enclosure for the encoded binary32
   * field. Not a bound on the source field's scalar or on pixel color.
   */
  encodedRootIntervalBound: string;
  /**
   * Dimensionless Q32 source uncertainty plus binary32 conversion error,
   * ordered scaleX/Y, centerX/Y, radiusX/Y.
   *
   * @minItems 6
   * @maxItems 6
   */
  parameterErrorBounds: [FixedQ32, FixedQ32, FixedQ32, FixedQ32, FixedQ32, FixedQ32];
}
export interface OpacityGroupWork {
  groups: number;
  maximumDepth: number;
  /**
   * Peak simultaneously live intermediate pixels, excluding output/snapshots.
   */
  peakPixelBytes: number;
  /**
   * Pixels cleared plus pixels composited, including fully transparent groups.
   */
  pixelWork: number;
}
export interface RasterFailure {
  code: RasterFailureCode;
  message: string;
}
