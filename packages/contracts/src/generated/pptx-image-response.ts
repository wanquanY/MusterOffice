/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type PptxImagesResponse =
  | {
      images: SourceImageResources;
      status: "inspected";
    }
  | {
      error: PptxFailure;
      status: "error";
    };
/**
 * Canonical uint64 byte length. Range requires semantic validation.
 */
export type ByteLength = string;
export type Digest = string;
/**
 * Explicit source policy. An embedded snapshot is never silently substituted
 * for a requested linked source, nor does inspection grant network authority.
 */
export type ImageSourceSelection = "embeddedSnapshot" | "linkedSource";
export type SourceImageOutcome =
  | {
      binding: SourceImageBinding;
      resource: number;
      status: "available";
    }
  | {
      binding: SourceImageBinding;
      status: "externalRequired";
    }
  | {
      reason: FillUnresolved;
      status: "unresolvedFill";
    }
  | {
      issue: ImageReferenceIssue;
      status: "unresolvedReference";
    }
  | {
      status: "notImage";
    };
export type FillOrigin =
  | {
      kind: "declaration";
      owner: FillOwner;
      sourceOrdinal: number;
    }
  | {
      kind: "tableStyle";
      part: string;
      sourceOrdinal: number;
      via: FillOwner;
    }
  | {
      kind: "theme";
      part: string;
      referenceOrdinal: number;
      sourceOrdinal: number;
      styleIndex: number;
      via: FillOwner;
    }
  | {
      kind: "schemaDefault";
      part: string;
      sourceOrdinal: number;
    }
  | {
      kind: "profileDefault";
    };
export type FillTarget =
  | {
      kind: "object";
      nativeId: number;
    }
  | {
      kind: "line";
      nativeId: number;
    }
  | {
      kind: "picture";
      nativeId: number;
    }
  | {
      cell: SourceCellAddress;
      kind: "tableCell";
      nativeId: number;
    }
  | {
      cell: SourceCellAddress;
      edge: TableCellEdge;
      kind: "tableCellBorder";
      nativeId: number;
    }
  | {
      kind: "tableBackground";
      nativeId: number;
    }
  | {
      kind: "tableStyleFill";
      nativeId: number;
      region?: TableStyleRegion | null;
    }
  | {
      edge: TableStyleEdge;
      kind: "tableStyleBorder";
      nativeId: number;
      region: TableStyleRegion;
    }
  | {
      kind: "rootGroup";
    }
  | {
      kind: "background";
    };
export type TableCellEdge = "left" | "right" | "top" | "bottom" | "topLeftToBottomRight" | "bottomLeftToTopRight";
export type TableStyleRegion =
  | "wholeTbl"
  | "band1H"
  | "band2H"
  | "band1V"
  | "band2V"
  | "lastCol"
  | "firstCol"
  | "lastRow"
  | "seCell"
  | "swCell"
  | "firstRow"
  | "neCell"
  | "nwCell";
export type TableStyleEdge =
  | "left"
  | "right"
  | "top"
  | "bottom"
  | "insideHorizontal"
  | "insideVertical"
  | "topLeftToBottomRight"
  | "topRightToBottomLeft";
export type NativeBlipCompression = "email" | "screen" | "print" | "hqprint" | "none";
export type EffectiveImageMode =
  | {
      declaredBy: FillOrigin;
      kind: "tile";
      tile: EffectiveFillTile;
    }
  | {
      declaredBy: FillOrigin;
      fillRect: EffectiveFillRect;
      kind: "stretch";
    };
export type NativeFillAlignment = "tl" | "t" | "tr" | "l" | "ctr" | "r" | "bl" | "b" | "br";
export type NativeTileFlip = "none" | "x" | "y" | "xy";
/**
 * Exact native percentage: int32 thousandths of a percent or decimal percent; ranges depend on use.
 */
export type NativePercentage = string;
/**
 * Native coordinate: bounded integer EMU or exact decimal universal measure.
 */
