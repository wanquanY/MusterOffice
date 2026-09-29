/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */
import type { Accessibility, Appearance, Crop, Document, ExportSettings, FontFace, Layout, Master, MotionPath, MotionPoint, Object, OperationEntry, Point, PresentationSequence, RationalTime, Resource, RetainedParagraph, Rgba, ScaleValue, Size, Slide, Table, TableCell, TableCellStyle1, TableColumn, TableRow, TemplateDefinition, TextBody, Theme, Timeline, Transform } from './part-002.js';

/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "DocumentAction".
 */
export type DocumentAction =
  | {
      documentId: DocumentId;
      kind: "import";
      source: AssetBinding;
    }
  | {
      document: Document;
      kind: "create";
    }
  | {
      definition: TemplateDefinition;
      kind: "describeTemplate";
    }
  | {
      bindings: {
        [k: string]: BindingValue | undefined;
      };
      definition: TemplateDefinition;
      documentId: DocumentId;
      kind: "instantiateTemplate";
      templateDigest: Digest;
    }
  | {
      baseRevision: Digest;
      documentId: DocumentId;
      kind: "apply";
      operations: OperationEntry[];
    }
  | {
      baseRevision: Digest;
      documentId: DocumentId;
      kind: "export";
      settings: ExportSettings;
    };

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "DocumentId".
 */
export type DocumentId = string;

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "RequestId".
 */
export type RequestId = string;

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "ResourceId".
 */
export type ResourceId = string;

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "FontId".
 */
export type FontId = string;

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "ModelVersion".
 */
export type ModelVersion = "musteroffice.presentation/0.1-draft";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Inherited".
 */
export type Inherited =
  | {
      kind: "inherit";
    }
  | {
      kind: "value";
      value: Fill;
    };

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Fill".
 */
export type Fill =
  | {
      kind: "none";
    }
  | {
      color: Color;
      kind: "solid";
    };

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Color".
 */
export type Color =
  | {
      kind: "srgb";
      rgba: Rgba;
    }
  | {
      kind: "theme";
      slot: ThemeColor;
    };

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "ThemeColor".
 */
export type ThemeColor =
  | "dark1"
  | "light1"
  | "dark2"
  | "light2"
  | "accent1"
  | "accent2"
  | "accent3"
  | "accent4"
  | "accent5"
  | "accent6"
  | "hyperlink"
  | "followedHyperlink";

/**
 * Signed int64 EMU. 1 point = 12700 EMU. Range requires semantic validation.
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Emu".
 */
export type Emu = string;

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "LayoutId".
 */
export type LayoutId = string;

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "MasterId".
 */
export type MasterId = string;

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "ObjectId".
 */
export type ObjectId = string;

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "ThemeId".
 */
export type ThemeId = string;

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Stroke".
 */
export type Stroke =
  | {
      kind: "none";
    }
  | {
      /**
       * Absent retains an unresolved declaration, not an implicit flat cap.
       */
      cap?: LineCap | null;
      color: Color;
      /**
       * Absent retains the source/default distinction.
       */
      join?: LineJoin | null;
      kind: "solid";
      width: Emu;
    };

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "LineCap".
 */
export type LineCap = "flat" | "round" | "square";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "LineJoin".
 */
export type LineJoin =
  | {
      kind: "round";
    }
  | {
      kind: "bevel";
    }
  | {
      kind: "miter";
      /**
       * Ratio in 1/100000 units: 400000 denotes four times line width.
       * The ratio compares full miter length with the full stroke width.
       */
      limit?: number | null;
    };

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "ObjectContent".
 */
