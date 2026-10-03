/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */
import type { ByteLength, Digest, EffectiveImageMode, Emu, FillOrigin, FillTarget, FixedQ32, FontMetric, FontSelectionReason, FontStyle, ImageFormat, NativeBlipCompression, NativeCoordinate, NativeFillAlignment, NativeFontCollectionIndex, NativeFontSlot, NativePercentage, NativeTableGridIssue, NativeTextElement, NativeTileFlip, NavigationDirection, NodePhase, PhysicalPixelSize, PlacementCause, PlaybackGeneration, PlaybackSessionId, PptxPageFailureCode, PresentationStepOutcome, ResolutionSource, ResolutionUnit, RotationBasis, ShapeFailureCode, SourceColor, SourceImageOutcome, SourcePageIssue, SourcePageProfile, SourcePlaceholderMatch, SourceVisualIssueKind, SurfaceKind, TableCellEdge, TableStyleSelectionError, TextStyleOrigin, Ticks, TimelineWorkCount, Timescale, TimingNodeId, Visibility } from './part-001.js';
import type { EffectiveVariation } from './part-003.js';

export type SourceTextIssue =
  | {
      kind: "cascade";
      reason: TextCascadeUnresolved;
    }
  | {
      kind: "field";
      paragraph: number;
      run: number;
      sourceOrdinal: number;
    }
  | {
      kind: "characterProperty";
      paragraph: number;
      property: CharacterProperty;
      run?: number | null;
    }
  | {
      kind: "directionOverride";
      paragraph: number;
      run?: number | null;
    }
  | {
      kind: "symbolFont";
      paragraph: number;
      run?: number | null;
    }
  | {
      kind: "paragraphControl";
      paragraph: number;
      run: number;
    }
  | {
      kind: "itemization";
      notice: ItemizationNotice;
      paragraph: number;
    }
  | {
      end: number;
      kind: "script";
      paragraph: number;
      script: string;
      start: number;
    }
  | {
      kind: "typeface";
      paragraph: number;
      reason: TypefaceUnresolved;
      run?: number | null;
      slot: NativeFontSlot;
    }
  | {
      boundary: number;
      kind: "graphemeStyleConflict";
      paragraph: number;
    };

export type TextCascadeUnresolved =
  | {
      kind: "chartText";
      origin: TextStyleOrigin;
      reason: ChartTextUnresolved;
    }
  | {
      kind: "tableGrid";
      reason: NativeTableGridIssue;
    }
  | {
      kind: "tableStyle";
      reason: TableStyleSelectionError;
    }
  | {
      kind: "noTextBody";
    }
  | {
      kind: "placeholder";
      matching: SourcePlaceholderMatch;
      object: SourceObjectRef;
    }
  | {
      kind: "retainedContent";
      origin: TextStyleOrigin;
    }
  | {
      kind: "ambiguousTemplateParagraph";
      level: number;
      object: SourceObjectRef;
    }
  | {
      kind: "fieldParagraph";
      origin: TextStyleOrigin;
    };

export type ChartTextUnresolved = "multiplePropertyParagraphs" | "propertyTextRuns" | "listStyle";

export type CharacterProperty =
  | "kumimoji"
  | "language"
  | "alternativeLanguage"
  | "size"
  | "bold"
  | "italic"
  | "underline"
  | "strike"
  | "kerning"
  | "caps"
  | "spacing"
  | "normalizeHeight"
  | "baseline"
  | "noProof"
  | "dirty"
  | "error"
  | "smartClean"
  | "smartId"
  | "bookmark";

export type ItemizationNoticeKind = "mixedScriptCluster" | "mixedLevelCluster" | "ambiguousScript";

export type TypefaceUnresolved =
  | {
      kind: "missingDeclaration";
    }
  | {
      kind: "emptyTypeface";
    }
  | {
      kind: "noThemeFont";
    }
  | {
      kind: "disabledThemeFont";
    }
  | {
      kind: "symbolThemeFont";
    }
  | {
      kind: "scriptRequired";
    }
  | {
      kind: "missingSupplemental";
      script: string;
    }
  | {
      kind: "ambiguousSupplemental";
      script: string;
    }
  | {
      kind: "unknownThemeToken";
      token: string;
    }
  | {
      kind: "retainedDeclaration";
      origin: TextStyleOrigin;
    }
  | {
      kind: "retainedTheme";
      scheme: SourceThemeSchemeRef;
      sourceOrdinal: number;
    };

