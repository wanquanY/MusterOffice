/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */
import type { CharacterProperty, ColorUnresolved, EffectiveFillRect, EffectiveFillTile, FillOwner, FillRedirect, FlowIssue, GeometryIssue, ItemizationNotice, ParagraphProperty, PathSceneIssue, PixelExtent, PlacementUnresolved, PptxPageFailure, PptxPlaybackRasterInfo, ShapeFailure, SourceCellAddress, SourceFontSelectionFailure, SourceImageBinding, SourceImageReference, SourceImageResult, SourceObjectRef, SourceVisualIssue, TableBorderTarget, TextBodyProperty, TextCascadeUnresolved, TextDecorationIssue, TextPaintLocation, TextStyleDeclaration, TimelineFailure, TypefaceUnresolved } from './part-002.js';

/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type PptxPlaybackRasterResponse =
  | {
      info: PptxPlaybackRasterInfo;
      status: "rendered";
    }
  | {
      error: PptxPlaybackFailure;
      status: "error";
    };

export type ImageFormat = "png" | "jpeg";

export type Digest = string;

export type ResolutionSource = "pngPhysical" | "jfif" | "exifIfd0";

export type ResolutionUnit = "aspectRatio" | "inch" | "centimetre" | "metre";

export type PhysicalPixelSize =
  | {
      status: "unspecified";
    }
  | {
      status: "zeroDensity";
    }
  | {
      status: "conflicting";
    }
  | {
      status: "known";
      x: PixelExtent;
      y: PixelExtent;
    };

/**
 * Signed int64 EMU. 1 point = 12700 EMU. Range requires semantic validation.
 */
export type Emu = string;

export type SourceColor = "assumedSrgb" | "icc" | "pngColor";

/**
 * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
 */
export type FixedQ32 = string;

export type ChartDataAuthority = "sourceCacheSnapshot";

export type SurfaceKind = "slide" | "master" | "layout";

export type SourcePageProfile = "drawingml-static-solid-page-v1-draft";

/**
 * Canonical uint64 byte length. Range requires semantic validation.
 */
export type ByteLength = string;

/**
 * Canonical uint64 playback generation; never wraps.
 */
export type PlaybackGeneration = string;

export type PlaybackSessionId = string;

export type TimingNodeId = string;

export type NodePhase = "waiting" | "scheduled" | "active" | "frozen" | "finished" | "suppressed";

export type NavigationDirection = "next" | "previous";

export type PresentationStepOutcome =
  | {
      kind: "consumed";
    }
  | {
      entry: PresentationPageEntry;
      kind: "pageBoundary";
    };

export type PresentationPageEntry = "initial";

export type RotationBasis = "absolute" | "layout";

/**
 * Signed int64 ticks. Range requires semantic validation.
 */
export type Ticks = string;

export type Timescale = number;

/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 */
export type Visibility = "visible" | "hidden";

export type PptxPlaybackFailure =
  | {
      error: PptxResourcePageFailure;
      stage: "resource";
    }
  | {
      error: TimelineFailure;
      stage: "timing";
    };

export type PptxResourcePageFailure =
  | {
      error: PptxPageFailure;
      stage: "request";
    }
  | {
      error: PptxPageFailure;
      stage: "source";
    }
  | {
      error: ShapeFailure;
      stage: "fonts";
    }
  | {
      error: PptxPageFailure;
      image?: PptxResourceImageIssue | null;
      paintLocation?: TextPaintLocation | null;
      stage: "page";
      text?: PptxTextPageIssue | null;
    };

export type PptxPageFailureCode =
  | "INPUT_INVALID"
  | "SOURCE_CONFLICT"
  | "PRESERVATION_CONFLICT"
  | "MAPPING_NOT_IMPLEMENTED"
  | "RESOURCE_REQUIRED"
  | "LIMIT_EXCEEDED"
  | "CANCELLED"
  | "READ_FAILED"
  | "COORDINATE_RANGE"
  | "PRECISION_EXCEEDED"
  | "COMPONENT_FAILURE"
  | "COMPONENT_INVALID"
  | "HOST_FAILURE";

