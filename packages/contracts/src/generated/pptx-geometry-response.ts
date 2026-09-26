/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type PptxGeometryResponse =
  | {
      geometry: SourceGeometryValues;
      status: "evaluated";
    }
  | {
      error: PptxFailure;
      status: "error";
    };
export type GeometryOutcome =
  | {
      geometry: EvaluatedGeometry;
      status: "resolved";
    }
  | {
      reason: GeometryUnresolved;
      status: "unresolved";
    };
export type GuideDependency =
  | {
      kind: "builtin";
      name: string;
    }
  | {
      kind: "guide";
      origin: GeometryOrigin;
    };
/**
 * Ordinals are zero-based element preorder in the indicated XML resource.
 * Preset ordinals address the pinned, generated catalog definition, not the
 * document part. A document override always retains its physical source.
 */
export type GeometryOrigin =
  | {
      kind: "document";
      sourceOrdinal: number;
    }
  | {
      definitionOrdinal: number;
      kind: "preset";
      preset: NativeShapeType;
    };
export type NativeShapeType =
  | "accentBorderCallout1"
  | "accentBorderCallout2"
  | "accentBorderCallout3"
  | "accentCallout1"
  | "accentCallout2"
  | "accentCallout3"
  | "actionButtonBackPrevious"
  | "actionButtonBeginning"
  | "actionButtonBlank"
  | "actionButtonDocument"
  | "actionButtonEnd"
  | "actionButtonForwardNext"
  | "actionButtonHelp"
  | "actionButtonHome"
  | "actionButtonInformation"
  | "actionButtonMovie"
  | "actionButtonReturn"
  | "actionButtonSound"
  | "arc"
  | "bentArrow"
  | "bentConnector2"
  | "bentConnector3"
  | "bentConnector4"
  | "bentConnector5"
  | "bentUpArrow"
  | "bevel"
  | "blockArc"
  | "borderCallout1"
  | "borderCallout2"
  | "borderCallout3"
  | "bracePair"
  | "bracketPair"
  | "callout1"
  | "callout2"
  | "callout3"
  | "can"
  | "chartPlus"
  | "chartStar"
  | "chartX"
  | "chevron"
  | "chord"
  | "circularArrow"
  | "cloud"
  | "cloudCallout"
  | "corner"
  | "cornerTabs"
  | "cube"
  | "curvedConnector2"
  | "curvedConnector3"
  | "curvedConnector4"
  | "curvedConnector5"
  | "curvedDownArrow"
  | "curvedLeftArrow"
  | "curvedRightArrow"
  | "curvedUpArrow"
  | "decagon"
  | "diagStripe"
  | "diamond"
  | "dodecagon"
  | "donut"
  | "doubleWave"
  | "downArrow"
  | "downArrowCallout"
  | "ellipse"
  | "ellipseRibbon"
  | "ellipseRibbon2"
  | "flowChartAlternateProcess"
  | "flowChartCollate"
  | "flowChartConnector"
  | "flowChartDecision"
  | "flowChartDelay"
  | "flowChartDisplay"
  | "flowChartDocument"
  | "flowChartExtract"
  | "flowChartInputOutput"
  | "flowChartInternalStorage"
  | "flowChartMagneticDisk"
  | "flowChartMagneticDrum"
  | "flowChartMagneticTape"
  | "flowChartManualInput"
  | "flowChartManualOperation"
  | "flowChartMerge"
  | "flowChartMultidocument"
  | "flowChartOfflineStorage"
  | "flowChartOffpageConnector"
  | "flowChartOnlineStorage"
  | "flowChartOr"
  | "flowChartPredefinedProcess"
  | "flowChartPreparation"
  | "flowChartProcess"
  | "flowChartPunchedCard"
  | "flowChartPunchedTape"
  | "flowChartSort"
  | "flowChartSummingJunction"
  | "flowChartTerminator"
  | "foldedCorner"
  | "frame"
  | "funnel"
  | "gear6"
  | "gear9"
  | "halfFrame"
  | "heart"
  | "heptagon"
  | "hexagon"
  | "homePlate"
  | "horizontalScroll"
  | "irregularSeal1"
  | "irregularSeal2"
  | "leftArrow"
  | "leftArrowCallout"
  | "leftBrace"
  | "leftBracket"
  | "leftCircularArrow"
  | "leftRightArrow"
  | "leftRightArrowCallout"
  | "leftRightCircularArrow"
  | "leftRightRibbon"
  | "leftRightUpArrow"
  | "leftUpArrow"
  | "lightningBolt"
  | "line"
  | "lineInv"
  | "mathDivide"
  | "mathEqual"
  | "mathMinus"
  | "mathMultiply"
  | "mathNotEqual"
  | "mathPlus"
  | "moon"
  | "noSmoking"
  | "nonIsoscelesTrapezoid"
  | "notchedRightArrow"
  | "octagon"
  | "parallelogram"
  | "pentagon"
  | "pie"
  | "pieWedge"
  | "plaque"
  | "plaqueTabs"
  | "plus"
  | "quadArrow"
  | "quadArrowCallout"
  | "rect"
  | "ribbon"
  | "ribbon2"
  | "rightArrow"
  | "rightArrowCallout"
  | "rightBrace"
  | "rightBracket"
  | "round1Rect"
  | "round2DiagRect"
  | "round2SameRect"
  | "roundRect"
  | "rtTriangle"
  | "smileyFace"
  | "snip1Rect"
  | "snip2DiagRect"
  | "snip2SameRect"
  | "snipRoundRect"
  | "squareTabs"
  | "star10"
  | "star12"
  | "star16"
  | "star24"
  | "star32"
  | "star4"
  | "star5"
  | "star6"
  | "star7"
  | "star8"
  | "straightConnector1"
  | "stripedRightArrow"
  | "sun"
  | "swooshArrow"
  | "teardrop"
  | "trapezoid"
  | "triangle"
  | "upArrow"
  | "upArrowCallout"
  | "upDownArrow"
  | "upDownArrowCallout"
  | "uturnArrow"
  | "verticalScroll"
  | "wave"
  | "wedgeEllipseCallout"
  | "wedgeRectCallout"
  | "wedgeRoundRectCallout";
