/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type HostRequest =
  | {
      operation: "capabilities";
    }
  | {
      id: SchemaId;
      operation: "getSchema";
    }
  | {
      operation: "beginUpload";
      request: UploadRequest;
    }
  | {
      operation: "getUpload";
      uploadId: RequestId;
    }
  | {
      operation: "sealUpload";
      uploadId: RequestId;
    }
  | {
      operation: "cancelUpload";
      uploadId: RequestId;
    }
  | {
      assetId: RequestId;
      operation: "readAsset";
    }
  | {
      operation: "submit";
      request: OperationRequest;
    }
  | {
      jobId: RequestId;
      operation: "getJob";
    }
  | {
      jobId: RequestId;
      operation: "cancelJob";
    }
  | {
      documentId: DocumentId;
      operation: "readDocument";
      revision?: Digest | null;
    };
export type SchemaId =
  | "host-request"
  | "host-response"
  | "operation-request"
  | "operation-job"
  | "host-capabilities"
  | "schema-document"
  | "document"
  | "upload-request"
  | "upload-info"
  | "asset-info";
/**
 * Canonical uint64 byte length. Range requires semantic validation.
 */
export type ByteLength = string;
export type Digest = string;
export type RequestId = string;
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
export type DocumentId = string;
export type ResourceId = string;
export type FontId = string;
export type ModelVersion = "musteroffice.presentation/0.1-draft";
export type Inherited =
  | {
      kind: "inherit";
    }
  | {
      kind: "value";
      value: Fill;
    };
export type Fill =
  | {
      kind: "none";
    }
  | {
      color: Color;
      kind: "solid";
    };
export type Color =
  | {
      kind: "srgb";
      rgba: Rgba;
    }
  | {
      kind: "theme";
      slot: ThemeColor;
    };
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
 */
export type Emu = string;
export type LayoutId = string;
export type MasterId = string;
export type ObjectId = string;
export type ThemeId = string;
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
export type LineCap = "flat" | "round" | "square";
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
export type ColumnId = string;
export type CellId = string;
export type TableVerticalAlignment = "top" | "center" | "bottom" | "justified" | "distributed";
export type OverflowPolicy = "report" | "clip" | "growShape";
export type ParagraphId = string;
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
export type RunId = string;
export type Alignment = "start" | "center" | "end" | "justify";
export type TextDirection = "leftToRight" | "rightToLeft" | "verticalRightToLeft" | "verticalLeftToRight";
/**
 * Native paragraph line spacing. Percentage is measured against the line's
 * largest font size by the shared layout engine; 100000 means 100%, 150000 150%.
 */
export type ParagraphLineSpacing =
  | {
      kind: "percent";
      value: number;
    }
  | {
      height: Emu;
      kind: "exact";
    };
export type RowId = string;
export type RetainedObjectKind = "shape" | "picture" | "group" | "connector" | "graphicFrame";
export type RetainedRunKind = "text" | "break" | "field";
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
export type SlideId = string;
export type ResourceKind = "font" | "picture" | "audio" | "video" | "sourcePackage" | "embeddedWorkbook" | "model3d";
export type NativeEditConstraint =
  | "missingDirectTransform"
  | "retainedTransform"
  | "compatibilityBranch"
  | "structuredLeaf"
  | "dynamicField"
  | "timingReferences"
  | "retainedReferences";
export type SourceBindingProfile =
  | "presentationml-retained-fields-v1-draft"
  | "presentationml-retained-fields-v2-draft"
  | "presentationml-retained-fields-v3-draft"
  | "presentationml-retained-fields-v4-draft";
export type TimelineVersion = "musteroffice.timeline/0.1-draft" | "musteroffice.timeline/0.2-draft";
/**
 * Signed int64 ticks. Range requires semantic validation.
 */
export type Ticks = string;
export type Timescale = number;
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
 */
export type RotationComposition = "absolute" | "layout" | "add";
export type Visibility = "visible" | "hidden";
/**
 * Exact decimal fraction of the slide dimension; canonicalized without rounding.
 */
