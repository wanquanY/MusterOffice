/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type LineBreakResponse =
  | {
      characters: LineBreakCharacterProperties[];
      status: "analyzed";
      texts: LineBreakAnalysis[];
      unicodeVersion: string;
    }
  | {
      code: TextAnalysisFailureCode;
      message: string;
      status: "error";
    };
export type LineBreakClass =
  | "XX"
  | "AI"
  | "AK"
  | "AL"
  | "AP"
  | "AS"
  | "B2"
  | "BA"
  | "BB"
  | "BK"
  | "CB"
  | "CJ"
  | "CL"
  | "CM"
  | "CP"
  | "CR"
  | "EB"
  | "EM"
  | "EX"
  | "GL"
  | "H2"
  | "H3"
  | "HH"
  | "HL"
  | "HY"
  | "ID"
  | "IN"
  | "IS"
  | "JL"
  | "JT"
  | "JV"
  | "LF"
  | "NL"
  | "NS"
  | "NU"
  | "OP"
  | "PO"
  | "PR"
  | "QU"
  | "RI"
  | "SA"
  | "SG"
  | "SP"
  | "SY"
  | "VF"
  | "VI"
  | "WJ"
  | "ZW"
  | "ZWJ";
export type BreakKind = "allowed" | "mandatory";
export type TextAnalysisFailureCode = "INPUT_INVALID" | "LIMIT_EXCEEDED" | "CANCELLED";

export interface LineBreakCharacterProperties {
  codepoint: number;
  properties: LineBreakProperties;
}
export interface LineBreakProperties {
  class: LineBreakClass;
  /**
   * General_Category is Mn or Mc (not all marks).
   */
  combiningMark: boolean;
  /**
   * East_Asian_Width is F, W or H; not a measured display width.
   */
  eastAsian: boolean;
  finalPunctuation: boolean;
  initialPunctuation: boolean;
  unassigned: boolean;
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
