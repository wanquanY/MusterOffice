/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type ParagraphInteractionResponse =
  | {
      result: ParagraphInteractionResult;
      status: "evaluated";
    }
  | {
      error: ShapeFailure;
      status: "error";
    };
export type BreakKind = "allowed" | "mandatory";
/**
 * Signed i128 integer divided by 2^32. Coordinate unit is specified by the owning geometry profile; semantic range validation required.
 */
export type FixedQ32 = string;
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
/**
 * Signed int64 EMU. 1 point = 12700 EMU. Range requires semantic validation.
 */
export type Emu = string;
export type FontFragment =
  | {
      candidate: number;
      end: number;
      font: number;
      shaped: ShapedText;
      start: number;
      status: "selected";
    }
  | {
      end: number;
      start: number;
      status: "unresolved";
    };
export type Digest = string;
export type Direction = "leftToRight" | "rightToLeft" | "topToBottom" | "bottomToTop";
export type CoverageOutcome =
  | {
      glyphId: number;
      kind: "mapped";
    }
  | {
      kind: "missing";
    }
  | {
      kind: "unsupportedVariation";
    }
  | {
      kind: "noUnicodeCmap";
    };
export type TextItemKind = "text" | "tab" | "lineBreak" | "paragraphBreak" | "bidiControl";
export type Script =
  | "Adlm"
  | "Aghb"
  | "Ahom"
  | "Arab"
  | "Armi"
  | "Armn"
  | "Avst"
  | "Bali"
  | "Bamu"
  | "Bass"
  | "Batk"
  | "Beng"
  | "Berf"
  | "Bhks"
  | "Bopo"
  | "Brah"
  | "Brai"
  | "Bugi"
  | "Buhd"
  | "Cakm"
  | "Cans"
  | "Cari"
  | "Cham"
  | "Cher"
  | "Chrs"
  | "Copt"
  | "Cpmn"
  | "Cprt"
  | "Cyrl"
  | "Deva"
  | "Diak"
  | "Dogr"
  | "Dsrt"
  | "Dupl"
  | "Egyp"
  | "Elba"
  | "Elym"
  | "Ethi"
  | "Gara"
  | "Geor"
  | "Glag"
  | "Gong"
  | "Gonm"
  | "Goth"
  | "Gran"
  | "Grek"
  | "Gujr"
  | "Gukh"
  | "Guru"
  | "Hang"
  | "Hani"
  | "Hano"
  | "Hatr"
  | "Hebr"
  | "Hira"
  | "Hluw"
  | "Hmng"
  | "Hmnp"
  | "Hrkt"
  | "Hung"
  | "Ital"
  | "Java"
  | "Jurc"
  | "Kali"
  | "Kana"
  | "Kawi"
  | "Khar"
  | "Khmr"
  | "Khoj"
  | "Kits"
  | "Knda"
  | "Krai"
  | "Kthi"
  | "Lana"
  | "Laoo"
  | "Latn"
  | "Lepc"
  | "Limb"
  | "Lina"
  | "Linb"
  | "Lisu"
  | "Lyci"
  | "Lydi"
  | "Mahj"
  | "Maka"
  | "Mand"
  | "Mani"
  | "Marc"
  | "Medf"
  | "Mend"
  | "Merc"
  | "Mero"
  | "Mlym"
  | "Modi"
  | "Mong"
  | "Mroo"
  | "Mtei"
  | "Mult"
  | "Mymr"
  | "Nagm"
  | "Nand"
  | "Narb"
  | "Nbat"
  | "Newa"
  | "Nkoo"
  | "Nshu"
  | "Ogam"
  | "Olck"
  | "Onao"
  | "Orkh"
  | "Orya"
  | "Osge"
  | "Osma"
  | "Ougr"
  | "Palm"
  | "Pauc"
  | "Pcun"
  | "Perm"
  | "Phag"
  | "Phli"
  | "Phlp"
  | "Phnx"
  | "Plrd"
  | "Prti"
  | "Rjng"
  | "Rohg"
  | "Runr"
  | "Samr"
  | "Sarb"
  | "Saur"
  | "Seal"
  | "Sgnw"
  | "Shaw"
  | "Shrd"
  | "Sidd"
  | "Sidt"
  | "Sind"
  | "Sinh"
  | "Sogd"
  | "Sogo"
  | "Sora"
  | "Soyo"
  | "Sund"
  | "Sunu"
  | "Sylo"
  | "Syrc"
  | "Tagb"
  | "Takr"
  | "Tale"
  | "Talu"
  | "Taml"
  | "Tang"
  | "Tavt"
  | "Tayo"
  | "Telu"
  | "Tfng"
  | "Tglg"
  | "Thaa"
  | "Thai"
  | "Tibt"
  | "Tirh"
  | "Tnsa"
  | "Todr"
  | "Tols"
  | "Toto"
  | "Tutg"
  | "Ugar"
  | "Vaii"
  | "Vith"
  | "Wara"
  | "Wcho"
  | "Xpeo"
  | "Xsux"
  | "Yezi"
  | "Yiii"
  | "Zanb"
  | "Zinh"
  | "Zyyy"
  | "Zzzz";