/**
 * Signed int64 EMU. 1 point = 12700 EMU. Range requires semantic validation.
 */
export type Emu = string;
export type EvaluatedHandle =
  | {
      guideX?: GeometryOrigin | null;
      guideY?: GeometryOrigin | null;
      kind: "xy";
      maxX?: number | null;
      maxY?: number | null;
      minX?: number | null;
      minY?: number | null;
      origin: GeometryOrigin;
      position: EvaluatedPoint;
    }
  | {
      guideAngle?: GeometryOrigin | null;
      guideRadius?: GeometryOrigin | null;
      kind: "polar";
      maxAngle?: number | null;
      maxRadius?: number | null;
      minAngle?: number | null;
      minRadius?: number | null;
      origin: GeometryOrigin;
      position: EvaluatedPoint;
    };
export type EvaluatedCommand =
  | {
      kind: "move";
      origin: GeometryOrigin;
      to: EvaluatedPoint;
    }
  | {
      kind: "line";
      origin: GeometryOrigin;
      to: EvaluatedPoint;
    }
  | {
      heightRadius: number;
      kind: "arc";
      origin: GeometryOrigin;
      startAngle: number;
      sweepAngle: number;
      widthRadius: number;
    }
  | {
      control: EvaluatedPoint;
      kind: "quadratic";
      origin: GeometryOrigin;
      to: EvaluatedPoint;
    }
  | {
      control1: EvaluatedPoint;
      control2: EvaluatedPoint;
      kind: "cubic";
      origin: GeometryOrigin;
      to: EvaluatedPoint;
    }
  | {
      kind: "close";
      origin: GeometryOrigin;
    };
export type NativePathFill = "none" | "norm" | "lighten" | "lightenLess" | "darken" | "darkenLess";
export type GeometryUnresolved =
  | {
      kind: "missingDeclaration";
    }
  | {
      kind: "unsupportedObject";
    }
  | {
      kind: "missingExtent";
    }
  | {
      kind: "invalidExtent";
    }
  | {
      kind: "retainedContent";
      origin: GeometryOrigin;
    }
  | {
      issue: FormulaIssue;
      kind: "formula";
      origin: GeometryOrigin;
    }
  | {
      kind: "invalidCoordinate";
      origin: GeometryOrigin;
    }
  | {
      kind: "invalidAngle";
      origin: GeometryOrigin;
    }
  | {
      kind: "unknownReference";
      origin: GeometryOrigin;
      token: string;
    }
  | {
      kind: "reservedGuide";
      origin: GeometryOrigin;
    }
  | {
      kind: "invalidGuideName";
      origin: GeometryOrigin;
    }
  | {
      kind: "invalidHandleReference";
      origin: GeometryOrigin;
    }
  | {
      kind: "numericRange";
      origin: GeometryOrigin;
    };
export type FormulaIssue = "unknownOperation" | "arity" | "divisionByZero" | "undefinedDirection" | "tangentPole";
export type GeometryProfile = "ecma376-2016-ms-presets-draft-v2";
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

export interface SourceGeometryValues {
  objects: GeometryResult[];
  profile: GeometryProfile;
  sourceSha256: Digest;
  surface: string;
}
export interface GeometryResult {
  nativeId: number;
  outcome: GeometryOutcome;
}
export interface EvaluatedGeometry {
  /**
   * Source order is preserved. Repeated names bind to the last prior value.
   */
  adjustments: GuideValue[];
  connections: EvaluatedConnection[];
  extent: SourceResolvedValue;
  guides: GuideValue[];
  handles: EvaluatedHandle[];
  paths: EvaluatedPath[];
  sourceOrdinal: number;
  /**
   * Absence is not replaced with an invented native text rectangle.
   */
  textRect?: EvaluatedRect | null;
}
export interface GuideValue {
  dependencies: GuideDependency[];
  name: string;
  origin: GeometryOrigin;
  value: number;
}
export interface EvaluatedConnection {
  angle: number;
  origin: GeometryOrigin;
  position: EvaluatedPoint;
}
export interface EvaluatedPoint {
  origin: GeometryOrigin;
  x: number;
  y: number;
}
export interface SourceResolvedValue {
  declaredBy: SourceObjectRef;
  value: Size;
}
/**
 * Ultimate explicit declaration, not merely the next inheritance hop.
 */
export interface SourceObjectRef {
  nativeId: number;
  part: string;
}
export interface Size {
  height: Emu;
  width: Emu;
}
export interface EvaluatedPath {
  commands: EvaluatedCommand[];
  extrusionOk?: boolean | null;
  fill?: NativePathFill | null;
  height?: Emu | null;
  origin: GeometryOrigin;
  stroke?: boolean | null;
  /**
   * Native path-space extents. Scaling into shape space is a later step.
   */
  width?: Emu | null;
}
export interface EvaluatedRect {
  bottom: number;
  left: number;
  origin: GeometryOrigin;
  right: number;
  top: number;
}
export interface PptxFailure {
  code: PptxFailureCode;
  message: string;
}