export type NativeCoordinate = string;
export type FillUnresolved =
  | {
      kind: "tableGrid";
      owner: FillOwner;
      reason: NativeTableGridIssue;
    }
  | {
      kind: "tableStyle";
      owner: FillOwner;
      reason: TableStyleSelectionError;
    }
  | {
      kind: "unsupportedTarget";
      owner: FillOwner;
    }
  | {
      kind: "placeholder";
      matching: SourcePlaceholderMatch;
      owner: FillOwner;
    }
  | {
      kind: "missingFormatScheme";
      owner: FillOwner;
    }
  | {
      available: number;
      index: number;
      kind: "styleIndexOutOfRange";
      owner: FillOwner;
    }
  | {
      kind: "retainedContent";
      origin: FillOrigin;
    }
  | {
      kind: "effectEvaluationRequired";
      origin: FillOrigin;
    }
  | {
      kind: "missingImage";
      origin: FillOrigin;
    }
  | {
      kind: "groupWithoutParent";
      origin: FillOrigin;
    }
  | {
      kind: "unsupportedBackgroundMode";
      owner: FillOwner;
    };
export type NativeTableGridIssue =
  | {
      kind: "emptyGrid";
    }
  | {
      actual: number;
      expected: number;
      kind: "rowWidth";
      row: number;
    }
  | {
      cell: SourceCellAddress;
      kind: "duplicateCellId";
    }
  | {
      cell: SourceCellAddress;
      kind: "invalidSpan";
    }
  | {
      cell: SourceCellAddress;
      kind: "missingNeighbour";
    }
  | {
      cell: SourceCellAddress;
      kind: "conflictingNeighbours";
    }
  | {
      cell: SourceCellAddress;
      kind: "outsideMerge";
      origin: SourceCellAddress;
    }
  | {
      cell: SourceCellAddress;
      kind: "conflictingSpan";
      origin: SourceCellAddress;
    }
  | {
      kind: "incompleteMerge";
      origin: SourceCellAddress;
    };
export type TableStyleSelectionError =
  | {
      kind: "conflictingStyles";
    }
  | {
      kind: "invalidIdentity";
    }
  | {
      kind: "missingDefinition";
    }
  | {
      kind: "gridMismatch";
    }
  | {
      kind: "cellOutsideGrid";
    }
  | {
      kind: "retainedDeclaration";
      sourceOrdinal: number;
    }
  | {
      kind: "cancelled";
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
export type ImageReferenceIssue =
  | {
      kind: "missingSelectedReference";
    }
  | {
      kind: "missingDeclaringPart";
    }
  | {
      kind: "missingRelationship";
      ownerPart: string;
      relationshipId: string;
    }
  | {
      kind: "wrongRelationshipType";
      reference: SourceImageReference;
      relationshipType: string;
    }
  | {
      kind: "wrongTargetMode";
      reference: SourceImageReference;
    }
  | {
      kind: "fragment";
      reference: SourceImageReference;
    }
  | {
      contentType: string;
      kind: "nonImageContentType";
      reference: SourceImageReference;
    };
export type PptxFailureCode =
  | "INPUT_INVALID"
  | "SOURCE_CONFLICT"
  | "PRESERVATION_CONFLICT"
  | "MAPPING_NOT_IMPLEMENTED"
  | "RESOURCE_REQUIRED"
  | "LIMIT_EXCEEDED"
  | "CANCELLED"
  | "READ_FAILED";

export interface SourceImageResources {
  bundleByteLength: ByteLength;
  resources: EncodedImageResource[];
  selection: ImageSourceSelection;
  sourceSha256: Digest;
  surface: string;
  targets: SourceImageResult[];
}
export interface EncodedImageResource {
  byteLength: ByteLength;
  contentType: string;
  /**
   * Canonical uint64 byte length. Range requires semantic validation.
   */
  offset: string;
  part: string;
  sha256: Digest;
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
export interface FillOwner {
  part: string;
  target: FillTarget;
}
export interface SourceCellAddress {
  column: number;
  row: number;
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
export interface FillRedirect {
  declaredBy: FillOrigin;
  target: FillOwner;
}
export interface SourceImageReference {
  declaredBy: FillOrigin;
  ownerPart: string;
  relationshipId: string;
  targetUri: string;
}
export interface SourceObjectRef {
  nativeId: number;
  part: string;
}
export interface PptxFailure {
  code: PptxFailureCode;
  message: string;
}