export type ObjectContent =
  | {
      kind: "table";
      table: Table;
    }
  | {
      children: ObjectId[];
      kind: "retainedSource";
      native_kind: RetainedObjectKind;
      paragraphs: RetainedParagraph[];
    }
  | {
      geometry: Geometry;
      kind: "shape";
      text?: TextBody | null;
    }
  | {
      crop: Crop;
      kind: "picture";
      resource: ResourceId;
    }
  | {
      children: ObjectId[];
      kind: "group";
      viewport: Size;
    }
  | {
      end: ConnectorEndpoint;
      kind: "connector";
      start: ConnectorEndpoint;
    };

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "ColumnId".
 */
export type ColumnId = string;

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "CellId".
 */
export type CellId = string;

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "TableVerticalAlignment".
 */
export type TableVerticalAlignment = "top" | "center" | "bottom" | "justified" | "distributed";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "OverflowPolicy".
 */
export type OverflowPolicy = "report" | "clip" | "growShape";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "ParagraphId".
 */
export type ParagraphId = string;

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "InlineContent".
 */
export type InlineContent =
  | {
      kind: "text";
      text: string;
    }
  | {
      kind: "break";
    }
  | {
      kind: "tab";
    };

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "RunId".
 */
export type RunId = string;

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Alignment".
 */
export type Alignment = "start" | "center" | "end" | "justify";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "TextDirection".
 */
export type TextDirection = "leftToRight" | "rightToLeft" | "verticalRightToLeft" | "verticalLeftToRight";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "RowId".
 */
export type RowId = string;

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "RetainedObjectKind".
 */
export type RetainedObjectKind = "shape" | "picture" | "group" | "connector" | "graphicFrame";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "RetainedRunKind".
 */
export type RetainedRunKind = "text" | "break" | "field";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Geometry".
 */
export type Geometry =
  | {
      kind: "rectangle";
    }
  | {
      kind: "ellipse";
    }
  | {
      kind: "roundRectangle";
      radius: Emu;
    }
  | {
      commands: PathCommand[];
      kind: "path";
      viewport: Size;
    };

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "PathCommand".
 */
export type PathCommand =
  | {
      kind: "move";
      to: Point;
    }
  | {
      kind: "line";
      to: Point;
    }
  | {
      control: Point;
      kind: "quadratic";
      to: Point;
    }
  | {
      control1: Point;
      control2: Point;
      kind: "cubic";
      to: Point;
    }
  | {
      kind: "close";
    };

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "ConnectorEndpoint".
 */
export type ConnectorEndpoint =
  | {
      kind: "free";
      position: Point;
    }
  | {
      kind: "attached";
      object: ObjectId;
      site: number;
    };

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "ContainerId".
 */
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

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "SlideId".
 */
export type SlideId = string;

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "ResourceKind".
 */
export type ResourceKind = "font" | "picture" | "audio" | "video" | "sourcePackage" | "embeddedWorkbook" | "model3d";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "NativeEditConstraint".
 */
export type NativeEditConstraint =
  | "missingDirectTransform"
  | "retainedTransform"
  | "compatibilityBranch"
  | "structuredLeaf"
  | "dynamicField"
  | "timingReferences"
  | "retainedReferences";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "SourceBindingProfile".
 */
export type SourceBindingProfile =
  | "presentationml-retained-fields-v1-draft"
  | "presentationml-retained-fields-v2-draft"
  | "presentationml-retained-fields-v3-draft";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "TimelineVersion".
 */
export type TimelineVersion = "musteroffice.timeline/0.1-draft" | "musteroffice.timeline/0.2-draft";

/**
 * Signed int64 ticks. Range requires semantic validation.
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Ticks".
 */
export type Ticks = string;

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Timescale".
 */
export type Timescale = number;

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Effect".
 */
export type Effect =
  | {
      composition?: RotationComposition;
      from: number;
      kind: "rotation";
      target: ObjectId;
      to: number;
    }
  | {
      from: ScaleValue;
      kind: "scale";
      target: ObjectId;
      to: ScaleValue;
    }
  | {
      kind: "setVisibility";
      target: ObjectId;
      value: Visibility;
    }
  | {
      from: MotionPoint;
      kind: "motionLine";
      target: ObjectId;
      to: MotionPoint;
    }
  | {
      kind: "motionPath";
      path: MotionPath;
      target: ObjectId;
    }
  | {
      kind: "fade";
      target: ObjectId;
      transition: FadeTransition;
    };