export type ItemizationNoticeKind = "mixedScriptCluster" | "mixedLevelCluster" | "ambiguousScript";
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
export type CaretPlacement =
  | {
      kind: "glyphEdges";
    }
  | {
      kind: "fontLigature";
    }
  | {
      kind: "clusterPartition";
      reason: PartitionReason;
    }
  | {
      kind: "invisible";
    };
export type PartitionReason =
  "fontCaretsAbsent" | "ambiguousGlyphs" | "fontCaretCountMismatch" | "nonMonotoneFontCarets";
export type TextQueryResult =
  | {
      caret: ResolvedCaret;
      kind: "caret";
    }
  | {
      caret: ResolvedCaret;
      inside: boolean;
      kind: "hit";
    }
  | {
      kind: "moved";
      result: CaretNavigation;
    }
  | {
      anchor: ResolvedCaret;
      focus: ResolvedCaret;
      fragments: SelectionFragment[];
      kind: "selection";
    };
export type Affinity = "upstream" | "downstream";
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

export interface ParagraphInteractionResult {
  layout: ParagraphLayoutResult;
  /**
   * Absent when existing layout prerequisites are unresolved; no partial map.
   */
  map?: InteractionMap | null;
  results: TextQueryResult[];
}
export interface ParagraphLayoutResult {
  breaks: LineBreakAnalysis;
  /**
   * Empty if prerequisites prevent a complete plan. A terminal explicit
   * line break has one final empty line at the same source end coordinate.
   */
  decisions: LineDecision[];
  geometry?: LineGeometryResult | null;
  issues: FlowIssue[];
  profile: string;
  suppressedGraphemeBreaks: TextBoundary[];
  work: FlowWork;
}
export interface LineBreakAnalysis {
  /**
   * Original SA positions. Default LB1 maps these to AL/CM; dictionary
   * segmentation and target-application tailoring are separate responsibilities.
   */
  complexContextScalars: number[];
  end: TextBoundary;
  /**
   * No start-of-text opportunity; empty text has none (LB2).
   */
  opportunities: LineBreakOpportunity[];
  profile: string;
}
export interface TextBoundary {
  scalarOffset: number;
  utf16Offset: number;
  utf8Offset: number;
}
export interface LineBreakOpportunity {
  boundary: TextBoundary;
  kind: BreakKind;
}
export interface LineDecision {
  emergency: boolean;
  end: TextBoundary;
  hanging?: HangingLineEnd | null;
  overflows: boolean;
}
/**
 * Source coordinates and exact alignment bounds of the non-hanging text.
 * Actual glyph origins, advance and ink bounds continue to include punctuation.
 */
