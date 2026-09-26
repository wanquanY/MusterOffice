/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type ItemizationResponse =
  | {
      result: ItemizationResult;
      status: "itemized";
    }
  | {
      error: ShapeFailure;
      status: "error";
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

export interface ItemizationResult {
  bidi: BidiParagraphResult;
  items: TextItem[];
  notices: ItemizationNotice[];
  profile: string;
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