/**
 * Rotation is composed before object/group placement. Layout replaces earlier
 * animation offsets while preserving the document's local orientation. Add
 * sums the sampled offset with the lower-priority visible rotation stack.
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "RotationComposition".
 */
export type RotationComposition = "absolute" | "layout" | "add";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Visibility".
 */
export type Visibility = "visible" | "hidden";

/**
 * Exact decimal fraction of the slide dimension; canonicalized without rounding.
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "MotionCoordinate".
 */
export type MotionCoordinate = string;

/**
 * Source control points remain editable; subdivision belongs only to the
 * immutable playback plan. Close returns to the initial `from` point.
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "MotionSegment".
 */
export type MotionSegment =
  | {
      kind: "line";
      to: MotionPoint;
    }
  | {
      control1: MotionPoint;
      control2: MotionPoint;
      kind: "cubic";
      to: MotionPoint;
    }
  | {
      kind: "close";
    };

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "FadeTransition".
 */
export type FadeTransition = "in" | "out";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "TimeCondition".
 */
export type TimeCondition =
  | {
      kind: "never";
    }
  | {
      kind: "at";
      offset: RationalTime;
    }
  | {
      delay: RationalTime;
      event: NodeEvent;
      kind: "after";
      node: TimingNodeId;
    }
  | {
      delay: RationalTime;
      kind: "click";
      target?: ObjectId | null;
    }
  | {
      delay: RationalTime;
      direction: NavigationDirection;
      kind: "navigation";
      target?: ObjectId | null;
    };

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "NodeEvent".
 */
export type NodeEvent = ("end" | "onEnd") | "begin" | "onBegin";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "TimingNodeId".
 */
export type TimingNodeId = string;

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "NavigationDirection".
 */
export type NavigationDirection = "next" | "previous";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "FillMode".
 */
export type FillMode = ("remove" | "freeze") | "hold";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "RepeatDuration".
 */
export type RepeatDuration = "indefinite" | RationalTime;

/**
 * Admission of new begin instances within one parent activation. Ancestor
 * reactivation resets this policy, including `Never`. Omission preserves the
 * existing draft's once-per-parent behavior, independently of native defaults.
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "RestartMode".
 */
export type RestartMode = "never" | "always" | "whenNotActive";

/**
 * A flat disjunction of native begin conditions. The single-condition wire
 * representation remains unchanged; alternatives cannot recursively nest.
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "StartCondition".
 */
export type StartCondition =
  | {
      /**
       * @minItems 1
       */
      conditions: [TimeCondition, ...TimeCondition[]];
      kind: "anyOf";
    }
  | TimeCondition;

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "ContainerDuration".
 */
export type ContainerDuration =
  | {
      kind: "automatic";
    }
  | {
      duration: RationalTime;
      kind: "fixed";
    }
  | {
      kind: "indefinite";
    };

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "ContainerKind".
 */
export type ContainerKind = "parallel" | "sequence";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "NextAction".
 */
export type NextAction = "none" | "seek";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "PreviousAction".
 */
export type PreviousAction = "none" | "skipTimed";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "PresentationRole".
 */
export type PresentationRole =
  | {
      kind: "mainSequence";
    }
  | {
      kind: "effect";
      preset: PresentationPreset;
      trigger: PresentationTrigger;
    };

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "PresentationPreset".
 */
export type PresentationPreset = "appear" | "disappear" | "spin" | "growShrink" | "customMotion" | "fadeIn" | "fadeOut";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "PresentationTrigger".
 */
export type PresentationTrigger = "click" | "withPrevious" | "afterPrevious";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "TemplateVersion".
 */
