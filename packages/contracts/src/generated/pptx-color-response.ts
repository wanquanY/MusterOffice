/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type PptxColorResponse =
  | {
      palette: SourceColorPalette;
      status: "evaluated";
    }
  | {
      error: PptxFailure;
      status: "error";
    };
export type ColorDependency =
  | {
      kind: "theme";
      part: string;
      slot: ColorSlot;
      sourceOrdinal: number;
    }
  | {
      color: SystemColor;
      kind: "system";
      origin: SystemColorOrigin;
    }
  | {
      kind: "placeholder";
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
export type SystemColorOrigin = "hostContext" | "fileLastColor";
export type ColorNotice = "grayWeightsProvisional" | "presetAliasDiscrepancy";
export type ColorOutcome =
  | {
      clippedForSrgb: boolean;
      /**
       * @minItems 4
       * @maxItems 4
       */
      rgba16: [number, number, number, number];
      /**
       * @minItems 4
       * @maxItems 4
       */
      rgba8: [number, number, number, number];
      status: "resolved";
    }
  | {
      reason: ColorUnresolved;
      status: "unresolved";
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
export type SchemeColor =
  | "bg1"
  | "tx1"
  | "bg2"
  | "tx2"
  | "accent1"
  | "accent2"
  | "accent3"
  | "accent4"
  | "accent5"
  | "accent6"
  | "hlink"
  | "folHlink"
  | "phClr"
  | "dk1"
  | "lt1"
  | "dk2"
  | "lt2";
/**
 * A named, provisional numerical interpretation, not an Office/WPS certificate.
 */
export type ColorProfile = "ecma376-2016-draft-v1";
export type Digest = string;
export type PptxFailureCode =
  | "INPUT_INVALID"
  | "SOURCE_CONFLICT"
  | "PRESERVATION_CONFLICT"
  | "MAPPING_NOT_IMPLEMENTED"
  | "RESOURCE_REQUIRED"
  | "LIMIT_EXCEEDED"
  | "CANCELLED"
  | "READ_FAILED";

export interface SourceColorPalette {
  colorMapping?: SourceColorMapRef | null;
  colorScheme?: SourceThemeSchemeRef | null;
  /**
   * Input order, including repeated queries.
   */
  colors: SchemeColorResult[];
  profile: ColorProfile;
  sourceSha256: Digest;
  surface: string;
}
export interface SourceColorMapRef {
  part: string;
  sourceOrdinal: number;
}
export interface SourceThemeSchemeRef {
  part: string;
  sourceOrdinal: number;
}
export interface SchemeColorResult {
  /**
   * Outer-to-inner dependency path. Original transforms are not rewritten.
   */
  dependencies: ColorDependency[];
  notices: ColorNotice[];
  outcome: ColorOutcome;
  scheme: SchemeColor;
}
export interface PptxFailure {
  code: PptxFailureCode;
  message: string;
}
