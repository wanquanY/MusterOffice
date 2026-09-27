/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "DocumentId".
 */
export type DocumentId = string;
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "RequestId".
 */
export type RequestId = string;
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "ResourceId".
 */
export type ResourceId = string;
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "FontId".
 */
export type FontId = string;
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "ModelVersion".
 */
export type ModelVersion = "musteroffice.presentation/0.1-draft";
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "Emu".
 */
export type Emu = string;
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "LayoutId".
 */
export type LayoutId = string;
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "MasterId".
 */
export type MasterId = string;
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "ObjectId".
 */
export type ObjectId = string;
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "ThemeId".
 */
export type ThemeId = string;
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "LineCap".
 */
export type LineCap = "flat" | "round" | "square";
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "ObjectContent".
 */
export type ObjectContent =
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
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "RetainedObjectKind".
 */
export type RetainedObjectKind = "shape" | "picture" | "group" | "connector" | "graphicFrame";
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "ParagraphId".
 */
export type ParagraphId = string;
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "RunId".
 */
export type RunId = string;
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "RetainedRunKind".
 */
export type RetainedRunKind = "text" | "break" | "field";
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "OverflowPolicy".
 */
export type OverflowPolicy = "report" | "clip" | "growShape";
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "Alignment".
 */
export type Alignment = "start" | "center" | "end" | "justify";
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "TextDirection".
 */
export type TextDirection = "leftToRight" | "rightToLeft" | "verticalRightToLeft" | "verticalLeftToRight";
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "SlideId".
 */
export type SlideId = string;
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "ResourceKind".
 */
export type ResourceKind = "font" | "picture" | "audio" | "video" | "sourcePackage" | "embeddedWorkbook" | "model3d";
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "SourceBindingProfile".
 */
export type SourceBindingProfile = "presentationml-retained-fields-v1-draft";
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "TimelineVersion".
 */
export type TimelineVersion = "musteroffice.timeline/0.1-draft" | "musteroffice.timeline/0.2-draft";
/**
 * Signed int64 ticks. Range requires semantic validation.
 *
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "Ticks".
 */
export type Ticks = string;
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "Timescale".
 */
export type Timescale = number;
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "TimeCondition".
 */
export type TimeCondition =
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
    };
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "NodeEvent".
 */
export type NodeEvent = "begin" | "end";
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "TimingNodeId".
 */
export type TimingNodeId = string;
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "FillMode".
 */
export type FillMode = ("remove" | "freeze") | "hold";
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "RepeatDuration".
 */
export type RepeatDuration = "indefinite" | RationalTime;
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "ContainerKind".
 */
export type ContainerKind = "parallel" | "sequence";
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "Operation".
 */
export type Operation =
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
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "DeletePolicy".
 */
export type DeletePolicy = "rejectDependencies" | "cascade";
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "OperationId".
 */
export type OperationId = string;
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "FontDelivery".
 */
export type FontDelivery = "referenceOnly";
/**
 * Canonical uint64 byte length. Range requires semantic validation.
 *
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "ByteLength".
 */
export type ByteLength = string;
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "FontManifestProfile".
 */
export type FontManifestProfile = "explicit-font-resource-manifest-draft-v1";
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "ImageSourceSelection".
 */
export type ImageSourceSelection = "embeddedSnapshot" | "linkedSource";
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "ImageSampling".
 */
export type ImageSampling = "nearest" | "linear";
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "ContractVersion".
 */
export type ContractVersion = "musteroffice.computation/1-draft";
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "OperationProfile".
 */
export type OperationProfile = "presentations-pptx-resource-delivery-v1-draft" | "presentations-author-model-v01-draft";
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "Effect".
 */
export type Effect = {
  from: number;
  kind: "rotation";
  target: ObjectId;
  to: number;
};
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * Finite values retain the existing integer wire form. Infinity is a named
 * alternative, never a sentinel count or a pre-expanded list of iterations.
 *
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "RepeatCount".
 */
export type RepeatCount = "indefinite" | number;

/**
 * The operation and its caller-supplied base. Resources are transferred outside
 * JSON; their logical identities are already bound by the operation.
 */
