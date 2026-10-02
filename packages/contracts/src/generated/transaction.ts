/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "DocumentId".
 */
export type DocumentId = string;
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
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
      kind: "setSlideName";
      name: string;
      slide: SlideId;
    }
  | {
      background: Inherited8;
      kind: "setSlideBackground";
      slide: SlideId;
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
      kind: "ensureResource";
      resource: Resource;
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
      fill: Inherited8;
      kind: "setFill";
      object: ObjectId;
    }
  | {
      kind: "setStroke";
      object: ObjectId;
      stroke: Inherited9;
    }
  | {
      crop?: Crop | null;
      kind: "setPicture";
      object: ObjectId;
      resource: ResourceId;
    }
  | {
      crop: Crop;
      kind: "setPictureCrop";
      object: ObjectId;
    }
  | {
      geometry: Geometry;
      kind: "setGeometry";
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
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "ObjectId".
 */
export type ObjectId = string;
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "ColumnId".
 */
export type ColumnId = string;
/**
 * Signed int64 EMU. 1 point = 12700 EMU. Range requires semantic validation.
 *
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Emu".
 */
export type Emu = string;
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "CellId".
 */
export type CellId = string;
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "LineCap".
 */
export type LineCap = "flat" | "round" | "square";
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "TableVerticalAlignment".
 */
export type TableVerticalAlignment = "top" | "center" | "bottom" | "justified" | "distributed";
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "OverflowPolicy".
 */
export type OverflowPolicy = "report" | "clip" | "growShape";
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "FontId".
 */
export type FontId = string;
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "ParagraphId".
 */
export type ParagraphId = string;
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "RunId".
 */
export type RunId = string;
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Alignment".
 */
export type Alignment = "start" | "center" | "end" | "justify";
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "TextDirection".
 */
export type TextDirection = "leftToRight" | "rightToLeft" | "verticalRightToLeft" | "verticalLeftToRight";
/**
 * Native paragraph line spacing. Percentage is measured against the line's
 * largest font size by the shared layout engine; 100000 means 100%, 150000 150%.
 *
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "ParagraphLineSpacing".
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
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "RowId".
 */
export type RowId = string;
/**
 * Signed int64 ticks. Range requires semantic validation.
 *
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Ticks".
 */
export type Ticks = string;
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Timescale".
 */
export type Timescale = number;
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "RotationComposition".
 */
export type RotationComposition = "absolute" | "layout" | "add";
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Visibility".
 */
export type Visibility = "visible" | "hidden";
/**
 * Exact decimal fraction of the slide dimension; canonicalized without rounding.
 *
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "MotionCoordinate".
 */
export type MotionCoordinate = string;
/**
 * Source control points remain editable; subdivision belongs only to the
 * immutable playback plan. Close returns to the initial `from` point.
 *
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "FadeTransition".
 */
export type FadeTransition = "in" | "out";
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "FillMode".
 */
export type FillMode = ("remove" | "freeze") | "hold";
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "RepeatDuration".
 */
export type RepeatDuration = "indefinite" | RationalTime2;
/**
 * Finite values retain the existing integer wire form. Infinity is a named
 * alternative, never a sentinel count or a pre-expanded list of iterations.
 *
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "RepeatCount".
 */
export type RepeatCount = "indefinite" | number;
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "SlideId".
 */
export type SlideId = string;
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "TimelineVersion".
 */
export type TimelineVersion = "musteroffice.timeline/0.1-draft" | "musteroffice.timeline/0.2-draft";
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "TimeCondition".
 */
export type TimeCondition =
  | {
      kind: "never";
    }
  | {
      kind: "at";
      offset: RationalTime2;
    }
  | {
      delay: RationalTime2;
      event: NodeEvent;
      kind: "after";
      node: TimingNodeId;
    }
  | {
      delay: RationalTime2;
      kind: "click";
      target?: ObjectId | null;
    }
  | {
      delay: RationalTime2;
      direction: NavigationDirection;
      kind: "navigation";
      target?: ObjectId | null;
    };
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "NodeEvent".
 */
export type NodeEvent = ("end" | "onEnd") | "begin" | "onBegin";
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "TimingNodeId".
 */
export type TimingNodeId = string;
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "NavigationDirection".
 */
export type NavigationDirection = "next" | "previous";
/**
 * Admission of new begin instances within one parent activation. Ancestor
 * reactivation resets this policy, including `Never`. Omission preserves the
 * existing draft's once-per-parent behavior, independently of native defaults.
 *
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "RestartMode".
 */
export type RestartMode = "never" | "always" | "whenNotActive";
/**
 * A flat disjunction of native begin conditions. The single-condition wire
 * representation remains unchanged; alternatives cannot recursively nest.
 *
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "ContainerDuration".
 */
export type ContainerDuration =
  | {
      kind: "automatic";
    }
  | {
      duration: RationalTime2;
      kind: "fixed";
    }
  | {
      kind: "indefinite";
    };
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "ContainerKind".
 */
export type ContainerKind = "parallel" | "sequence";
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "NextAction".
 */
export type NextAction = "none" | "seek";
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "PreviousAction".
 */
export type PreviousAction = "none" | "skipTimed";
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "PresentationPreset".
 */
export type PresentationPreset = "appear" | "disappear" | "spin" | "growShrink" | "customMotion" | "fadeIn" | "fadeOut";
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "PresentationTrigger".
 */
export type PresentationTrigger = "click" | "withPrevious" | "afterPrevious";
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Inherited8".
 */
export type Inherited8 =
  | {
      kind: "inherit";
    }
  | {
      kind: "value";
      value: Fill;
    };
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "LayoutId".
 */
export type LayoutId = string;
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "DeletePolicy".
 */
export type DeletePolicy = "rejectDependencies" | "cascade";
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "ThemeId".
 */
export type ThemeId = string;
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "MasterId".
 */
export type MasterId = string;
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "ResourceId".
 */
export type ResourceId = string;
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "ResourceKind".
 */
export type ResourceKind = "font" | "picture" | "audio" | "video" | "sourcePackage" | "embeddedWorkbook" | "model3d";
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "RetainedObjectKind".
 */
export type RetainedObjectKind = "shape" | "picture" | "group" | "connector" | "graphicFrame";
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "RetainedRunKind".
 */
export type RetainedRunKind = "text" | "break" | "field";
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Inherited9".
 */
export type Inherited9 =
  | {
      kind: "inherit";
    }
  | {
      kind: "value";
      value: Stroke;
    };
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "OperationId".
 */
export type OperationId = string;
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "RequestId".
 */
export type RequestId = string;
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Inherited".
 */
export type Inherited =
  | {
      kind: "inherit";
    }
  | {
      kind: "value";
      value: Alignment;
    };
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Inherited2".
 */
export type Inherited2 =
  | {
      kind: "inherit";
    }
  | {
      kind: "value";
      value: TextDirection;
    };
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Inherited3".
 */
export type Inherited3 =
  | {
      kind: "inherit";
    }
  | {
      kind: "value";
      value: Emu;
    };
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Inherited4".
 */
export type Inherited4 =
  | {
      kind: "inherit";
    }
  | {
      kind: "value";
      value: FontId;
    };
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Inherited5".
 */
export type Inherited5 =
  | {
      kind: "inherit";
    }
  | {
      kind: "value";
      value: Color;
    };
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Inherited6".
 */
export type Inherited6 =
  | {
      kind: "inherit";
    }
  | {
      kind: "value";
      value: boolean;
    };
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Inherited7".
 */
export type Inherited7 =
  | {
      kind: "inherit";
    }
  | {
      kind: "value";
      value: string;
    };
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "PresentationGroupStart".
 */
export type PresentationGroupStart = "automatic" | "next";
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
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

export interface Transaction {
  baseRevision: Digest;
  documentId: DocumentId;
  operations: OperationEntry[];
  requestId: RequestId;
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "OperationEntry".
 */
export interface OperationEntry {
  operation: Operation;
  operationId: OperationId;
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Table".
 */
export interface Table {
  columns: TableColumn[];
  rows: TableRow[];
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "TableColumn".
 */
export interface TableColumn {
  id: ColumnId;
  width: Emu;
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Rgba".
 */
export interface Rgba {
  alpha: number;
  blue: number;
  green: number;
  red: number;
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Insets".
 */
export interface Insets {
  bottom: Emu;
  left: Emu;
  right: Emu;
  top: Emu;
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Paragraph".
 */
export interface Paragraph {
  defaultRunStyle: CharacterStyle;
  id: ParagraphId;
  runs: TextRun[];
  style: ParagraphStyle;
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "TextRun".
 */
export interface TextRun {
  content: InlineContent;
  id: RunId;
  style: CharacterStyle;
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
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
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "PresentationSequence".
 */
export interface PresentationSequence {
  groups: PresentationGroup[];
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "PresentationBatch".
 */
export interface PresentationBatch {
  delay: RationalTime;
  effects: PresentationEffect[];
}
/**
 * Additional delay after the preceding batch ends (or group activation).
 */
export interface RationalTime {
  ticks: Ticks;
  timescale: Timescale;
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "PresentationEffect".
 */
export interface PresentationEffect {
  delay: RationalTime1;
  duration: RationalTime2;
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
 * Offset from this batch's activation, not from the preceding effect.
 */
export interface RationalTime1 {
  ticks: Ticks;
  timescale: Timescale;
}
/**
 * Exact wire representation. Equality compares author values; compare_time compares instants.
 *
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "RationalTime".
 */
export interface RationalTime2 {
  ticks: Ticks;
  timescale: Timescale;
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "ScaleValue".
 */
export interface ScaleValue {
  x: number;
  y: number;
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "TimeTransform".
 */
export interface TimeTransform {
  accelerationMilliPercent: number;
  autoReverse: boolean;
  decelerationMilliPercent: number;
  speedMilliPercent: number;
}
/**
 * Behaviors plus an optional explicit timing forest. The legacy graph retains
 * its byte representation; version 0.2 owns every behavior through tree roots.
 *
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Timeline".
 */
export interface Timeline {
  format: TimelineVersion;
  nodes: TimingNode[];
  tree?: TimingTree | null;
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "TimingNode".
 */
export interface TimingNode {
  duration: RationalTime2;
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
   * Finite values retain the existing integer wire form. Infinity is a named
   * alternative, never a sentinel count or a pre-expanded list of iterations.
   */
  repeatMilli: "indefinite" | number;
  restart?: RestartMode;
  start: StartCondition;
  timeTransform?: TimeTransform | null;
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "TimingTree".
 */
export interface TimingTree {
  containers: TimingContainer[];
  roots: TimingNodeId[];
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Slide".
 */
export interface Slide {
  background: Inherited8;
  hidden: boolean;
  id: SlideId;
  layout?: LayoutId | null;
  name: string;
  objects: ObjectId[];
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Master".
 */
export interface Master {
  background: Inherited8;
  defaultText: CharacterStyle;
  id: MasterId;
  objects: ObjectId[];
  theme: ThemeId;
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Layout".
 */
export interface Layout {
  background: Inherited8;
  defaultText: CharacterStyle;
  id: LayoutId;
  master: MasterId;
  name: string;
  objects: ObjectId[];
}
/**
 * Opaque authorized handle and declared content identity; byte verification belongs to the host.
 *
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Resource".
 */
export interface Resource {
  id: ResourceId;
  kind: ResourceKind;
  mediaType: string;
  sha256: Digest;
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Accessibility".
 */
export interface Accessibility {
  decorative: boolean;
  description?: string;
  title?: string;
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "RetainedParagraph".
 */
export interface RetainedParagraph {
  id: ParagraphId;
  runs: RetainedTextRun[];
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "RetainedTextRun".
 */
export interface RetainedTextRun {
  id: RunId;
  kind: RetainedRunKind;
  text: string;
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Point".
 */
export interface Point {
  x: Emu;
  y: Emu;
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Size".
 */
export interface Size {
  height: Emu;
  width: Emu;
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
 * via the `definition` "Crop".
 */
export interface Crop {
  bottom: number;
  left: number;
  right: number;
  top: number;
}
/**
 * This interface was referenced by `Transaction`'s JSON-Schema
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
 * This interface was referenced by `Transaction`'s JSON-Schema
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