export type MotionCoordinate = string;
/**
 * Source control points remain editable; subdivision belongs only to the
 * immutable playback plan. Close returns to the initial `from` point.
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
export type FadeTransition = "in" | "out";
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
export type NodeEvent = ("end" | "onEnd") | "begin" | "onBegin";
export type TimingNodeId = string;
export type NavigationDirection = "next" | "previous";
export type FillMode = ("remove" | "freeze") | "hold";
export type RepeatDuration = "indefinite" | RationalTime;
/**
 * Admission of new begin instances within one parent activation. Ancestor
 * reactivation resets this policy, including `Never`. Omission preserves the
 * existing draft's once-per-parent behavior, independently of native defaults.
 */
export type RestartMode = "never" | "always" | "whenNotActive";
/**
 * A flat disjunction of native begin conditions. The single-condition wire
 * representation remains unchanged; alternatives cannot recursively nest.
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
export type ContainerKind = "parallel" | "sequence";
export type NextAction = "none" | "seek";
export type PreviousAction = "none" | "skipTimed";
export type PresentationRole =
  | {
      kind: "mainSequence";
    }
  | {
      kind: "effect";
      preset: PresentationPreset;
      trigger: PresentationTrigger;
    };
export type PresentationPreset = "appear" | "disappear" | "spin" | "growShrink" | "customMotion" | "fadeIn" | "fadeOut";
export type PresentationTrigger = "click" | "withPrevious" | "afterPrevious";
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
 */
export type RepeatCount = "indefinite" | number;
export type DeletePolicy = "rejectDependencies" | "cascade";
export type OperationId = string;
export type FontDelivery = "referenceOnly";
export type FontManifestProfile = "explicit-font-resource-manifest-draft-v1";
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
 */
export type ImageSourceSelection = "embeddedSnapshot" | "linkedSource";
export type ImageSampling = "nearest" | "linear";
export type ContractVersion = "musteroffice.operations/1-draft";
export type OutputMode = "auto" | "sync" | "job";
export type OperationProfile = "presentations-pptx-resource-delivery-v1-draft" | "presentations-author-model-v01-draft";

export interface UploadRequest {
  descriptor: AssetDescriptor;
  requestId: RequestId;
}
export interface AssetDescriptor {
  byteLength: ByteLength;
  /**
   * A declaration, not evidence of successful decoding or safe execution.
   */
  mediaType: string;
  sha256: Digest;
}
export interface OperationRequest {
  action: DocumentAction;
  contractVersion: ContractVersion;
  outputMode: OutputMode;
  profileId: OperationProfile;
  requestId: RequestId;
}
export interface AssetBinding {
  assetId: RequestId;
  resourceId: ResourceId;
}
/**
 * Document declarations and immutable source provenance. Revisions, compilation
 * caches, clocks and decoder state live outside this model.
 * The example is a complete editable 16:9 slide with a text shape. Coordinates
 * and sizes are decimal EMU strings (12700 EMU per point). Empty theme, master,
 * layout, font and resource maps are valid; inherited text uses the caller's
 * explicit delivery defaults. Copy slides/objects with distinct IDs to expand
 * the deck. Inspect optional feature definitions only when those features are
 * needed; the example requires no system font discovery or external resources.
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
 */
export interface Layout {
  background: Inherited;
  defaultText: CharacterStyle;
  id: LayoutId;
  master: MasterId;
  name: string;
  objects: ObjectId[];
}
export interface Rgba {
  alpha: number;
  blue: number;
  green: number;
  red: number;
}
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
export interface Accessibility {
  decorative: boolean;
  description: string;
  title: string;
}
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
export interface Table {
  columns: TableColumn[];
  rows: TableRow[];
}
export interface TableColumn {
  id: ColumnId;
  width: Emu;
}
export interface TableRow {
  /**
   * One entry per grid column, even when covered by another cell.
   */
  cells: TableCell[];
  height: Emu;
  id: RowId;
}
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
export interface TextBody {
  insets: Insets;
  overflow: OverflowPolicy;
  paragraphs: Paragraph[];
  style: CharacterStyle;
  wrap: boolean;
}
export interface Insets {
  bottom: Emu;
  left: Emu;
  right: Emu;
  top: Emu;
}
export interface Paragraph {
  defaultRunStyle: CharacterStyle;
  id: ParagraphId;
  runs: TextRun[];
  style: ParagraphStyle;
}
export interface TextRun {
  content: InlineContent;
  id: RunId;
  style: CharacterStyle;
}
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
  /**
   * Relative to the paragraph margin; negative values create a hanging indent.
   */
  indent?: Emu | null;
  leftMargin?: Emu | null;
  /**
   * Omitted values retain native inheritance and historical document digests.
   */
  lineSpacing?: ParagraphLineSpacing | null;
  rightMargin?: Emu | null;
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
export interface RetainedParagraph {
  id: ParagraphId;
  runs: RetainedTextRun[];
}
export interface RetainedTextRun {
  id: RunId;
  kind: RetainedRunKind;
  text: string;
}
export interface Point {
  x: Emu;
  y: Emu;
}
export interface Size {
  height: Emu;
  width: Emu;
}
export interface Crop {
  bottom: number;
  left: number;
  right: number;
  top: number;
}
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
 */
