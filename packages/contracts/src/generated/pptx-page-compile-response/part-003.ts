/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */
import type { FixedQ32, TableCellEdge } from './part-001.js';
import type { FillRule, PathCommand, PlacementCause, Point, Point1, PptxPageFailureCode, SourceCellAddress, SourceObjectRef, SourcePageIssue, SourcePageLocation, SourceVisualIssueKind, StrokeCap, StrokeJoin } from './part-002.js';

/**
 * World Q32 EMU position of source pixel boundary (0, 0).
 */
export interface Point7 {
  x: FixedQ32;
  y: FixedQ32;
}

export interface ImageSourceDomain {
  bottom: FixedQ32;
  left: FixedQ32;
  right: FixedQ32;
  top: FixedQ32;
}

export interface ImageBrushUncertainty {
  origin: Point8;
  /**
   * Nonnegative Q32 source pixel errors, left/top/right/bottom. Must be zero
   * when the brush has no explicit source domain.
   *
   * @minItems 4
   * @maxItems 4
   */
  sourceDomain: [FixedQ32, FixedQ32, FixedQ32, FixedQ32];
  xStep: Point9;
  yStep: Point1;
}

/**
 * Nonnegative Q32 world EMU errors; rebase never changes these bounds.
 */
export interface Point8 {
  x: FixedQ32;
  y: FixedQ32;
}

/**
 * Nonnegative Q32 world EMU per source pixel.
 */
export interface Point9 {
  x: FixedQ32;
  y: FixedQ32;
}

/**
 * World Q32 EMU displacement per source pixel, not endpoint coordinates.
 */
export interface Point10 {
  x: FixedQ32;
  y: FixedQ32;
}

export interface StrokeStyle {
  cap: StrokeCap;
  join: StrokeJoin;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  width: string;
}

/**
 * Render the complete interval onto transparent pixels, then source-over it
 * onto its parent with a single opacity. This is not per-paint alpha.
 */
export interface OpacityGroup {
  /**
   * Exclusive, and strictly greater than first_draw.
   */
  endDraw: number;
  firstDraw: number;
  /**
   * 0 is transparent; 65535 is opaque. Linear coverage of premultiplied sRGB.
   */
  opacity: number;
}

export interface FillPath {
  /**
   * Local Q32 EMU; open contours are implicitly closed for filling.
   */
  commands: PathCommand[];
  fillRule: FillRule;
}

export interface TransformNode {
  affine: Affine1;
  /**
   * Optional outer transform. Parents must precede children.
   */
  parent?: number | null;
}

/**
 * Evaluated affine transform, not author rotation/flip/viewport semantics.
 */
export interface Affine1 {
  /**
   * Dimensionless Q32 in row-major order: xx, xy, yx, yy.
   *
   * @minItems 4
   * @maxItems 4
   */
  linear: [FixedQ32, FixedQ32, FixedQ32, FixedQ32];
  translation: Point;
}

export interface RasterViewport {
  /**
   * Straight sRGB RGBA8; output is premultiplied.
   *
   * @minItems 4
   * @maxItems 4
   */
  background: [number, number, number, number];
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  coordinateTolerance: string;
  height: number;
  origin: Point11;
  scale: PixelScale;
  width: number;
}

/**
 * Q32 EMU. Subtracted before converting to device-space float32.
 */
export interface Point11 {
  x: FixedQ32;
  y: FixedQ32;
}

export interface PixelScale {
  denominator: number;
  /**
   * Positive rational pixels per EMU; normalized internally.
   */
  numerator: number;
}

export interface PptxPageFailure {
  code: PptxPageFailureCode;
  issue?: SourcePageIssue | null;
  location?: SourcePageLocation | null;
  message: string;
}

export interface SourceVisualIssue {
  kind: SourceVisualIssueKind;
  localName: string;
  namespace: string;
  sourceOrdinal: number;
}

export interface PlacementUnresolved {
  cause: PlacementCause;
  object: SourceObjectRef;
}

export interface TableBorderTarget {
  cell: SourceCellAddress;
  edge: TableCellEdge;
  nativeId: number;
}
