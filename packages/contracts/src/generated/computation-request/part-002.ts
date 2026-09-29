/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */
import type { Alignment, AssetBinding, ByteLength, CellId, Color, ColumnId, ContainerDuration, ContainerId, ContainerKind, Digest, DocumentId, Effect, Emu, Fill, FillMode, FontDelivery, FontId, FontManifestProfile, ImageSampling, ImageSourceSelection, Inherited, InlineContent, LayoutId, MasterId, ModelVersion, MotionCoordinate, MotionSegment, NativeEditConstraint, NextAction, ObjectContent, ObjectId, Operation, OperationId, OverflowPolicy, ParagraphId, ParameterTarget, PresentationRole, PreviousAction, RepeatCount, RepeatDuration, RequestId, ResourceId, ResourceKind, RestartMode, RetainedRunKind, RowId, RunId, SlideId, SourceBindingProfile, StartCondition, Stroke, TableVerticalAlignment, TemplateVersion, TextDirection, ThemeId, Ticks, TimeCondition, TimelineVersion, Timescale, TimingNodeId } from './part-001.js';
import type { ManifestTypeface, RendererIdentity } from './part-003.js';

/**
 * Document declarations and immutable source provenance. Revisions, compilation
 * caches, clocks and decoder state live outside this model.
 * The example is a complete editable 16:9 slide with a text shape. Coordinates
 * and sizes are decimal EMU strings (12700 EMU per point). Empty theme, master,
 * layout, font and resource maps are valid; inherited text uses the caller's
 * explicit delivery defaults. Copy slides/objects with distinct IDs to expand
 * the deck. Inspect optional feature definitions only when those features are
 * needed; the example requires no system font discovery or external resources.
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Document".
 */
export interface Document {
  fonts: {
    [k: string]: FontFace | undefined;
  };
  format: ModelVersion;
  id: DocumentId;
  layouts: {
    [k: string]: Layout | undefined;
  };
  masters: {
    [k: string]: Master | undefined;
  };
  objects: {
    [k: string]: Object | undefined;
  };
  pageSize: Size;
  resources: {
    [k: string]: Resource | undefined;
  };
  slideOrder: SlideId[];
  slides: {
    [k: string]: Slide | undefined;
  };
  /**
   * Immutable native addresses/constraints. Known fields are edited in
   * objects; this provenance never becomes a separate mutable document.
   */
  sourceBindings?: SourceBindings | null;
  themes: {
    [k: string]: Theme | undefined;
  };
  /**
   * Slide-owned animation graphs; empty storage preserves older author digests.
   */
  timelines?: {
    [k: string]: Timeline | undefined;
  };
  title: string;
}

/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "FontFace".
 */
export interface FontFace {
  faceIndex: number;
  family: string;
  id: FontId;
  italic: boolean;
  resource: ResourceId;
  weight: number;
}

/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Layout".
 */
export interface Layout {
  background: Inherited;
  defaultText: CharacterStyle;
  id: LayoutId;
  master: MasterId;
  name: string;
  objects: ObjectId[];
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Rgba".
 */
export interface Rgba {
  alpha: number;
  blue: number;
  green: number;
  red: number;
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "CharacterStyle".
 */
export interface CharacterStyle {
  bold?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: boolean;
      };
  color?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: Color;
      };
  font?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: FontId;
      };
  italic?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: boolean;
      };
  language?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: string;
      };
  size?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: Emu;
      };
  underline?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: boolean;
      };
}

/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Master".
 */
export interface Master {
  background: Inherited;
  defaultText: CharacterStyle;
  id: MasterId;
  objects: ObjectId[];
  theme: ThemeId;
}

/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Object".
 */