export interface HangingLineEnd {
  bodyPenMax: FixedQ32;
  bodyPenMin: FixedQ32;
  end: TextBoundary;
  start: TextBoundary;
}
export interface LineGeometryResult {
  issues: GeometryIssue[];
  /**
   * None when any layout prerequisite is unresolved; no partial geometry.
   */
  layout?: GeometryLayout | null;
  metricInstances: MetricInstance[];
  profile: string;
  shaping: LineShapeResult;
}
export interface GeometryLayout {
  height: Emu;
  lines: GeometryLine[];
}
export interface GeometryLine {
  /**
   * Signed int64 EMU. 1 point = 12700 EMU. Range requires semantic validation.
   */
  advance: string;
  advanceY: Emu;
  baseline: Emu;
  bottom: Emu;
  glyphs: PositionedGlyph[];
  height: Emu;
  penMax: Emu;
  penMin: Emu;
  removedByX9: FragmentRef[];
  top: Emu;
  visualFragments: FragmentRef[];
}
export interface PositionedGlyph {
  glyph: number;
  source: FragmentRef;
  x: Emu;
  y: Emu;
}
export interface FragmentRef {
  fallbackItem: number;
  fragment: number;
}
export interface MetricInstance {
  font: number;
  measured: MeasuredInstance;
  positionUnitsPerEm: number;
  variations: ShapeVariation[];
}
export interface MeasuredInstance {
  effectiveVariations: EffectiveVariation[];
  values: MeasuredMetric[];
}
export interface EffectiveVariation {
  /**
   * Effective IEEE754 binary32 design coordinate. Rounding is explicit.
   */
  effectiveF32Bits: number;
  requested1616: number;
  tag: string;
}
export interface MeasuredMetric {
  metric: FontMetric;
  /**
   * None means the component could not read this metric; zero is a real value.
   */
  position?: number | null;
}
export interface ShapeVariation {
  tag: string;
  /**
   * Exact requested OpenType 16.16 design coordinate.
   */
  value1616: number;
}
export interface LineShapeResult {
  bidi: BidiParagraphResult;
  fallback: FallbackResult;
  /**
   * Paragraph-resolved script/style and line-L1-adjusted cluster levels.
   */
  items: TextItem[];
  lines: ShapedLine[];
  notices: ItemizationNotice[];
  profile: string;
  shapedItemIndices: number[];
}
export interface BidiParagraphResult {
  lines: BidiLine[];
  paragraphLevel: number;
  profile: string;
  /**
   * I1/I2 result, before line-specific L1 resets. None means removed by X9.
   */
  resolvedLevels: (number | null)[];
}
export interface BidiLine {
  end: TextBoundary;
  /**
   * L1-adjusted levels, one per scalar in this line. X9 entries remain None.
   */
  levels: (number | null)[];
  start: TextBoundary;
  /**
   * L2 visual-to-logical scalar indices relative to the full paragraph.
   * X9 entries are omitted. This is not a glyph/cluster painting order (L3/L4).
   */
  visualOrder: number[];
}
export interface FallbackResult {
  componentCalls: number;
  contextScalars: number;
  items: FallbackItem[];
  probedGlyphs: number;
  profile: string;
  shapingRuns: number;
  verifiedFaces: number;
}
export interface FallbackItem {
  end: number;
  /**
   * Exhaustive, disjoint, logical ranges; not visual painting order.
   */
  fragments: FontFragment[];
  /**
   * Whole-item probe evidence, in the caller's candidate order.
   */
  probes: FontAttempt[];
  /**
   * Grapheme boundaries disallowed by observed clusters/unsafe-to-break.
   */
  protectedBoundaries: number[];
  reshapeRejections: ReshapeRejection[];
  start: number;
}
export interface ShapedText {
  faceIndex: number;
  fontSha256: Digest;
  positionUnitsPerEm: number;
  profile: string;
  runs: ShapedRun[];
  unitsPerEm: number;
}
export interface ShapedRun {
  direction: Direction;
  effectiveVariations: EffectiveVariation[];
  end: number;
  glyphs: ShapedGlyph[];
  /**
   * A computational success may still contain missing glyphs; never hide it.
   */
  missingGlyphClusters: number[];
  start: number;
}
export interface ShapedGlyph {
  cluster: number;
  glyphId: number;
  safeToInsertTatweel: boolean;
  unsafeToBreak: boolean;
  unsafeToConcat: boolean;
  xAdvance: number;
  xOffset: number;
  yAdvance: number;
  yOffset: number;
}
export interface FontAttempt {
  candidate: number;
  font: number;
  missingGlyphClusters: number[];
  /**
   * Conservative cmap-14 evidence, not proof of GSUB-only selector handling.
   */
  variationIssues: VariationIssue[];
}
export interface VariationIssue {
  /**
   * None for a selector without an immediately preceding non-selector scalar.
   */
  baseOffset?: number | null;
  outcome: CoverageOutcome;
  selectorOffset: number;
}
export interface ReshapeRejection {
  candidate: number;
  end: number;
  missingGlyphClusters: number[];
  start: number;
  variationIssues: VariationIssue[];
}
export interface TextItem {
  end: TextBoundary;
  kind: TextItemKind;
  /**
   * Before line-specific L1 resets; not a final glyph painting order.
   */
  level: number;
  script: Script;
  start: TextBoundary;
  style: number;
}
export interface ShapedLine {
  end: TextBoundary;
  fallbackEnd: number;
  /**
   * Half-open indices into fallback.items and shapedItemIndices.
   */
  fallbackStart: number;
  itemEnd: number;
  /**
   * Half-open indices into the result's logical items.
   */
  itemStart: number;
  start: TextBoundary;
}
export interface ItemizationNotice {
  end: number;
  kind: ItemizationNoticeKind;
  start: number;
}
export interface FlowWork {
  candidateScalars: number;
  componentCalls: number;
  contextScalars: number;
  evaluatedCandidates: number;
  probedGlyphs: number;
  shapingRuns: number;
  verifiedFaces: number;
}
/**
 * Computed, immutable data. Public queries accept this trusted Rust value,
 * never an unvalidated serialized map supplied by an external caller.
 */
