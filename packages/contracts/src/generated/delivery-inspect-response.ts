/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type DeliveryInspectResponse =
  | {
      report: ReceiptInspection;
      status: "inspected";
    }
  | {
      error: PptxFailure;
      status: "error";
    };
export type Digest = string;
export type ClaimBasis = "none" | "static-inspection" | "roundtrip" | "application-test";
export type RequestId = string;
export type ClaimKind = "structure" | "layout" | "native-editability" | "playback" | "target-application";
export type ClaimStatus = "passed" | "failed" | "not_proven" | "not_applicable";
export type DocumentId = string;
/**
 * Signed int64 EMU. 1 point = 12700 EMU. Range requires semantic validation.
 */
export type Emu = string;
export type ObjectId = string;
export type SlideId = string;
/**
 * Canonical uint64 byte length. Range requires semantic validation.
 */
export type ByteLength = string;
export type PptxFailureCode =
  | "INPUT_INVALID"
  | "SOURCE_CONFLICT"
  | "PRESERVATION_CONFLICT"
  | "MAPPING_NOT_IMPLEMENTED"
  | "RESOURCE_REQUIRED"
  | "LIMIT_EXCEEDED"
  | "CANCELLED"
  | "READ_FAILED";

/**
 * A diagnostic report, not a transferable grant or a substitute for a commit.
 * Claims remain producer declarations; this checker does not upgrade them.
 */
export interface ReceiptInspection {
  assetsVerified: number;
  bundleDigest: Digest;
  declaredClaims: Claim[];
  documentId: DocumentId;
  /**
   * Bounded observations from the verified preview evidence. Historical
   * reports can omit them; absence never establishes layout quality.
   */
  layoutDiagnostics?: LayoutDiagnostics | null;
  pages: number;
  profile: string;
  revision: Digest;
  semanticDigest: Digest;
  settingsDigest: Digest;
  totalBytes: ByteLength;
}
export interface Claim {
  basis: ClaimBasis;
  evidenceAssetIds: RequestId[];
  kind: ClaimKind;
  profileId: string;
  reason?: string | null;
  status: ClaimStatus;
  subjectSha256: Digest;
}
export interface LayoutDiagnostics {
  affectedFrames: number;
  /**
   * At most 32 affected frames and 24 KiB of finding JSON in page/paint order.
   * Once either budget is exhausted the remaining findings are counted only.
   */
  findings: TextLayoutFinding[];
  measuredFrames: number;
  measuredPages: number;
  omittedFindings: number;
  profile: string;
  textOverlaps?: TextOverlapObservations | null;
  unmeasuredPages: number;
}
export interface TextLayoutFinding {
  /**
   * Includes line leading/paragraph spacing; not necessarily visible spill.
   */
  capacityExcessEmu: string;
  cell?: SourceCellAddress | null;
  contentHeightEmu: Emu;
  emergencyLines: number;
  evidenceAssetId: RequestId;
  horizontalOverflowLines: number;
  inkExcessEmu: InkExcess;
  innerHeightEmu: Emu;
  /**
   * Signed int64 EMU. 1 point = 12700 EMU. Range requires semantic validation.
   */
  innerWidthEmu: string;
  lineCount: number;
  /**
   * The editable model identity, never guessed from the object's name.
   */
  objectId?: ObjectId | null;
  pageId: SlideId;
  /**
   * One-based position in the delivered deck, including hidden slides.
   */
  pageNumber: number;
  source: SourceObjectRef;
}
export interface SourceCellAddress {
  column: number;
  row: number;
}
/**
 * Glyph outlines outside the inner text region. Intentional overhang and
 * hanging punctuation can cause this; it is not a clipping/overlap verdict.
 */
export interface InkExcess {
  bottom: Emu;
  left: Emu;
  right: Emu;
  top: Emu;
}
export interface SourceObjectRef {
  nativeId: number;
  part: string;
}
export interface TextOverlapObservations {
  checkedPairs: number;
  /**
   * At most 16 candidate pairs and 12 KiB of candidate JSON, in paint order.
   */
  findings: TextOverlapFinding[];
  /**
   * Positive-area intersections of page-space text ink envelopes. Bounds
   * precede clipping/compositing and may include whitespace between glyphs.
   */
  intersectingPairs: number;
  measuredPages: number;
  omittedFindings: number;
  profile: string;
  uncheckedPairs: number;
  unmeasuredPages: number;
}
export interface TextOverlapFinding {
  evidenceAssetId: RequestId;
  first: TextInkReference;
  intersectionHeightEmu: Emu;
  intersectionWidthEmu: Emu;
  pageId: SlideId;
  pageNumber: number;
  second: TextInkReference;
}
export interface TextInkReference {
  cell?: SourceCellAddress | null;
  clippingApplied: boolean;
  coordinateErrorEmu: Emu;
  objectId?: ObjectId | null;
  source: SourceObjectRef;
}
export interface PptxFailure {
  code: PptxFailureCode;
  message: string;
}