export type TextBodyProperty =
  | "rotation"
  | "paragraphSpacing"
  | "verticalOverflow"
  | "horizontalOverflow"
  | "vertical"
  | "wrap"
  | "leftInset"
  | "topInset"
  | "rightInset"
  | "bottomInset"
  | "columns"
  | "columnSpacing"
  | "rightToLeftColumns"
  | "fromWordArt"
  | "anchor"
  | "centerAnchor"
  | "forceAntialiasing"
  | "upright"
  | "compatibleLineSpacing";

export type ParagraphProperty =
  | "leftMargin"
  | "rightMargin"
  | "level"
  | "indent"
  | "alignment"
  | "defaultTabSize"
  | "rightToLeft"
  | "eastAsianLineBreak"
  | "fontAlignment"
  | "latinLineBreak"
  | "hangingPunctuation";

export type FlowIssue =
  | {
      kind: "tab";
      scalar: number;
    }
  | {
      kind: "conditionalHyphen";
      scalar: number;
    }
  | {
      kind: "contingentObject";
      scalar: number;
    }
  | {
      end: number;
      kind: "mixedLevelCluster";
      start: number;
    }
  | {
      end: number;
      kind: "unresolvedFont";
      start: number;
    };

export type GeometryIssue =
  | {
      end: number;
      kind: "unresolvedFont";
      start: number;
    }
  | {
      instance: number;
      kind: "missingMetric";
      metric: FontMetric;
    }
  | {
      end: number;
      kind: "tab";
      start: number;
    }
  | {
      end: number;
      kind: "mixedLevelCluster";
      start: number;
    }
  | {
      kind: "nonPositiveNaturalHeight";
      line: number;
    };

export type PathSceneIssue =
  | {
      font: number;
      kind: "colorRepresentationRequired";
      variations: EffectiveVariation[];
    }
  | {
      font: number;
      glyph_id: number;
      kind: "outlineUnavailable";
      variations: EffectiveVariation[];
    };

export type ColorUnresolved =
  | {
      kind: "missingColorMap";
    }
  | {
      kind: "missingColorScheme";
    }
  | {
      kind: "missingThemeSlot";
      slot: ColorSlot;
    }
  | {
      color: SystemColor;
      kind: "missingSystemColor";
    }
  | {
      kind: "missingPlaceholder";
    }
  | {
      kind: "retainedPlaceholderContext";
      part: string;
      sourceOrdinal: number;
    }
  | {
      kind: "schemeCycle";
      slot: ColorSlot;
    }
  | {
      kind: "numericRange";
    };

export type ColorSlot =
  | "dk1"
  | "lt1"
  | "dk2"
  | "lt2"
  | "accent1"
  | "accent2"
  | "accent3"
  | "accent4"
  | "accent5"
  | "accent6"
  | "hlink"
  | "folHlink";

export type SystemColor =
  | "scrollBar"
  | "background"
  | "activeCaption"
  | "inactiveCaption"
  | "menu"
  | "window"
  | "windowFrame"
  | "menuText"
  | "windowText"
  | "captionText"
  | "activeBorder"
  | "inactiveBorder"
  | "appWorkspace"
  | "highlight"
  | "highlightText"
  | "btnFace"
  | "btnShadow"
  | "grayText"
  | "btnText"
  | "inactiveCaptionText"
  | "btnHighlight"
  | "3dDkShadow"
  | "3dLight"
  | "infoText"
  | "infoBk"
  | "hotLight"
  | "gradientActiveCaption"
  | "gradientInactiveCaption"
  | "menuHighlight"
  | "menuBar";

