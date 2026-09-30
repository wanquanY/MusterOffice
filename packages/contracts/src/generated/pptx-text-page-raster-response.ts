/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type PptxTextPageRasterResponse =
  | {
      info: SourceTextPageRasterInfo;
      status: "rendered";
    }
  | {
      error: PptxTextPageFailure;
      status: "error";
    };
/**
 * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
 */
export type FixedQ32 = string;
export type SurfaceKind = "slide" | "master" | "layout";
export type SourcePageProfile = "drawingml-static-solid-page-v1-draft";
export type Digest = string;
/**
 * Canonical uint64 byte length. Range requires semantic validation.
 */
export type ByteLength = string;
export type PptxTextPageFailure =
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
      detail?: PptxTextPageIssue | null;
      error: PptxPageFailure;
      paintLocation?: TextPaintLocation | null;
      stage: "page";
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
  | "font";
export type TextStyleOrigin =
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

export interface SourceTextPageRasterInfo {
  page: SourcePageRasterInfo;
  profile: string;
  /**
   * Missing in historical responses; absence never means measured to fit.
   */
  textCapacity?: TextCapacity | null;
  textFrames: number;
  textWork: FrameWork;
}
export interface SourcePageRasterInfo {
  downstreamCoordinateErrorBound: FixedQ32;
  page: SourcePageInfo;
  scene: SceneRasterInfo;
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
export interface SceneRasterInfo {
  profile: string;
  raster: RasterInfo;
  work: SceneWork;
}
export interface RasterInfo {
  byteLength: ByteLength;
  /**
   * SHA-256 of the little-endian device batch; use together with profile.
   */
  frameSha256: string;
  height: number;
  profile: string;
  sha256: Digest;
  width: number;
  work: RasterWork;
}
export interface RasterWork {
  clips?: ClipWork | null;
  commands: number;
  compositing?: CompositeWork | null;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  coordinateErrorBound: string;
  drawnCommands: number;
  draws: number;
  /**
   * Only present for V12 ellipse fields. Geometry and solver errors are separate.
   */
  ellipticGradients?: EllipticGradientWork | null;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  gradientCoordinateErrorBound: string;
  gradientDraws: number;
  gradientStops: number;
  /**
   * Maximum stop/color conversion or linear/rectangular field error.
   * Elliptic parameter and root bounds are reported separately.
   */
  gradientValueErrorBound: number;
  gradients: number;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  miterLimitErrorBound: string;
  opacityGroups?: OpacityGroupWork | null;
  paths: number;
  strokeDraws: number;
  strokeStyles: number;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  strokeWidthErrorBound: string;
}
export interface ClipWork {
  /**
   * Actual pushes when preserving common ancestors between consecutive draws.
   */
  applications: number;
  appliedCommands: number;
  maximumDepth: number;
  nodes: number;
  /**
   * All supplied clip path commands checked during device placement.
   */
  placementCommands: number;
}
export interface CompositeWork {
  /**
   * Exact packed pixel bytes copied for all snapshots; not process RSS.
   */
  capturedBytes: number;
  captures: number;
  snapshotDraws: number;
  sourceDraws: number;
}
/**
 * Maximum across elliptic fields; excludes pixel coverage and shader inverse arithmetic.
 */
export interface EllipticGradientWork {
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  coordinateErrorBound: string;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  encodedRootIntervalBound: string;
  /**
   * Dimensionless Q32 source uncertainty plus binary32 conversion error,
   * ordered scaleX/Y, centerX/Y, radiusX/Y.
   *
   * @minItems 6
   * @maxItems 6
   */
  parameterErrorBounds: [FixedQ32, FixedQ32, FixedQ32, FixedQ32, FixedQ32, FixedQ32];
}
export interface OpacityGroupWork {
  groups: number;
  maximumDepth: number;
  /**
   * Peak simultaneously live intermediate pixels, excluding output/snapshots.
   */
  peakPixelBytes: number;
  /**
   * Pixels cleared plus pixels composited, including fully transparent groups.
   */
  pixelWork: number;
}
export interface SceneWork {
  clips?: SceneClipWork | null;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  combinedCoordinateErrorBound: string;
  compiledCommands: number;
  compiledPaths: number;
  evaluatedPoints: number;
  /**
   * 2 means the shared matrix candidate failed numeric/precision checks and
   * lowering used the original node chain before any backend call.
   */
  loweringAttempts: number;
  maximumDepth: number;
  /**
   * Conservative point/node work per attempt, including identity instances.
   */
  pointTransformWork: number;
  sourceCommands: number;
  sourcePaths: number;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  transformErrorBound: string;
  transforms: number;
}
export interface SceneClipWork {
  compiledNodes: number;
  sourceNodes: number;
}
export interface TextCapacity {
  /**
   * Complete frame coverage in paint order, including empty text frames.
   */
  frames: FrameCapacity[];
  /**
   * Page-space painted text envelopes, absent from historical measurements.
   * These are before clipping/compositing; intersections are only candidates.
   */
  pageInk?: PageTextInk[] | null;
  profile: string;
}
export interface FrameCapacity {
  cell?: SourceCellAddress | null;
  contentHeight: FixedQ32;
  emergencyLines: number;
  firstHorizontalOverflow?: FrameLine | null;
  /**
   * Counts the actual line search decisions, without rounding to integer EMU.
   */
  horizontalOverflowLines: number;
  /**
   * Unpainted glyph ink, including overhang; not a clipping/overlap verdict.
   */
  inkBounds?: Rect | null;
  inner: Rect1;
  lineCount: number;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  maximumLeftExcess: string;
  maximumRightExcess: FixedQ32;
  object: SourceObjectRef;
  /**
   * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
   */
  verticalExcess: string;
}
export interface SourceCellAddress {
  column: number;
  row: number;
}
export interface FrameLine {
  line: number;
  paragraph: number;
}
export interface Rect {
  max: Point;
  min: Point;
}
export interface Point {
  x: FixedQ32;
  y: FixedQ32;
}
/**
 * Q32 EMU before page/group/animation placement. Insets are already applied.
 */
export interface Rect1 {
  max: Point;
  min: Point;
}
export interface SourceObjectRef {
  nativeId: number;
  part: string;
}
export interface PageTextInk {
  /**
   * Page-space Q32 EMU, including the recorded coordinate uncertainty.
   * None means no nontransparent glyph outline was painted by this frame.
   */
  bounds?: Rect | null;
  cell?: SourceCellAddress | null;
  /**
   * Observations precede the native clip and later layer compositing.
   */
  clippingApplied: boolean;
  coordinateErrorBound: FixedQ32;
  object: SourceObjectRef;
}
export interface FrameWork {
  componentCalls: number;
  fontUploadBytes: number;
  glyphs: number;
  pathCommands: number;
  requestWords: number;
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
export interface TextPaintLocation {
  paragraph: number;
  run: number;
  sourceOrdinal: number;
}
