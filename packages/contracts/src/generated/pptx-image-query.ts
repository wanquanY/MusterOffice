/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `SourceImageQuery`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;
/**
 * An explicit interpretation, not a certificate for any Office/WPS version.
 *
 * This interface was referenced by `SourceImageQuery`'s JSON-Schema
 * via the `definition` "FillProfile".
 */
export type FillProfile = "ms-oi29500-fills-2024-draft-v1";
/**
 * This interface was referenced by `SourceImageQuery`'s JSON-Schema
 * via the `definition` "FillTarget".
 */
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
/**
 * This interface was referenced by `SourceImageQuery`'s JSON-Schema
 * via the `definition` "TableCellEdge".
 */
export type TableCellEdge = "left" | "right" | "top" | "bottom" | "topLeftToBottomRight" | "bottomLeftToTopRight";
/**
 * This interface was referenced by `SourceImageQuery`'s JSON-Schema
 * via the `definition` "TableStyleRegion".
 */
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
/**
 * This interface was referenced by `SourceImageQuery`'s JSON-Schema
 * via the `definition` "TableStyleEdge".
 */
export type TableStyleEdge =
  | "left"
  | "right"
  | "top"
  | "bottom"
  | "insideHorizontal"
  | "insideVertical"
  | "topLeftToBottomRight"
  | "topRightToBottomLeft";
/**
 * Explicit source policy. An embedded snapshot is never silently substituted
 * for a requested linked source, nor does inspection grant network authority.
 *
 * This interface was referenced by `SourceImageQuery`'s JSON-Schema
 * via the `definition` "ImageSourceSelection".
 */
export type ImageSourceSelection = "embeddedSnapshot" | "linkedSource";

export interface SourceImageQuery {
  fill: SourceFillQuery;
  selection: ImageSourceSelection;
}
/**
 * This interface was referenced by `SourceImageQuery`'s JSON-Schema
 * via the `definition` "SourceFillQuery".
 */
export interface SourceFillQuery {
  expectedSourceSha256: Digest;
  profile: FillProfile;
  surface: string;
  targets: FillTarget[];
}
/**
 * This interface was referenced by `SourceImageQuery`'s JSON-Schema
 * via the `definition` "SourceCellAddress".
 */
export interface SourceCellAddress {
  column: number;
  row: number;
}