export type TimelineFailureCode =
  | "INPUT_INVALID"
  | "REVISION_CONFLICT"
  | "EVENT_HISTORY_REQUIRED"
  | "EVENT_HISTORY_INVALID"
  | "LIMIT_EXCEEDED"
  | "CANCELLED";

export interface PptxPlaybackSessionInfo {
  binding: PlaybackBinding;
  /**
   * In-process implementation/content identity, not host authorization.
   */
  planId: string;
  preparation: ResourcePreparationInfo;
  profile: string;
  slide: string;
  sourceSha256: Digest;
  viewport: RasterViewport;
  viewportRevision: number;
}

export interface PlaybackBinding {
  generation: PlaybackGeneration;
  revision: Digest;
  session: PlaybackSessionId;
}

export interface ResourcePreparationInfo {
  decodedImages: number;
  decodedPixelBytes: number;
  encodedBytes: number;
  gatherCopyBytes: number;
  resourcesSha256: Digest;
  textFrames: number;
  /**
   * Logical paths, draw elements and optional clip metadata; not heap capacity or RSS.
   */
  textPathBytes: number;
  textWork: FrameWork;
}

export interface FrameWork {
  componentCalls: number;
  fontUploadBytes: number;
  glyphs: number;
  pathCommands: number;
  requestWords: number;
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
   * Raw Q32 pixels; 256..=2^24 (at most 1/256 pixel).
   */
  coordinateTolerance: string;
  height: number;
  origin: Point;
  scale: PixelScale;
  width: number;
}

/**
 * Q32 EMU. Subtracted before converting to device-space float32.
 */
