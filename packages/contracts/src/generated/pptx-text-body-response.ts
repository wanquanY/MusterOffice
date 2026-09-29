/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type PptxTextBodyResponse =
  | {
      status: "evaluated";
      styles: SourceTextBodies;
    }
  | {
      error: PptxFailure;
      status: "error";
    };
export type TextBodyOutcome =
  | {
      body: EffectiveTextBody;
      status: "resolved";
    }
  | {
      reason: TextBodyUnresolved;
      status: "unresolved";
    };
export type NativeTextAnchor = "t" | "ctr" | "b" | "just" | "dist";
/**
 * Native coordinate: bounded integer EMU or exact decimal universal measure.
 */
export type NativeCoordinate = string;
export type NativeTextHorizontalOverflow = "overflow" | "clip";
export type NativeTextVertical =
  "horz" | "vert" | "vert270" | "wordArtVert" | "eaVert" | "mongolianVert" | "wordArtVertRtl";
export type NativeTextVerticalOverflow = "overflow" | "ellipsis" | "clip";
export type NativeTextWrap = "none" | "square";
export type EffectiveTextAutofit =
  | {
      declaredBy: TextBodyOrigin;
      kind: "none";
    }
  | {
      declaredBy: TextBodyOrigin;
      kind: "shape";
    }
  | {
      declaredBy: TextBodyOrigin;
      fontScale: NativePercentage;
      /**
       * Defaulted within the chosen normAutofit element, not inherited from a different choice.
       */
      fontScaleDefaulted: boolean;
      kind: "normal";
      lineSpacingReduction: NativePercentage;
      lineSpacingReductionDefaulted: boolean;
    };
export type TextBodyOrigin =
  | {
      cell: SourceCellAddress;
      kind: "cell";
      object: SourceObjectRef;
      sourceOrdinal: number;
    }
  | {
      cell: SourceCellAddress;
      kind: "cellDefault";
      object: SourceObjectRef;
    }
  | {
      kind: "object";
      object: SourceObjectRef;
      sourceOrdinal: number;
    }
  | {
      defaultKind?: SourceThemeDefaultKind | null;
      kind: "theme";
      part: string;
      sourceOrdinal: number;
    }
  | {
      kind: "profileDefault";
    };
export type SourceThemeDefaultKind = "txDef" | "lnDef" | "spDef";
/**
 * Exact native percentage: int32 thousandths of a percent or decimal percent; ranges depend on use.
 */
export type NativePercentage = string;
export type TextBodyUnresolved =
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
      origin: TextBodyOrigin;
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
export type TextBodyProfile = "drawingml-body-inheritance-draft-v1";
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

export interface SourceTextBodies {
  objects: SourceTextBodyResult[];
  profile: TextBodyProfile;
  sourceSha256: Digest;
  surface: string;
}
export interface SourceTextBodyResult {
  nativeId: number;
  outcome: TextBodyOutcome;
}
export interface EffectiveTextBody {
  attributes: SourceTextBodyAttributes;
  autofit: EffectiveTextAutofit;
  origins: {
    anchor?: TextBodyOrigin;
    bottomInset?: TextBodyOrigin;
    centerAnchor?: TextBodyOrigin;
    columnSpacing?: TextBodyOrigin;
    columns?: TextBodyOrigin;
    compatibleLineSpacing?: TextBodyOrigin;
    forceAntialiasing?: TextBodyOrigin;
    fromWordArt?: TextBodyOrigin;
    horizontalOverflow?: TextBodyOrigin;
    leftInset?: TextBodyOrigin;
    paragraphSpacing?: TextBodyOrigin;
    rightInset?: TextBodyOrigin;
    rightToLeftColumns?: TextBodyOrigin;
    rotation?: TextBodyOrigin;
    topInset?: TextBodyOrigin;
    upright?: TextBodyOrigin;
    vertical?: TextBodyOrigin;
    verticalOverflow?: TextBodyOrigin;
    wrap?: TextBodyOrigin;
  };
}
/**
 * All 19 fields are populated after the explicit profile defaults are applied.
 */
export interface SourceTextBodyAttributes {
  anchor?: NativeTextAnchor | null;
  bottomInset?: NativeCoordinate | null;
  centerAnchor?: boolean | null;
  columnSpacing?: NativeCoordinate | null;
  columns?: number | null;
  compatibleLineSpacing?: boolean | null;
  forceAntialiasing?: boolean | null;
  fromWordArt?: boolean | null;
  horizontalOverflow?: NativeTextHorizontalOverflow | null;
  leftInset?: NativeCoordinate | null;
  paragraphSpacing?: boolean | null;
  rightInset?: NativeCoordinate | null;
  rightToLeftColumns?: boolean | null;
  rotation?: number | null;
  topInset?: NativeCoordinate | null;
  upright?: boolean | null;
  vertical?: NativeTextVertical | null;
  verticalOverflow?: NativeTextVerticalOverflow | null;
  wrap?: NativeTextWrap | null;
}
export interface SourceCellAddress {
  column: number;
  row: number;
}
export interface SourceObjectRef {
  nativeId: number;
  part: string;
}
export interface PptxFailure {
  code: PptxFailureCode;
  message: string;
}