export type SourcePageIssue =
  | {
      issue: SourceVisualIssue;
      kind: "visual";
    }
  | {
      kind: "object";
      nativeKind: SourceObjectKind;
    }
  | {
      kind: "text";
      sourceOrdinal: number;
    }
  | {
      kind: "effects";
      sourceOrdinal: number;
    }
  | {
      kind: "placeholder";
      matching: SourcePlaceholderMatch;
    }
  | {
      kind: "specialPlaceholder";
    }
  | {
      kind: "placement";
      reason: PlacementUnresolved;
    }
  | {
      kind: "geometry";
      reason: GeometryUnresolved;
    }
  | {
      kind: "fill";
    }
  | {
      kind: "line";
    }
  | {
      kind: "pathFillModifier";
    }
  | {
      first: TableBorderTarget;
      kind: "tableBorderConflict";
      second: TableBorderTarget;
    }
  | {
      kind: "fillSpace";
      redirects: FillRedirect[];
    };

export type SourceVisualIssueKind = "element" | "attribute";

export type SourceObjectKind = "shape" | "picture" | "group" | "connector" | "graphicFrame";

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

export type PlacementCause =
  | {
      kind: "missingOrigin";
    }
  | {
      kind: "missingSize";
    }
  | {
      kind: "inheritance";
      status: SourcePlaceholderMatch;
    }
  | {
      kind: "retainedTransform";
      sourceOrdinal: number;
    }
  | {
      field: string;
      kind: "invalidCoordinate";
    }
  | {
      kind: "numericRange";
    };

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

export type FormulaIssue = "unknownOperation" | "arity" | "divisionByZero" | "undefinedDirection" | "tangentPole";

export type TableCellEdge = "left" | "right" | "top" | "bottom" | "topLeftToBottomRight" | "bottomLeftToTopRight";

export type FillOrigin =
  | {
      kind: "chart";
      part: string;
      sourceOrdinal: number;
    }
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

export type ShapeFailureCode =
  | "INPUT_INVALID"
  | "FONT_INVALID"
  | "UNSUPPORTED"
  | "RESOURCE_CONFLICT"
  | "RESOURCE_REQUIRED"
  | "LIMIT_EXCEEDED"
  | "CANCELLED"
  | "COMPONENT_FAILURE"
  | "COMPONENT_INVALID"
  | "HOST_FAILURE";

export type FontStyle = "regular" | "bold" | "italic" | "boldItalic";

export type FontSelectionReason = "unmappedTypeface" | "missingStyle";

export type PptxResourceImageIssue =
  | {
      kind: "resource";
      result: SourceImageResult;
    }
  | {
      kind: "stationaryOrientation";
    }
  | {
      kind: "physicalSize";
      resolution: PhysicalPixelSize;
    };

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

export type PptxTextPageIssue =
  | {
      kind: "decorationMetric";
      reason: TextDecorationIssue;
    }
  | {
      failure: SourceFontSelectionFailure;
      kind: "fontSelection";
    }
  | {
      kind: "frame";
      reason: SourceFrameIssue;
    }
  | {
      kind: "paintProperty";
      property: CharacterProperty;
    }
  | {
      declaration: TextStyleDeclaration;
      kind: "paintDeclaration";
    }
  | {
      declaration: TextStyleDeclaration;
      kind: "color";
      reason: ColorUnresolved;
    }
  | {
      kind: "missingPaint";
    }
  | {
      end: number;
      kind: "glyphPaintConflict";
      paragraph: number;
      start: number;
    };

export type FontMetric =
  | "horizontalAscender"
  | "horizontalDescender"
  | "horizontalLineGap"
  | "horizontalClippingAscent"
  | "horizontalClippingDescent"
  | "verticalAscender"
  | "verticalDescender"
  | "verticalLineGap"
  | "horizontalCaretRise"
  | "horizontalCaretRun"
  | "horizontalCaretOffset"
  | "verticalCaretRise"
  | "verticalCaretRun"
  | "verticalCaretOffset"
  | "xHeight"
  | "capHeight"
  | "subscriptXSize"
  | "subscriptYSize"
  | "subscriptXOffset"
  | "subscriptYOffset"
  | "superscriptXSize"
  | "superscriptYSize"
  | "superscriptXOffset"
  | "superscriptYOffset"
  | "strikeoutSize"
  | "strikeoutOffset"
  | "underlineSize"
  | "underlineOffset";

