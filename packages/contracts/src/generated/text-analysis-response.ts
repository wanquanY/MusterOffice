/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type TextAnalysisResponse =
  | {
      characters: CharacterProperties[];
      status: "analyzed";
      texts: TextSegmentation[];
      unicodeVersion: string;
    }
  | {
      code: TextAnalysisFailureCode;
      message: string;
      status: "error";
    };
export type GraphemeBreak =
  | "other"
  | "cr"
  | "lf"
  | "control"
  | "extend"
  | "zwj"
  | "regionalIndicator"
  | "prepend"
  | "spacingMark"
  | "l"
  | "v"
  | "t"
  | "lv"
  | "lvt";
export type IndicConjunct = "none" | "consonant" | "extend" | "linker";
export type TextAnalysisFailureCode = "INPUT_INVALID" | "LIMIT_EXCEEDED" | "CANCELLED";

export interface CharacterProperties {
  codepoint: number;
  properties: Properties;
}
export interface Properties {
  defaultIgnorable: boolean;
  extendedPictographic: boolean;
  graphemeBreak: GraphemeBreak;
  indicConjunct: IndicConjunct;
  variationSelector: boolean;
}
export interface TextSegmentation {
  /**
   * Start and end boundaries; an empty text has one zero sentinel.
   */
  boundaries: TextBoundary[];
  defaultIgnorables: number[];
  profile: string;
  variationSelectors: number[];
}
export interface TextBoundary {
  scalarOffset: number;
  utf16Offset: number;
  utf8Offset: number;
}