export type TemplateVersion = "musteroffice.presentation-template/1-draft";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "ParameterTarget".
 */
export type ParameterTarget =
  | {
      kind: "textRun";
      maxScalars: number;
      minScalars: number;
      object: ObjectId;
      paragraph: ParagraphId;
      run: RunId;
    }
  | {
      kind: "resource";
      mediaTypes: string[];
      resource: ResourceId;
    }
  | {
      kind: "themeColor";
      slot: ThemeColor;
      theme: ThemeId;
    }
  | {
      kind: "transform";
      object: ObjectId;
    };

/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "BindingValue".
 */
export type BindingValue =
  | {
      kind: "text";
      value: string;
    }
  | {
      kind: "resource";
      value: Resource;
    }
  | {
      kind: "color";
      value: Rgba;
    }
  | {
      kind: "transform";
      value: Transform;
    };

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Operation".
 */
export type Operation =
  | {
      kind: "editTable";
      object: ObjectId;
      operation: TableOperation;
    }
  | {
      kind: "setPresentationSequence";
      sequence: PresentationSequence;
      slide: SlideId;
    }
  | {
      kind: "setTimeline";
      slide: SlideId;
      timeline?: Timeline | null;
    }
  | {
      kind: "setTitle";
      title: string;
    }
  | {
      index: number;
      kind: "insertSlide";
      slide: Slide;
    }
  | {
      kind: "deleteSlide";
      policy: DeletePolicy;
      slide: SlideId;
    }
  | {
      index: number;
      kind: "moveSlide";
      slide: SlideId;
    }
  | {
      kind: "setLayout";
      layout?: LayoutId | null;
      slide: SlideId;
    }
  | {
      kind: "putTheme";
      theme: Theme;
    }
  | {
      kind: "putMaster";
      master: Master;
    }
  | {
      kind: "putLayout";
      layout: Layout;
    }
  | {
      kind: "attachResource";
      resource: Resource;
    }
  | {
      kind: "detachResource";
      resource: ResourceId;
    }
  | {
      font: FontFace;
      kind: "putFont";
    }
  | {
      index: number;
      kind: "insertObject";
      object: Object;
    }
  | {
      kind: "deleteObject";
      object: ObjectId;
      policy: DeletePolicy;
    }
  | {
      index: number;
      kind: "moveObject";
      object: ObjectId;
      parent: ContainerId;
      transform: Transform;
    }
  | {
      kind: "setTransform";
      object: ObjectId;
      transform: Transform;
    }
  | {
      appearance: Appearance;
      kind: "setAppearance";
      object: ObjectId;
    }
  | {
      accessibility: Accessibility;
      kind: "setAccessibility";
      object: ObjectId;
    }
  | {
      kind: "setText";
      object: ObjectId;
      text: TextBody;
    }
  | {
      delete: number;
      insert: string;
      kind: "spliceText";
      object: ObjectId;
      paragraph: ParagraphId;
      run: RunId;
      start: number;
    };

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "TableOperation".
 */
export type TableOperation =
  | {
      kind: "replace";
      table: Table;
    }
  | {
      cell: CellId;
      kind: "setCellText";
      text?: TextBody | null;
    }
  | {
      cell: CellId;
      kind: "setCellStyle";
      style: TableCellStyle1;
    }
  | {
      column: ColumnId;
      kind: "setColumnWidth";
      width: Emu;
    }
  | {
      height: Emu;
      kind: "setRowHeight";
      row: RowId;
    }
  | {
      columns: number;
      kind: "merge";
      origin: CellId;
      rows: number;
    }
  | {
      cell: CellId;
      kind: "split";
    }
  | {
      index: number;
      kind: "insertRow";
      row: TableRow;
    }
  | {
      cells: TableCell[];
      column: TableColumn;
      index: number;
      kind: "insertColumn";
    }
  | {
      kind: "deleteRow";
      row: RowId;
    }
  | {
      column: ColumnId;
      kind: "deleteColumn";
    }
  | {
      kind: "reorderRows";
      order: RowId[];
    }
  | {
      kind: "reorderColumns";
      order: ColumnId[];
    };