export interface Object {
  accessibility: Accessibility;
  appearance: Appearance;
  content: ObjectContent;
  id: ObjectId;
  parent: ContainerId;
  /**
   * Missing only for retained native coordinates that cannot be represented
   * as a complete direct declaration. Never substitute a resolved identity.
   */
  transform?: Transform | null;
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Accessibility".
 */
export interface Accessibility {
  decorative: boolean;
  description: string;
  title: string;
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Appearance".
 */
export interface Appearance {
  fill?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: Fill;
      };
  stroke?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: Stroke;
      };
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Table".
 */
export interface Table {
  columns: TableColumn[];
  rows: TableRow[];
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "TableColumn".
 */
export interface TableColumn {
  id: ColumnId;
  width: Emu;
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "TableRow".
 */
export interface TableRow {
  /**
   * One entry per grid column, even when covered by another cell.
   */
  cells: TableCell[];
  height: Emu;
  id: RowId;
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "TableCell".
 */
export interface TableCell {
  /**
   * Row/column/cell identities are table-scoped. Text identities remain
   * document-scoped so existing anchors retain their unambiguous meaning.
   */
  id: string;
  merge?:
    | {
        columns: number;
        kind: "span";
        rows: number;
      }
    | {
        kind: "covered";
        origin: CellId;
      };
  style?: TableCellStyle;
  text?: TextBody | null;
}

export interface TableCellStyle {
  borders?: TableCellBorders;
  fill?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: Fill;
      };
  verticalAlignment?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: TableVerticalAlignment;
      };
}

export interface TableCellBorders {
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

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "TextBody".
 */
export interface TextBody {
  insets: Insets;
  overflow: OverflowPolicy;
  paragraphs: Paragraph[];
  style: CharacterStyle;
  wrap: boolean;
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Insets".
 */
export interface Insets {
  bottom: Emu;
  left: Emu;
  right: Emu;
  top: Emu;
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Paragraph".
 */
export interface Paragraph {
  defaultRunStyle: CharacterStyle;
  id: ParagraphId;
  runs: TextRun[];
  style: ParagraphStyle;
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "TextRun".
 */
export interface TextRun {
  content: InlineContent;
  id: RunId;
  style: CharacterStyle;
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "ParagraphStyle".
 */
export interface ParagraphStyle {
  alignment?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: Alignment;
      };
  direction?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: TextDirection;
      };
  spaceAfter?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: Emu;
      };
  spaceBefore?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: Emu;
      };
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "RetainedParagraph".
 */
export interface RetainedParagraph {
  id: ParagraphId;
  runs: RetainedTextRun[];
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "RetainedTextRun".
 */
export interface RetainedTextRun {
  id: RunId;
  kind: RetainedRunKind;
  text: string;
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Point".
 */
export interface Point {
  x: Emu;
  y: Emu;
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Size".
 */
export interface Size {
  height: Emu;
  width: Emu;
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Crop".
 */
export interface Crop {
  bottom: number;
  left: number;
  right: number;
  top: number;
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Transform".
 */
export interface Transform {
  flipHorizontal: boolean;
  flipVertical: boolean;
  origin: Point;
  /**
   * Units of 1/60000 degree. Author direction is preserved.
   */
  rotation: number;
  size: Size;
}

/**
 * Opaque authorized handle and declared content identity; byte verification belongs to the host.
 *
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Resource".
 */
export interface Resource {
  id: ResourceId;
  kind: ResourceKind;
  mediaType: string;
  sha256: Digest;
}

/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Slide".
 */
export interface Slide {
  background: Inherited;
  hidden: boolean;
  id: SlideId;
  layout?: LayoutId | null;
  name: string;
  objects: ObjectId[];
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "SourceBindings".
 */
export interface SourceBindings {
  /**
   * Immutable origin namespace used to derive source-local IDs. Absence
   * retains the original document-ID-derived representation. A whole-deck
   * instance pins this namespace before changing its document identity.
   * This is provenance for calculation, never host access authority.
   */
  identityScope?: DocumentId | null;
  layouts: {
    /**
     * This interface was referenced by `undefined`'s JSON-Schema definition
     * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
     */
    [k: string]: string | undefined;
  };
  masters: {
    /**
     * This interface was referenced by `undefined`'s JSON-Schema definition
     * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
     */
    [k: string]: string | undefined;
  };
  objects: {
    [k: string]: NativeObjectBinding | undefined;
  };
  profile: SourceBindingProfile;
  resource: ResourceId;
  slides: {
    /**
     * This interface was referenced by `undefined`'s JSON-Schema definition
     * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
     */
    [k: string]: string | undefined;
  };
  themes: {
    /**
     * This interface was referenced by `undefined`'s JSON-Schema definition
     * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
     */
    [k: string]: string | undefined;
  };
}

/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "NativeObjectBinding".
 */
export interface NativeObjectBinding {
  nativeId: number;
  part: string;
  runs: {
    [k: string]: NativeRunBinding | undefined;
  };
  /**
   * None means the complete direct transform can be changed in place.
   */
  transformConstraint?: NativeEditConstraint | null;
}

/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "NativeRunBinding".
 */
export interface NativeRunBinding {
  constraint?: NativeEditConstraint | null;
  paragraph: number;
  run: number;
}

/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Theme".
 */
export interface Theme {
  colors: {
    accent1?: Rgba;
    accent2?: Rgba;
    accent3?: Rgba;
    accent4?: Rgba;
    accent5?: Rgba;
    accent6?: Rgba;
    dark1?: Rgba;
    dark2?: Rgba;
    followedHyperlink?: Rgba;
    hyperlink?: Rgba;
    light1?: Rgba;
    light2?: Rgba;
  };
  defaultText: CharacterStyle;
  id: ThemeId;
  name: string;
}

/**
 * Behaviors plus an optional explicit timing forest. The legacy graph retains
 * its byte representation; version 0.2 owns every behavior through tree roots.
 *
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Timeline".
 */
export interface Timeline {
  format: TimelineVersion;
  nodes: TimingNode[];
  tree?: TimingTree | null;
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "TimingNode".
 */
export interface TimingNode {
  duration: RationalTime;
  effect: Effect;
  /**
   * Earliest resolved eligible end; absent conditions add no end constraint.
   */
  endConditions?: TimeCondition[];
  fill: FillMode;
  id: TimingNodeId;
  /**
   * Additional bound in local active time, before speed scaling.
   */
  repeatDuration?: RepeatDuration | null;
  /**
   * Native count in thousandths, or explicit indefinite repetition.
   */
  repeatMilli: "indefinite" | number;
  restart?: RestartMode;
  start: StartCondition;
  timeTransform?: TimeTransform | null;
}

/**
 * Exact wire representation. Equality compares author values; compare_time compares instants.
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "RationalTime".
 */
export interface RationalTime {
  ticks: Ticks;
  timescale: Timescale;
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "ScaleValue".
 */
export interface ScaleValue {
  x: number;
  y: number;
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "MotionPoint".
 */
export interface MotionPoint {
  x: MotionCoordinate;
  y: MotionCoordinate;
}

/**
 * Connected native path. Coordinates are absolute offsets from the original
 * layout center, measured in slide fractions. Pacing uses length in this
 * normalized coordinate space, before scaling the axes to slide dimensions.
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "MotionPath".
 */
export interface MotionPath {
  from: MotionPoint;
  segments: MotionSegment[];
}

/**
 * Local behavior clock. Percentages use native thousandths of one percent;
 * 100000 speed is normal playback. The clock is independent of effect values.
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "TimeTransform".
 */
export interface TimeTransform {
  accelerationMilliPercent: number;
  autoReverse: boolean;
  decelerationMilliPercent: number;
  speedMilliPercent: number;
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "TimingTree".
 */
export interface TimingTree {
  containers: TimingContainer[];
  roots: TimingNodeId[];
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "TimingContainer".
 */
export interface TimingContainer {
  children: TimingNodeId[];
  duration: ContainerDuration;
  endConditions?: TimeCondition[];
  fill: FillMode;
  id: TimingNodeId;
  kind: ContainerKind;
  navigation?: SequenceNavigation | null;
  /**
   * Native presentation identity. It participates in initial playback state
   * and editable export; it never changes the container's declared clock.
   */
  presentation?: PresentationRole | null;
  restart?: RestartMode;
  start: StartCondition;
  /**
   * Filter the container's simple time before its descendants consume it.
   * Compilation validates the supported clock domain; this is never copied
   * into the leaves or interpreted as an independent per-effect easing.
   */
  timeTransform?: TimeTransform | null;
}

/**
 * Sequence controls are document computation, independent of host buttons or
 * keyboard bindings. Conditions are disjunctions, like begin/end conditions.
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "SequenceNavigation".
 */
export interface SequenceNavigation {
  concurrent: boolean;
  nextAction: NextAction;
  nextConditions: TimeCondition[];
  previousAction: PreviousAction;
  previousConditions: TimeCondition[];
}

/**
 * A template is an exact document revision plus typed editable parameters.
 * Catalog identity, ownership, storage and inference are outside this contract.
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "TemplateDefinition".
 */
export interface TemplateDefinition {
  format: TemplateVersion;
  parameters: {
    [k: string]: Parameter | undefined;
  };
  source: TemplateSource;
}

/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Parameter".
 */
export interface Parameter {
  label: string;
  required: boolean;
  target: ParameterTarget;
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "TemplateSource".
 */
export interface TemplateSource {
  documentId: DocumentId;
  revision: Digest;
  semanticDigest: Digest;
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "OperationEntry".
 */
export interface OperationEntry {
  operation: Operation;
  operationId: OperationId;
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "TableCellStyle".
 */
export interface TableCellStyle1 {
  borders?: TableCellBorders;
  fill?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: Fill;
      };
  verticalAlignment?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: TableVerticalAlignment;
      };
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "PresentationSequence".
 */
export interface PresentationSequence {
  groups: PresentationGroup[];
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "PresentationGroup".
 */
export interface PresentationGroup {
  batches: PresentationBatch[];
  /**
   * Automatic is valid only for the first group; Next waits for navigation.
   */
  start: "automatic" | "next";
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "PresentationBatch".
 */
export interface PresentationBatch {
  delay: RationalTime1;
  effects: PresentationEffect[];
}

/**
 * Exact wire representation. Equality compares author values; compare_time compares instants.
 */
export interface RationalTime1 {
  ticks: Ticks;
  timescale: Timescale;
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "PresentationEffect".
 */
export interface PresentationEffect {
  delay: RationalTime2;
  duration: RationalTime;
  effect: Effect;
  fill: FillMode;
  /**
   * Stable behavior identity. Container identities are allocated separately.
   */
  id: string;
  repeatDuration?: RepeatDuration | null;
  repeatMilli: RepeatCount;
  timeTransform?: TimeTransform | null;
}

/**
 * Exact wire representation. Equality compares author values; compare_time compares instants.
 */
export interface RationalTime2 {
  ticks: Ticks;
  timescale: Timescale;
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "ExportSettings".
 */
export interface ExportSettings {
  delivery: DeliverySettings;
  fontAssetId?: RequestId | null;
  renderer: RendererIdentity;
  resources: AssetBinding[];
}

/**
 * Explicit render/export choices are input identity. Pixels preserve page
 * aspect ratio; the pipeline derives a viewport covering the entire page.
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "DeliverySettings".
 */
export interface DeliverySettings {
  colorContext: ColorContext;
  defaults: ExportDefaults;
  fonts?: FontManifest | null;
  imageSource: ImageSourceSelection;
  previewWidth: number;
  sampling: ImageSampling;
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "ColorContext".
 */
export interface ColorContext {
  /**
   * Explicit unassociated sRGB and alpha, supplied by the owning style.
   *
   * @minItems 4
   * @maxItems 4
   */
  placeholder?: [number, number, number, number] | null;
  systemColors: {
    /**
     * @minItems 3
     * @maxItems 3
     */
    "3dDkShadow"?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    "3dLight"?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    activeBorder?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    activeCaption?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    appWorkspace?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    background?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    btnFace?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    btnHighlight?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    btnShadow?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    btnText?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    captionText?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    gradientActiveCaption?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    gradientInactiveCaption?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    grayText?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    highlight?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    highlightText?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    hotLight?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    inactiveBorder?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    inactiveCaption?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    inactiveCaptionText?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    infoBk?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    infoText?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    menu?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    menuBar?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    menuHighlight?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    menuText?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    scrollBar?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    window?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    windowFrame?: [number, number, number];
    /**
     * @minItems 3
     * @maxItems 3
     */
    windowText?: [number, number, number];
  };
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "ExportDefaults".
 */
export interface ExportDefaults {
  fontDelivery: FontDelivery;
  /**
   * Explicit host choice. Export alone does not resolve, load or embed system fonts.
   */
  fontFamily: string;
  pageBackground: Rgba;
  textColor: Rgba;
  textSize: Emu;
  /**
   * All 12 native theme color slots are required for generated fallback definitions.
   */
  themeColors: {
    accent1?: Rgba;
    accent2?: Rgba;
    accent3?: Rgba;
    accent4?: Rgba;
    accent5?: Rgba;
    accent6?: Rgba;
    dark1?: Rgba;
    dark2?: Rgba;
    followedHyperlink?: Rgba;
    hyperlink?: Rgba;
    light1?: Rgba;
    light2?: Rgba;
  };
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "FontManifest".
 */
export interface FontManifest {
  faces: ManifestFace[];
  fonts: CascadeFont[];
  profile: FontManifestProfile;
  typefaces: ManifestTypeface[];
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "ManifestFace".
 */
export interface ManifestFace {
  family: FontNameBinding;
  font: number;
  postscript?: FontNameBinding | null;
  subfamily: FontNameBinding;
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "FontNameBinding".
 */
export interface FontNameBinding {
  expected: string;
  /**
   * Exact index in VerifiedFont metadata.names, preserving original record order.
   */
  record: number;
}

/**
 * Explicit resource bundle bindings, not system font names or legal permissions.
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "CascadeFont".
 */
export interface CascadeFont {
  byteLength: ByteLength;
  expectedSha256: Digest;
  faceIndex: number;
  offset: ByteLength;
}
