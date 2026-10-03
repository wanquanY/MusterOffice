/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type EditorPageResponse =
  | {
      info: EditorPageInfo;
      status: "prepared";
      view: Digest;
    }
  | {
      results: PageTextQueryResult[];
      status: "queried";
      view: Digest;
    }
  | {
      results: EditorPickResult[];
      status: "picked";
      view: Digest;
    }
  | {
      status: "cleared";
      view: Digest;
    }
  | {
      error: PptxResourcePageFailure;
      status: "error";
    };
/**
 * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
 */
export type FixedQ32 = string;
export type SourceObjectKind = "shape" | "picture" | "group" | "connector" | "graphicFrame";
export type ObjectId = string;
export type SurfaceKind = "slide" | "master" | "layout";
export type SourcePageProfile = "drawingml-static-solid-page-v1-draft";
export type Digest = string;
export type CellId = string;
export type ParagraphId = string;
export type RunId = string;
export type PageTextQueryResult =
  | {
      caret: PageCaret;
      exhausted: boolean;
      frame: number;
      kind: "moved";
      preferredX?: FixedQ32 | null;
    }
  | {
      caret: PageCaret;
      frame: number;
      kind: "caret";
    }
  | {
      caret?: PageCaret | null;
      frame: number;
      inside: boolean;
      kind: "hit";
    }
  | {
      anchor: PageCaret;
      focus: PageCaret;
      fragments: PageSelectionFragment[];
      frame: number;
      kind: "selection";
      paragraphBreaks: number[];
    };
export type Affinity = "upstream" | "downstream";
export type TextItemKind = "text" | "tab" | "lineBreak" | "paragraphBreak" | "bidiControl";
export type DrawHitKind = "exact" | "nearby";
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

export interface EditorPageInfo {
  downstreamCoordinateErrorBound: FixedQ32;
  /**
   * Page paint targets and their group ancestors, in source paint preorder.
   * Hit.object and parent refer to indices in this immutable view's list.
   */
  objects: EditorPageObjectInfo[];
  page: SourcePageInfo;
  resourcesSha256: Digest;
  textFrames: EditorTextFrameInfo[];
  textWork: FrameWork;
  viewport: RasterViewport;
}
export interface EditorPageObjectInfo {
  kind: SourceObjectKind;
  name: string;
  object: SourceObjectRef;
  objectId?: ObjectId | null;
  /**
   * None is a top-level object; the source shape-tree root is not a user group.
   */
  parent?: number | null;
  surface: SurfaceKind;
}
export interface SourceObjectRef {
  nativeId: number;
  part: string;
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
export interface EditorTextFrameInfo {
  cell?: SourceCellAddress | null;
  /**
   * Authored table cell identity. Retained cells use their paragraph/run IDs.
   */
  cellId?: CellId | null;
  frame: number;
  object: SourceObjectRef;
  /**
   * Stable model identity for author and retained document inputs.
   */
  objectId?: ObjectId | null;
  paragraphs: EditorParagraphInfo[];
}
export interface SourceCellAddress {
  column: number;
  row: number;
}
export interface EditorParagraphInfo {
  boundaries: TextBoundary[];
  /**
   * Actual model identities, never inferred from XML discovery ordinals.
   * None for raw PPTX, legacy opaque text or an authored empty cell body.
   */
  model?: EditorParagraphIdentity | null;
  sourceOrdinal: number;
  text: string;
}
export interface TextBoundary {
  scalarOffset: number;
  utf16Offset: number;
  utf8Offset: number;
}
export interface EditorParagraphIdentity {
  id: ParagraphId;
  runs: EditorTextRunIdentity[];
}
export interface EditorTextRunIdentity {
  id: RunId;
  scalarEnd: number;
  /**
   * Half-open offsets in the same displayed paragraph, in Unicode scalars.
   * These are identities, not editing permission or grapheme boundaries.
   */
  scalarStart: number;
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
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
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
export interface PageCaret {
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  coordinateErrorBound: string;
  /**
   * @minItems 2
   * @maxItems 2
   */
  edge: [Point1, Point1];
  local: FrameCaret;
  /**
   * @minItems 2
   * @maxItems 2
   */
  visible?: [Point1, Point1] | null;
}
export interface Point1 {
  x: FixedQ32;
  y: FixedQ32;
}
export interface FrameCaret {
  caret: ResolvedCaret;
  paragraph: number;
  visible?: CaretEdge | null;
}
/**
 * Includes the actual frame line translation (insets, paragraph alignment,
 * native indentation, vertical anchor and paragraph spacing).
 */
export interface ResolvedCaret {
  boundary: TextBoundary;
  edge: CaretEdge;
  line: number;
  position: TextPosition;
}
export interface CaretEdge {
  bottom: FixedQ32;
  top: FixedQ32;
  x: FixedQ32;
}
export interface TextPosition {
  affinity: Affinity;
  scalarOffset: number;
}
export interface PageSelectionFragment {
  coordinateErrorBound: FixedQ32;
  local: FrameSelectionFragment;
  /**
   * Four actual corners, not an axis-aligned envelope of rotated text.
   *
   * @minItems 4
   * @maxItems 4
   */
  quad: [Point1, Point1, Point1, Point1];
  /**
   * @minItems 4
   * @maxItems 4
   */
  visible?: [Point1, Point1, Point1, Point1] | null;
}
export interface FrameSelectionFragment {
  fragment: SelectionFragment;
  paragraph: number;
  /**
   * Native overflow clips the overlay along active local axes only.
   */
  visible?: Rect | null;
}
export interface SelectionFragment {
  bounds: Rect;
  end: TextBoundary;
  kind: TextItemKind;
  line: number;
  start: TextBoundary;
}
export interface Rect {
  max: Point1;
  min: Point1;
}
export interface EditorPickResult {
  hits: EditorObjectHit[];
  truncated: boolean;
}
export interface EditorObjectHit {
  kind: DrawHitKind;
  object: number;
  textFrame?: number | null;
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
/**
 * Reduced, positive rational EMU per normalized output pixel.
 */
export interface PixelExtent {
  denominator: number;
  numerator: Emu;
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
export interface EffectiveVariation {
  /**
   * Effective IEEE754 binary32 design coordinate. Rounding is explicit.
   */
  effectiveF32Bits: number;
  requested1616: number;
  tag: string;
}
