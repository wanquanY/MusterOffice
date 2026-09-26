/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type FontResponse =
  | {
      font: FontInspection;
      status: "inspected";
    }
  | {
      error: FontFailure;
      status: "error";
    };
/**
 * Canonical uint64 byte length. Range requires semantic validation.
 */
export type ByteLength = string;
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
export type Digest = string;
export type FontFailureCode =
  "INPUT_INVALID" | "FONT_INVALID" | "UNSUPPORTED" | "RESOURCE_CONFLICT" | "LIMIT_EXCEEDED" | "CANCELLED";

export interface FontInspection {
  axes: FontAxis[];
  byteLength: ByteLength;
  cmap?: FontCmap | null;
  coverage: FontCoverage[];
  faceCount: number;
  faceIndex: number;
  fontRevision1616: number;
  glyphCount: number;
  instances: FontInstance[];
  /**
   * Format 1 language tags, addressed by name language IDs starting at 0x8000.
   */
  languageTags: string[];
  metrics: FontMetrics;
  /**
   * Original record order; no locale-dependent family-name selection.
   */
  names: FontName[];
  notices: string[];
  /**
   * OS/2 flags are font declarations, not legal authorization to distribute.
   */
  os2?: Os2Metadata | null;
  sfntVersion: number;
  sha256: Digest;
  tables: FontTable[];
  unitsPerEm: number;
}
export interface FontAxis {
  default1616: number;
  flags: number;
  maximum1616: number;
  minimum1616: number;
  nameId: number;
  tag: string;
}
export interface FontCmap {
  encodingId: number;
  format: number;
  platformId: number;
  recordIndex: number;
  variationRecordIndex?: number | null;
}
export interface FontCoverage {
  character: FontCharacter;
  outcome: CoverageOutcome;
}
export interface FontCharacter {
  codepoint: number;
  variationSelector?: number | null;
}
export interface FontInstance {
  /**
   * Exact fvar 16.16 coordinates in original axis order, before avar mapping.
   */
  coordinates1616: number[];
  flags: number;
  postscriptNameId?: number | null;
  subfamilyNameId: number;
}
export interface FontMetrics {
  horizontalAscender: number;
  horizontalDescender: number;
  horizontalLineGap: number;
  vertical?: VerticalMetrics | null;
}
export interface VerticalMetrics {
  ascender: number;
  descender: number;
  lineGap: number;
}
export interface FontName {
  encodingId: number;
  languageId: number;
  nameId: number;
  platformId: number;
  /**
   * None for an unsupported encoding; source bytes remain bound by SHA-256.
   */
  text?: string | null;
}
export interface Os2Metadata {
  fsSelection: number;
  fsType: number;
  typoAscender: number;
  typoDescender: number;
  typoLineGap: number;
  version: number;
  weightClass: number;
  widthClass: number;
  winAscent: number;
  winDescent: number;
}
export interface FontTable {
  byteLength: ByteLength;
  checksum: number;
  offset: ByteLength;
  tag: string;
}
export interface FontFailure {
  code: FontFailureCode;
  message: string;
}