export interface Invocation {
  request: OperationRequest;
  snapshot?: SnapshotRecord | null;
}
/**
 * A computation request carries no identity, permissions, durable job or output mode.
 *
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "OperationRequest".
 */
export interface OperationRequest {
  action: DocumentAction;
  contractVersion: ContractVersion;
  profileId: OperationProfile;
  requestId: RequestId;
}
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "AssetBinding".
 */
export interface AssetBinding {
  assetId: RequestId;
  resourceId: ResourceId;
}
/**
 * Document declarations and immutable source provenance. Revisions, compilation
 * caches, clocks and decoder state live outside this model.
 *
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "Rgba".
 */
export interface Rgba {
  alpha: number;
  blue: number;
  green: number;
  red: number;
}
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "Accessibility".
 */
export interface Accessibility {
  decorative: boolean;
  description: string;
  title: string;
}
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "RetainedParagraph".
 */
export interface RetainedParagraph {
  id: ParagraphId;
  runs: RetainedTextRun[];
}
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "RetainedTextRun".
 */
export interface RetainedTextRun {
  id: RunId;
  kind: RetainedRunKind;
  text: string;
}
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "Point".
 */
export interface Point {
  x: Emu;
  y: Emu;
}
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "Size".
 */
export interface Size {
  height: Emu;
  width: Emu;
}
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "Insets".
 */
export interface Insets {
  bottom: Emu;
  left: Emu;
  right: Emu;
  top: Emu;
}
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "Paragraph".
 */
export interface Paragraph {
  defaultRunStyle: CharacterStyle;
  id: ParagraphId;
  runs: TextRun[];
  style: ParagraphStyle;
}
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "TextRun".
 */
export interface TextRun {
  content: InlineContent;
  id: RunId;
  style: CharacterStyle;
}
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "Crop".
 */
export interface Crop {
  bottom: number;
  left: number;
  right: number;
  top: number;
}
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "SourceBindings".
 */
export interface SourceBindings {
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
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "Timeline".
 */
export interface Timeline {
  format: TimelineVersion;
  nodes: TimingNode[];
  tree?: TimingTree | null;
}
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "TimingNode".
 */
export interface TimingNode {
  duration: RationalTime;
  /**
   * The current graph activates each node once (native restart="never").
   */
  effect: {
    from: number;
    kind: "rotation";
    target: ObjectId;
    to: number;
  };
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
  start: TimeCondition;
  timeTransform?: TimeTransform | null;
}
/**
 * Exact wire representation. Equality compares author values; compare_time compares instants.
 *
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "RationalTime".
 */
export interface RationalTime {
  ticks: Ticks;
  timescale: Timescale;
}
/**
 * Local behavior clock. Percentages use native thousandths of one percent;
 * 100000 speed is normal playback. The clock is independent of effect values.
 *
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "TimeTransform".
 */
export interface TimeTransform {
  accelerationMilliPercent: number;
  autoReverse: boolean;
  decelerationMilliPercent: number;
  speedMilliPercent: number;
}
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "TimingTree".
 */
export interface TimingTree {
  containers: TimingContainer[];
  roots: TimingNodeId[];
}
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "TimingContainer".
 */
export interface TimingContainer {
  children: TimingNodeId[];
  duration: ContainerDuration;
  endConditions?: TimeCondition[];
  fill: FillMode;
  id: TimingNodeId;
  kind: ContainerKind;
  start: TimeCondition;
}
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "OperationEntry".
 */
export interface OperationEntry {
  operation: Operation;
  operationId: OperationId;
}
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
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
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "FontManifest".
 */
export interface FontManifest {
  faces: ManifestFace[];
  fonts: CascadeFont[];
  profile: FontManifestProfile;
  typefaces: ManifestTypeface[];
}
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
 * via the `definition` "ManifestFace".
 */
export interface ManifestFace {
  family: FontNameBinding;
  font: number;
  postscript?: FontNameBinding | null;
  subfamily: FontNameBinding;
}
/**
 * This interface was referenced by `Invocation`'s JSON-Schema
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