export interface InteractionMap {
  boundaries: TextBoundary[];
  cells: InteractionCell[];
  lines: InteractionLine[];
  paragraphLevel: number;
  profile: string;
  work: InteractionWork;
}
export interface InteractionCell {
  end: TextBoundary;
  kind: TextItemKind;
  leading: CaretEdge;
  level: number;
  line: number;
  placement: CaretPlacement;
  start: TextBoundary;
  trailing: CaretEdge;
}
export interface CaretEdge {
  bottom: FixedQ32;
  top: FixedQ32;
  x: FixedQ32;
}
export interface InteractionLine {
  bottom: FixedQ32;
  /**
   * Indices into the paragraph's logical-order cells.
   */
  cells: number[];
  emptyCaret: CaretEdge;
  end: TextBoundary;
  start: TextBoundary;
  top: FixedQ32;
}
export interface InteractionWork {
  caretCalls: number;
  fontInstances: number;
  uniqueGlyphs: number;
}
export interface ResolvedCaret {
  boundary: TextBoundary;
  edge: CaretEdge;
  line: number;
  position: TextPosition;
}
export interface TextPosition {
  affinity: Affinity;
  scalarOffset: number;
}
export interface CaretNavigation {
  caret: ResolvedCaret;
  /**
   * Directional movement reached the paragraph edge; containers may continue.
   */
  exhausted: boolean;
  preferredX?: FixedQ32 | null;
}
export interface SelectionFragment {
  bounds: Rect;
  end: TextBoundary;
  kind: TextItemKind;
  line: number;
  start: TextBoundary;
}
export interface Rect {
  max: Point;
  min: Point;
}
export interface Point {
  x: FixedQ32;
  y: FixedQ32;
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