export interface Slide {
  background: Inherited;
  hidden: boolean;
  id: SlideId;
  layout?: LayoutId | null;
  name: string;
  objects: ObjectId[];
}
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
 */
export interface NativeRunBinding {
  constraint?: NativeEditConstraint | null;
  paragraph: number;
  run: number;
}
/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
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
 */
export interface Timeline {
  format: TimelineVersion;
  nodes: TimingNode[];
  tree?: TimingTree | null;
}
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
 */
export interface RationalTime {
  ticks: Ticks;
  timescale: Timescale;
}
export interface ScaleValue {
  x: number;
  y: number;
}
export interface MotionPoint {
  x: MotionCoordinate;
  y: MotionCoordinate;
}
/**
 * Connected native path. Coordinates are absolute offsets from the original
 * layout center, measured in slide fractions. Pacing uses length in this
 * normalized coordinate space, before scaling the axes to slide dimensions.
 */
export interface MotionPath {
  from: MotionPoint;
  segments: MotionSegment[];
}
/**
 * Local behavior clock. Percentages use native thousandths of one percent;
 * 100000 speed is normal playback. The clock is independent of effect values.
 */
export interface TimeTransform {
  accelerationMilliPercent: number;
  autoReverse: boolean;
  decelerationMilliPercent: number;
  speedMilliPercent: number;
}
export interface TimingTree {
  containers: TimingContainer[];
  roots: TimingNodeId[];
}
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
 */
export interface SequenceNavigation {
  concurrent: boolean;
  nextAction: NextAction;
  nextConditions: TimeCondition[];
  previousAction: PreviousAction;
  previousConditions: TimeCondition[];
}
export interface OperationEntry {
  operation: Operation;
  operationId: OperationId;
}
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
export interface PresentationSequence {
  groups: PresentationGroup[];
}
export interface PresentationGroup {
  batches: PresentationBatch[];
  /**
   * Automatic is valid only for the first group; Next waits for navigation.
   */
  start: "automatic" | "next";
}
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
export interface ExportSettings {
  delivery: DeliverySettings;
  fontAssetId?: RequestId | null;
  renderer: RendererIdentity;
  resources: AssetBinding[];
}
/**
 * Explicit render/export choices are input identity. Pixels preserve page
 * aspect ratio; the pipeline derives a viewport covering the entire page.
 */
export interface DeliverySettings {
  colorContext: ColorContext;
  defaults: ExportDefaults;
  fonts?: FontManifest | null;
  imageSource: ImageSourceSelection;
  previewWidth: number;
  sampling: ImageSampling;
}
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
export interface FontManifest {
  faces: ManifestFace[];
  fonts: CascadeFont[];
  profile: FontManifestProfile;
  typefaces: ManifestTypeface[];
}
export interface ManifestFace {
  family: FontNameBinding;
  font: number;
  postscript?: FontNameBinding | null;
  subfamily: FontNameBinding;
}
export interface FontNameBinding {
  expected: string;
  /**
   * Exact index in VerifiedFont metadata.names, preserving original record order.
   */
  record: number;
}
/**
 * Explicit resource bundle bindings, not system font names or legal permissions.
 */
export interface CascadeFont {
  byteLength: ByteLength;
  expectedSha256: Digest;
  faceIndex: number;
  offset: ByteLength;
}
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
export interface ManifestInstance {
  face: number;
  /**
   * All axes are validated, including instances unused by this paragraph.
   */
  variations: ShapeVariation[];
}
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