/**
 * Finite values retain the existing integer wire form. Infinity is a named
 * alternative, never a sentinel count or a pre-expanded list of iterations.
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "RepeatCount".
 */
export type RepeatCount = "indefinite" | number;

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "DeletePolicy".
 */
export type DeletePolicy = "rejectDependencies" | "cascade";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "OperationId".
 */
export type OperationId = string;

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "FontDelivery".
 */
export type FontDelivery = "referenceOnly";

/**
 * Canonical uint64 byte length. Range requires semantic validation.
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "ByteLength".
 */
export type ByteLength = string;

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "FontManifestProfile".
 */
export type FontManifestProfile = "explicit-font-resource-manifest-draft-v1";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "TypefaceMappingPolicy".
 */
export type TypefaceMappingPolicy =
  | {
      kind: "exactFamily";
    }
  | {
      kind: "substitution";
      profileSha256: Digest;
      reason: string;
    };

/**
 * Explicit source policy. An embedded snapshot is never silently substituted
 * for a requested linked source, nor does inspection grant network authority.
 *
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "ImageSourceSelection".
 */
export type ImageSourceSelection = "embeddedSnapshot" | "linkedSource";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "ImageSampling".
 */
export type ImageSampling = "nearest" | "linear";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "ContractVersion".
 */
export type ContractVersion = "musteroffice.computation/1-draft";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "OperationProfile".
 */
export type OperationProfile = "presentations-pptx-resource-delivery-v1-draft" | "presentations-author-model-v01-draft";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Inherited10".
 */
export type Inherited10 =
  | {
      kind: "inherit";
    }
  | {
      kind: "value";
      value: TableVerticalAlignment;
    };

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Inherited2".
 */
export type Inherited2 =
  | {
      kind: "inherit";
    }
  | {
      kind: "value";
      value: Stroke;
    };

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Inherited3".
 */
export type Inherited3 =
  | {
      kind: "inherit";
    }
  | {
      kind: "value";
      value: Alignment;
    };

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Inherited4".
 */
export type Inherited4 =
  | {
      kind: "inherit";
    }
  | {
      kind: "value";
      value: TextDirection;
    };

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Inherited5".
 */
export type Inherited5 =
  | {
      kind: "inherit";
    }
  | {
      kind: "value";
      value: Emu;
    };

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Inherited6".
 */
export type Inherited6 =
  | {
      kind: "inherit";
    }
  | {
      kind: "value";
      value: FontId;
    };

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Inherited7".
 */
export type Inherited7 =
  | {
      kind: "inherit";
    }
  | {
      kind: "value";
      value: Color;
    };

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Inherited8".
 */
export type Inherited8 =
  | {
      kind: "inherit";
    }
  | {
      kind: "value";
      value: boolean;
    };

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "Inherited9".
 */
export type Inherited9 =
  | {
      kind: "inherit";
    }
  | {
      kind: "value";
      value: string;
    };

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "PresentationGroupStart".
 */
export type PresentationGroupStart = "automatic" | "next";

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "TableCellMerge".
 */
export type TableCellMerge =
  | {
      columns: number;
      kind: "span";
      rows: number;
    }
  | {
      kind: "covered";
      origin: CellId;
    };

/**
 * A computation request carries no identity, permissions, durable job or output mode.
 */
export interface OperationRequest {
  action: DocumentAction;
  contractVersion: ContractVersion;
  profileId: OperationProfile;
  requestId: RequestId;
}

/**
 * This interface was referenced by `OperationRequest`'s JSON-Schema
 * via the `definition` "AssetBinding".
 */
export interface AssetBinding {
  assetId: RequestId;
  resourceId: ResourceId;
}
