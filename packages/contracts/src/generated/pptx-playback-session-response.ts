/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type PptxPlaybackSessionResponse =
  | {
      info: PptxPlaybackSessionInfo;
      status: "prepared";
    }
  | {
      info: PptxPlaybackSessionInfo;
      status: "inspected";
    }
  | {
      info: PlaybackTimingInfo;
      status: "timingInspected";
    }
  | {
      info: PptxPlaybackSessionInfo;
      status: "advanced";
    }
  | {
      binding: PlaybackBinding;
      status: "disposed";
    }
  | {
      info: PptxPlaybackRasterInfo;
      status: "rendered";
    }
  | {
      error: PptxPlaybackSessionFailure;
      status: "error";
    };
/**
 * Canonical uint64 playback generation; never wraps.
 */
export type PlaybackGeneration = string;
export type Digest = string;
export type PlaybackSessionId = string;
/**
 * Canonical uint64 timeline work count; never wraps.
 */
export type TimelineWorkCount = string;
export type ImageFormat = "png" | "jpeg";
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
export type SurfaceKind = "slide" | "master" | "layout";
export type SourcePageProfile = "drawingml-static-solid-page-v1-draft";
/**
 * Canonical uint64 byte length. Range requires semantic validation.
 */
export type ByteLength = string;
export type TimingNodeId = string;
export type NodePhase = "waiting" | "scheduled" | "active" | "frozen" | "finished" | "suppressed";
/**
 * Signed int64 ticks. Range requires semantic validation.
 */
export type Ticks = string;
export type Timescale = number;
export type PptxPlaybackSessionFailure =
  | {
      code: PlaybackSessionFailureCode;
      kind: "session";
      message: string;
    }
  | {
      error: PptxPlaybackFailure;
      kind: "computation";
    };
export type PlaybackSessionFailureCode =
  | "INPUT_INVALID"
  | "LIMIT_EXCEEDED"
  | "CANCELLED"
  | "NOT_PREPARED"
  | "ALREADY_PREPARED"
  | "DISPOSED"
  | "BINDING_CONFLICT"
  | "GENERATION_NOT_INCREASING"
  | "RASTER_REQUIRED";
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
export type FillOrigin =
  | {
      kind: "declaration";
      owner: FillOwner;
      sourceOrdinal: number;
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
      kind: "rootGroup";
    }
  | {
      kind: "background";
    };
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
  | "prstClr";
export type TextStyleOrigin =
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
export type SourceThemeDefaultKind = "txDef" | "lnDef" | "spDef";
export type NativeFontCollectionIndex = "major" | "minor" | "none";
export type NativeFontSlot = "latin" | "eastAsian" | "complexScript" | "symbol";
export type SourceFrameIssue =
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
export type TimelineFailureCode =
  | "INPUT_INVALID"
  | "REVISION_CONFLICT"
  | "EVENT_HISTORY_REQUIRED"
  | "EVENT_HISTORY_INVALID"
  | "LIMIT_EXCEEDED"
  | "CANCELLED";

export interface PptxPlaybackSessionInfo {
  binding: PlaybackBinding;
  /**
   * In-process implementation/content identity, not host authorization.
   */
  planId: string;
  preparation: ResourcePreparationInfo;
  profile: string;
  slide: string;
  sourceSha256: Digest;
}
export interface PlaybackBinding {
  generation: PlaybackGeneration;
  revision: Digest;
  session: PlaybackSessionId;
}
export interface ResourcePreparationInfo {
  decodedImages: number;
  decodedPixelBytes: number;
  encodedBytes: number;
  gatherCopyBytes: number;
  resourcesSha256: Digest;
  textFrames: number;
  /**
   * Logical paths/draw elements; not heap capacity or process RSS.
   */
  textPathBytes: number;
  textWork: FrameWork;
}
export interface FrameWork {
  componentCalls: number;
  fontUploadBytes: number;
  glyphs: number;
  pathCommands: number;
  requestWords: number;
}
/**
 * Read-only diagnostics. Counts successful timing evaluations, including those
 * followed by a page/raster failure; does not count published or displayed frames.
 */