export type NativeTextElement =
  | (
      | "txBody"
      | "txStyles"
      | "defaultTextStyle"
      | "titleStyle"
      | "bodyStyle"
      | "otherStyle"
      | "lstStyle"
      | "bodyPr"
      | "p"
      | "pPr"
      | "defPPr"
      | "lvl1pPr"
      | "lvl2pPr"
      | "lvl3pPr"
      | "lvl4pPr"
      | "lvl5pPr"
      | "lvl6pPr"
      | "lvl7pPr"
      | "lvl8pPr"
      | "lvl9pPr"
      | "r"
      | "br"
      | "fld"
      | "t"
      | "rPr"
      | "defRPr"
      | "endParaRPr"
      | "noAutofit"
      | "normAutofit"
      | "spAutoFit"
      | "lnSpc"
      | "spcBef"
      | "spcAft"
      | "spcPct"
      | "spcPts"
      | "buClrTx"
      | "buClr"
      | "buSzTx"
      | "buSzPct"
      | "buSzPts"
      | "buFontTx"
      | "buFont"
      | "buNone"
      | "buAutoNum"
      | "buChar"
      | "tabLst"
      | "tab"
      | "latin"
      | "ea"
      | "cs"
      | "sym"
      | "fontRef"
      | "highlight"
      | "uLnTx"
      | "uLn"
      | "uFillTx"
      | "uFill"
      | "hlinkClick"
      | "hlinkMouseOver"
      | "rtl"
      | "noFill"
      | "solidFill"
      | "gradFill"
      | "blipFill"
      | "pattFill"
      | "grpFill"
      | "ln"
      | "effectLst"
      | "effectDag"
      | "srgbClr"
      | "scrgbClr"
      | "hslClr"
      | "sysClr"
      | "schemeClr"
      | "prstClr"
    )
  | "txPr"
  | "rich"
  | "font";

export type TextStyleOrigin =
  | {
      bodySourceOrdinal: number;
      kind: "chart";
      part: string;
      sourceOrdinal: number;
    }
  | {
      kind: "tableStyle";
      region: TableStyleRegion;
      source: TableTextStyleSource;
      sourceOrdinal: number;
    }
  | {
      kind: "object";
      object: SourceObjectRef;
      sourceOrdinal: number;
    }
  | {
      kind: "master";
      part: string;
      sourceOrdinal: number;
    }
  | {
      kind: "presentation";
      part: string;
      sourceOrdinal: number;
    }
  | {
      defaultKind: SourceThemeDefaultKind;
      kind: "theme";
      part: string;
      sourceOrdinal: number;
    }
  | {
      kind: "profileDefault";
    };

export type TableTextStyleSource =
  | {
      kind: "inline";
      object: SourceObjectRef;
    }
  | {
      kind: "catalog";
      part: string;
      styleId: string;
    };

export type SourceThemeDefaultKind = "txDef" | "lnDef" | "spDef";

export type NativeFontSlot = "latin" | "eastAsian" | "complexScript" | "symbol";

export type NativeFontCollectionIndex = "major" | "minor" | "none";

export type SourceFrameIssue =
  | {
      cell: SourceCellAddress;
      kind: "coveredCell";
      origin: SourceCellAddress;
    }
  | {
      kind: "body";
      reason: TextBodyUnresolved;
    }
  | {
      kind: "text";
      reason: SourceTextIssue;
    }
  | {
      kind: "geometry";
      reason: GeometryUnresolved;
    }
  | {
      kind: "bodyProperty";
      property: TextBodyProperty;
    }
  | {
      kind: "autofit";
    }
  | {
      kind: "paragraphProperty";
      paragraph: number;
      property: ParagraphProperty;
    }
  | {
      declaration: TextStyleDeclaration;
      kind: "paragraphDeclaration";
      paragraph: number;
    }
  | {
      kind: "invalidRegion";
    }
  | {
      flow: FlowIssue[];
      geometry: GeometryIssue[];
      kind: "incompleteParagraph";
      paragraph: number;
      paths: PathSceneIssue[];
    };

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