export interface Point {
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

/**
 * Read-only diagnostics. Counts successful timing evaluations, including those
 * followed by a page/raster failure; does not count published or displayed frames.
 */
export interface PlaybackTimingInfo {
  binding: PlaybackBinding;
  sampler: TimelineSamplerInfo;
}

export interface TimelineSamplerInfo {
  cachedBinding?: PlaybackBinding | null;
  retainedEvents: TimelineWorkCount;
  retainedIntervals: TimelineWorkCount;
  /**
   * Canonical uint64 timeline work count; never wraps.
   */
  schedulesBuilt: string;
  schedulesReused: TimelineWorkCount;
  timelineSha256: Digest;
}

export interface PptxPlaybackRasterInfo {
  page: SourceResourcePageRasterInfo;
  playback: SourcePlaybackFrame;
  profile: string;
}

export interface SourceResourcePageRasterInfo {
  decodedImages: DecodedImageInfo[];
  encodedBytes: number;
  gatherCopyBytes: number;
  images: ImageWork;
  page: SourcePageRasterInfo;
  profile: string;
  resourcesSha256: Digest;
  /**
   * Shape-local capacity from the painted frame; None is historical/unmeasured.
   */
  textCapacity?: TextCapacity | null;
  textFrames: number;
  textWork: FrameWork;
}

export interface DecodedImageInfo {
  byteLength: number;
  encodedBitDepth: number;
  encodedHeight: number;
  encodedWidth: number;
  format: ImageFormat;
  height: number;
  orientation: number;
  pixelsSha256: Digest;
  profile: string;
  resolution: ImageResolution;
  sourceColor: SourceColor;
  sourceSha256: Digest;
  width: number;
}

export interface ImageResolution {
  /**
   * Raw fields in encoded axes. Missing EXIF fields remain null; derivation
   * uses Exif's specified defaults (72, 72, inch), never a host DPI default.
   */
  declarations: ResolutionDeclaration[];
  physicalPixelSize: PhysicalPixelSize;
}

export interface ResolutionDeclaration {
  source: ResolutionSource;
  /**
   * Absolute byte offset of the PNG chunk or JPEG marker in source bytes.
   */
  sourceOffset: number;
  /**
   * None preserves an absent EXIF ResolutionUnit. Derivation applies the
   * Exif specification's inch default without rewriting this declaration.
   */
  unit?: ResolutionUnit | null;
  x?: Density | null;
  y?: Density | null;
}

export interface Density {
  denominator: number;
  numerator: number;
}

/**
 * Reduced, positive rational EMU per normalized output pixel.
 */
export interface PixelExtent {
  denominator: number;
  numerator: Emu;
}

export interface ImageWork {
  brushes: number;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  coordinateErrorBound: string;
  draws: number;
  resourceBytes: number;
  resources: number;
}

export interface SourcePageRasterInfo {
  downstreamCoordinateErrorBound: FixedQ32;
  page: SourcePageInfo;
  scene: SceneRasterInfo;
}

export interface SourcePageInfo {
  arcSegments: number;
  generatedCommands: number;
  hiddenSlide: boolean;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  imageClipCoordinateErrorBound?: string;
  layers: SourcePageLayer[];
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  pathCoordinateErrorBound: string;
  placementCoordinateErrorBound: FixedQ32;
  profile: SourcePageProfile;
  slide: string;
  sourceSha256: Digest;
}

export interface SourcePageLayer {
  hiddenObjects: number[];
  kind: SurfaceKind;
  objects: number[];
  part: string;
  templatePlaceholders: number[];
  visible: boolean;
}

export interface SceneRasterInfo {
  profile: string;
  raster: RasterInfo;
  work: SceneWork;
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
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
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
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  coordinateErrorBound: string;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
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

export interface SceneWork {
  clips?: SceneClipWork | null;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  combinedCoordinateErrorBound: string;
  compiledCommands: number;
  compiledPaths: number;
  evaluatedPoints: number;
  /**
   * 2 means the shared matrix candidate failed numeric/precision checks and
   * lowering used the original node chain before any backend call.
   */
  loweringAttempts: number;
  maximumDepth: number;
  /**
   * Conservative point/node work per attempt, including identity instances.
   */
  pointTransformWork: number;
  sourceCommands: number;
  sourcePaths: number;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  transformErrorBound: string;
  transforms: number;
}

export interface SceneClipWork {
  compiledNodes: number;
  sourceNodes: number;
}

export interface TextCapacity {
  /**
   * Complete frame coverage in paint order, including empty text frames.
   */
  frames: FrameCapacity[];
  /**
   * Page-space painted text envelopes, absent from historical measurements.
   * These are before clipping/compositing; intersections are only candidates.
   */
  pageInk?: PageTextInk[] | null;
  profile: string;
}

export interface FrameCapacity {
  cell?: SourceCellAddress | null;
  contentHeight: FixedQ32;
  emergencyLines: number;
  firstHorizontalOverflow?: FrameLine | null;
  /**
   * Counts the actual line search decisions, without rounding to integer EMU.
   */
  horizontalOverflowLines: number;
  /**
   * Unpainted glyph ink, including overhang; not a clipping/overlap verdict.
   */
  inkBounds?: Rect | null;
  inner: Rect1;
  lineCount: number;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  maximumLeftExcess: string;
  maximumRightExcess: FixedQ32;
  object: SourceObjectRef;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  verticalExcess: string;
}

export interface SourceCellAddress {
  column: number;
  row: number;
}

export interface FrameLine {
  line: number;
  paragraph: number;
}

export interface Rect {
  max: Point1;
  min: Point1;
}

export interface Point1 {
  x: FixedQ32;
  y: FixedQ32;
}

/**
 * Q32 EMU before page/group/animation placement. Insets are already applied.
 */
export interface Rect1 {
  max: Point1;
  min: Point1;
}

/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 */
export interface SourceObjectRef {
  nativeId: number;
  part: string;
}

export interface PageTextInk {
  /**
   * Page-space Q32 EMU, including the recorded coordinate uncertainty.
   * None means no nontransparent glyph outline was painted by this frame.
   */
  bounds?: Rect | null;
  cell?: SourceCellAddress | null;
  /**
   * Observations precede the native clip and later layer compositing.
   */
  clippingApplied: boolean;
  coordinateErrorBound: FixedQ32;
  object: SourceObjectRef;
}

export interface SourcePlaybackFrame {
  evaluated: EvaluatedFrame;
  /**
   * Synthetic time-graph keys are scoped to this source slide. Never resolve
   * a bare native id against a master, layout or another slide.
   */
  objectBindings: {
    [k: string]: SourceObjectRef | undefined;
  };
  partSha256: Digest;
  slide: string;
  sourceSha256: Digest;
}

export interface EvaluatedFrame {
  sha256: Digest;
  state: FrameState;
}

export interface FrameState {
  binding: PlaybackBinding;
  containers?: NodeFrame[];
  eventCursor: number;
  /**
   * Exact slide-relative offsets from the original layout center.
   */
  motion?: {
    [k: string]: ExactMotion | undefined;
  };
  nodes: NodeFrame[];
  /**
   * Exact whole-object opacity in [0, 1], before one render-boundary rounding.
   */
  opacity?: {
    [k: string]: ExactValue | undefined;
  };
  presentationStep?: PresentationStepReceipt | null;
  profile: string;
  rotations: {
    [k: string]: ExactRotation | undefined;
  };
  scales?: {
    [k: string]: ExactScale | undefined;
  };
  sequences?: SequenceFrame[];
  time: RationalTime;
  timelineSha256: Digest;
  visibility?: {
    [k: string]: Visibility | undefined;
  };
}

export interface NodeFrame {
  end?: ExactValue | null;
  iteration?: string | null;
  node: TimingNodeId;
  phase: NodePhase;
  progress?: ExactValue | null;
  start?: ExactValue | null;
}

/**
 * Output-only exact reduced ratio. Units are defined by each property channel.
 *
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 */
export interface ExactValue {
  denominator: string;
  numerator: string;
}

/**
 * Position offsets in fractions of the slide width/height. Ratios encode the
 * computed position exactly; the frame profile states any path approximation.
 *
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 */
export interface ExactMotion {
  x: ExactValue;
  y: ExactValue;
}

/**
 * The last presentation step in the sampled, validated event prefix. The
 * enclosing frame binds document/session/generation and hashes this receipt.
 */
export interface PresentationStepReceipt {
  at: ExactValue;
  direction: NavigationDirection;
  outcome: PresentationStepOutcome;
  sequence: number;
}

/**
 * An exact angle with its document dependency still explicit. The layout
 * orientation is resolved only at placement, after source inheritance.
 *
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 */
export interface ExactRotation {
  basis?: RotationBasis;
  denominator: string;
  numerator: string;
}

/**
 * Scale values remain thousandths of a percent until the placement boundary.
 *
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 */
export interface ExactScale {
  x: ExactValue;
  y: ExactValue;
}

export interface SequenceFrame {
  current?: TimingNodeId | null;
  node: TimingNodeId;
  /**
   * Zero-based cursor. The child count denotes the position after the end.
   */
  position: number;
}

/**
 * Exact wire representation. Equality compares author values; compare_time compares instants.
 */
export interface RationalTime {
  ticks: Ticks;
  timescale: Timescale;
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

export interface FillRedirect {
  declaredBy: FillOrigin;
  target: FillOwner;
}

export interface FillOwner {
  part: string;
  target: FillTarget;
}

export interface SourcePageLocation {
  /**
   * None denotes a surface/background declaration.
   */
  object?: number | null;
  part: string;
}

export interface ShapeFailure {
  code: ShapeFailureCode;
  fontSelection?: FontSelectionFailure | null;
  message: string;
}

export interface FontSelectionFailure {
  fontStyle: FontStyle;
  reason: FontSelectionReason;
  /**
   * Index in the actual ManifestParagraphInput.styles, not a native run id.
   */
  style: number;
  typeface: string;
}

export interface SourceImageResult {
  outcome: SourceImageOutcome;
  target: FillTarget;
}

export interface SourceImageBinding {
  declaredBy: FillOrigin;
  image: EffectiveImageFill;
  redirects: FillRedirect[];
  reference: SourceImageReference;
}

export interface EffectiveImageFill {
  compression: FillValue2;
  dpi: FillValue7;
  embed: FillValue;
  link: FillValue1;
  mode: EffectiveImageMode;
  rotateWithShape: FillValue8;
  sourceRect: EffectiveFillRect;
}

export interface FillValue2 {
  declaredBy: FillOrigin;
  value: NativeBlipCompression;
}

export interface FillValue7 {
  declaredBy: FillOrigin;
  value: number;
}

/**
 * Each relationship belongs to the part in its own declaring origin.
 * Empty strings are explicit or profile-default empty relationship IDs.
 */
export interface FillValue {
  declaredBy: FillOrigin;
  value: string;
}

export interface FillValue1 {
  declaredBy: FillOrigin;
  value: string;
}

export interface EffectiveFillTile {
  alignment: FillValue6;
  flip: FillValue5;
  scaleX: FillValue3;
  scaleY: FillValue3;
  translateX: FillValue4;
  translateY: FillValue4;
}

export interface FillValue6 {
  declaredBy: FillOrigin;
  value: NativeFillAlignment;
}

export interface FillValue5 {
  declaredBy: FillOrigin;
  value: NativeTileFlip;
}

export interface FillValue3 {
  declaredBy: FillOrigin;
  value: NativePercentage;
}

export interface FillValue4 {
  declaredBy: FillOrigin;
  value: NativeCoordinate;
}

export interface EffectiveFillRect {
  bottom: FillValue3;
  declaredBy: FillOrigin;
  left: FillValue3;
  right: FillValue3;
  top: FillValue3;
}

export interface FillValue8 {
  declaredBy: FillOrigin;
  value: boolean;
}

export interface SourceImageReference {
  declaredBy: FillOrigin;
  ownerPart: string;
  relationshipId: string;
  targetUri: string;
}

export interface TextPaintLocation {
  paragraph: number;
  run: number;
  sourceOrdinal: number;
}

export interface TextDecorationIssue {
  font: number;
  metric: FontMetric;
  paragraph: number;
  run: number;
  sourceOrdinal: number;
  /**
   * None means unavailable; nonpositive thickness is an unusable metric.
   */
  value?: number | null;
}

export interface SourceFontSelectionFailure {
  object: SourceObjectRef;
  paragraph: number;
  selection: FontSelectionFailure;
  sourceOrdinal: number;
  sourceSha256: Digest;
  /**
   * Every source binding sharing the failed computation style, including
   * insertion style (run=None), with native declaration/theme provenance.
   */
  uses: SourceFontBinding[];
}

export interface SourceFontBinding {
  font: NativeTypeface;
  run?: number | null;
  script: string;
  slot: NativeFontSlot;
  style: number;
  themeScript?: string | null;
}

export interface NativeTypeface {
  /**
   * Original selected font declaration; absent for a fontRef fallback.
   */
  authoredFont?: SourceTextFont | null;
  declaredBy: TextStyleDeclaration;
  /**
   * Script/collection location for an explicit table a:font declaration.
   */
  tableFont?: TableFontBinding | null;
  theme?: ThemeFontBinding | null;
  /**
   * Selected named theme font preserves its own metadata, independently of author hints.
   */
  themeFont?: SourceTextFont | null;
  typeface: string;
}

export interface SourceTextFont {
  charset?: number | null;
  panose?: string | null;
  pitchFamily?: number | null;
  typeface: string;
}

/**
 * Selected whole declaration. Consumers must still resolve its native semantics,
 * retained descendants, colors, theme fonts, resources, effects and permissions.
 * The reference is meaningful only with this result's immutable SourceIndex.
 */
export interface TextStyleDeclaration {
  element: NativeTextElement;
  origin: TextStyleOrigin;
}

export interface TableFontBinding {
  slot: NativeFontSlot;
  /**
   * Index into the table font collection's ordered supplemental list.
   */
  supplemental?: number | null;
}

export interface ThemeFontBinding {
  collection: NativeFontCollectionIndex;
  scheme: SourceThemeSchemeRef;
  slot: NativeFontSlot;
  /**
   * Index into the original ordered supplemental list, not a physical ordinal.
   */
  supplemental?: number | null;
}

export interface SourceThemeSchemeRef {
  part: string;
  sourceOrdinal: number;
}

export interface ItemizationNotice {
  end: number;
  kind: ItemizationNoticeKind;
  start: number;
}