export interface PlaybackTimingInfo {
  binding: PlaybackBinding;
  sampler: TimelineSamplerInfo;
}
export interface TimelineSamplerInfo {
  cachedBinding?: PlaybackBinding | null;
  retainedEvents: TimelineWorkCount;
  retainedIntervals: TimelineWorkCount;
  /**
   * Canonical uint64 timeline work count; never wraps.
   */
  schedulesBuilt: string;
  schedulesReused: TimelineWorkCount;
  timelineSha256: Digest;
}
export interface PptxPlaybackRasterInfo {
  page: SourceResourcePageRasterInfo;
  playback: SourcePlaybackFrame;
  profile: string;
}
export interface SourceResourcePageRasterInfo {
  decodedImages: DecodedImageInfo[];
  encodedBytes: number;
  gatherCopyBytes: number;
  images: ImageWork;
  page: SourcePageRasterInfo;
  profile: string;
  resourcesSha256: Digest;
  textFrames: number;
  textWork: FrameWork;
}
export interface DecodedImageInfo {
  byteLength: number;
  encodedBitDepth: number;
  encodedHeight: number;
  encodedWidth: number;
  format: ImageFormat;
  height: number;
  orientation: number;
  pixelsSha256: Digest;
  profile: string;
  resolution: ImageResolution;
  sourceColor: SourceColor;
  sourceSha256: Digest;
  width: number;
}
export interface ImageResolution {
  /**
   * Raw fields in encoded axes. Missing EXIF fields remain null; derivation
   * uses Exif's specified defaults (72, 72, inch), never a host DPI default.
   */
  declarations: ResolutionDeclaration[];
  physicalPixelSize: PhysicalPixelSize;
}
export interface ResolutionDeclaration {
  source: ResolutionSource;
  /**
   * Absolute byte offset of the PNG chunk or JPEG marker in source bytes.
   */
  sourceOffset: number;
  /**
   * None preserves an absent EXIF ResolutionUnit. Derivation applies the
   * Exif specification's inch default without rewriting this declaration.
   */
  unit?: ResolutionUnit | null;
  x?: Density | null;
  y?: Density | null;
}
export interface Density {
  denominator: number;
  numerator: number;
}
/**
 * Reduced, positive rational EMU per normalized output pixel.
 */
export interface PixelExtent {
  denominator: number;
  numerator: Emu;
}
export interface ImageWork {
  brushes: number;
  /**
   * Bound for upstream uncertainty and affine/domain quantization, in Q32
   * device pixels. Domain repetition is bounded over the viewport. Excludes
   * inverse/shader arithmetic, filter output changes and sample coverage.
   */
  coordinateErrorBound: string;
  draws: number;
  resourceBytes: number;
  resources: number;
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
export interface SourcePlaybackFrame {
  evaluated: EvaluatedFrame;
  /**
   * Synthetic time-graph keys are scoped to this source slide. Never resolve
   * a bare native id against a master, layout or another slide.
   */
  objectBindings: {
    [k: string]: SourceObjectRef | undefined;
  };
  partSha256: Digest;
  slide: string;
  sourceSha256: Digest;
}
export interface EvaluatedFrame {
  sha256: Digest;
  state: FrameState;
}
export interface FrameState {
  binding: PlaybackBinding;
  containers?: NodeFrame[];
  eventCursor: number;
  nodes: NodeFrame[];
  profile: string;
  rotations: {
    [k: string]: ExactValue | undefined;
  };
  time: RationalTime;
  timelineSha256: Digest;
}
export interface NodeFrame {
  end?: ExactValue | null;
  iteration?: string | null;
  node: TimingNodeId;
  phase: NodePhase;
  progress?: ExactValue | null;
  start?: ExactValue | null;
}
/**
 * Output-only exact reduced ratio. Rotation units remain 1/60000 degree.
 *
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 */
export interface ExactValue {
  denominator: string;
  numerator: string;
}
/**
 * Exact wire representation. Equality compares author values; compare_time compares instants.
 */
export interface RationalTime {
  ticks: Ticks;
  timescale: Timescale;
}
/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 */
export interface SourceObjectRef {
  nativeId: number;
  part: string;
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
   * Original run font declaration; absent for a shape fontRef fallback.
   */
  authoredFont?: SourceTextFont | null;
  declaredBy: TextStyleDeclaration;
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
export interface TimelineFailure {
  code: TimelineFailureCode;
  message: string;
}
