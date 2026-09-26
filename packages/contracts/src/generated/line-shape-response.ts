/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type LineShapeResponse =
  | {
      result: LineShapeResult;
      status: "evaluated";
    }
  | {
      error: ShapeFailure;
      status: "error";
    };
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
export interface TextBoundary {
  scalarOffset: number;
  utf16Offset: number;
  utf8Offset: number;
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
export interface EffectiveVariation {
  /**
   * Effective IEEE754 binary32 design coordinate. Rounding is explicit.
   */
  effectiveF32Bits: number;
  requested1616: number;
  tag: string;
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
