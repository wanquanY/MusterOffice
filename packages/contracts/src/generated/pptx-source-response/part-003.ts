/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */
import type { PptxFailureCode, SourceColor, SourceEffectProperties, SourceFill, SourceFontCollection, SourceLine, SourceTextCatalog } from './part-002.js';

export interface SourceColorScheme {
  colors: {
    accent1?: SourceColor;
    accent2?: SourceColor;
    accent3?: SourceColor;
    accent4?: SourceColor;
    accent5?: SourceColor;
    accent6?: SourceColor;
    dk1?: SourceColor;
    dk2?: SourceColor;
    folHlink?: SourceColor;
    hlink?: SourceColor;
    lt1?: SourceColor;
    lt2?: SourceColor;
  };
  name: string;
  sourceOrdinal: number;
}

export interface SourceFontScheme {
  major: SourceFontCollection;
  minor: SourceFontCollection;
  name: string;
  /**
   * Unknown font-scheme declarations prevent claiming resolved font semantics.
   */
  retainedOrdinals?: number[];
  sourceOrdinal: number;
}

export interface SourceFormatScheme {
  backgroundFills: SourceStyleEntry[];
  effects: SourceStyleEntry[];
  fills: SourceStyleEntry[];
  lines: SourceStyleEntry[];
  name?: string | null;
  sourceOrdinal: number;
}

export interface SourceStyleEntry {
  effectStyle?: SourceEffectStyle | null;
  fill?: SourceFill | null;
  /**
   * Parsed line style declaration.
   */
  line?: SourceLine | null;
  /**
   * DrawingML kind; content remains in the digest-bound source part.
   */
  localName: string;
  sourceOrdinal: number;
}

export interface SourceEffectStyle {
  effects: SourceEffectProperties;
  /**
   * Includes 3D properties until their independent source family is parsed.
   */
  retainedOrdinals: number[];
  sourceOrdinal: number;
}

export interface SourceThemeTextDefaults {
  entries: {
    lnDef?: SourceThemeTextDefault;
    spDef?: SourceThemeTextDefault;
    txDef?: SourceThemeTextDefault;
  };
  retainedOrdinals: number[];
  sourceOrdinal: number;
}

export interface SourceThemeTextDefault {
  retainedOrdinals: number[];
  sourceOrdinal: number;
  text: SourceTextCatalog;
}

export interface PptxFailure {
  code: PptxFailureCode;
  message: string;
}
