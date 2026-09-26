/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

export type HostResponse =
  | {
      job: JobInfo;
      outcome: "accepted";
      pollAfterMs: number;
    }
  | {
      outcome: "succeeded";
      result: HostResult;
    }
  | {
      error: Failure;
      job?: JobInfo | null;
      outcome: "failed";
    };
export type ContractVersion = "musteroffice.operations/1-draft";
export type UnixMillis = string;
export type DocumentId = string;
export type Digest = string;
export type JobFence = string;
export type RequestId = string;
export type TerminalResult =
  | {
      outcome: "succeeded";
      receipt: OperationReceipt;
    }
  | {
      error: Failure;
      outcome: "failed";
    };
/**
 * The mutation wire shape is preserved for existing durable receipts. Both
 * alternatives deny unknown fields and have distinct required members.
 */
export type OperationReceipt = MutationReceipt | ExportReceipt;
export type ParagraphId = string;
export type FontId = string;
export type LayoutId = string;
export type MasterId = string;
export type ResourceId = string;
export type ThemeId = string;
export type SlideId = string;
export type ObjectId = string;
/**
 * Canonical uint64 byte length. Range requires semantic validation.
 */
export type ByteLength = string;
export type AssetRole =
  | "editable-document"
  | "pptx"
  | "preview"
  | "quality-report"
  | "image"
  | "font"
  | "audio"
  | "video"
  | "model3d"
  | "embedded"
  | "source"
  | "other";
export type ClaimBasis = "none" | "static-inspection" | "roundtrip" | "application-test";
export type ClaimKind = "structure" | "layout" | "native-editability" | "playback" | "target-application";
export type ClaimStatus = "passed" | "failed" | "not_proven" | "not_applicable";
export type PreviewSample = {
  mode: "editor";
};
export type FailureCode =
  | (
      | "INPUT_INVALID"
      | "NOT_AUTHORIZED"
      | "NOT_FOUND"
      | "REQUEST_ID_REUSED"
      | "REVISION_CONFLICT"
      | "REFERENCE_CONFLICT"
      | "DOCUMENT_EXISTS"
      | "LIMIT_EXCEEDED"
      | "CANCELLED"
      | "EXECUTION_INTERRUPTED"
      | "EXECUTOR_MISMATCH"
      | "STALE_EXECUTION"
      | "STORAGE_FAILURE"
      | "RESOURCE_CONFLICT"
      | "RESOURCE_EXPIRED"
      | "RESOURCE_INCOMPLETE"
      | "RESOURCE_BUSY"
      | "MAPPING_NOT_IMPLEMENTED"
      | "RENDER_FAILURE"
    )
  | "STORAGE_BUSY";
export type JobState = "queued" | "running" | "succeeded" | "failed" | "cancelled";
export type HostResult =
  | {
      capabilities: HostCapabilities;
      kind: "capabilities";
    }
  | {
      document: SchemaDocument;
      kind: "schema";
    }
  | {
      kind: "upload";
      upload: UploadInfo;
    }
  | {
      asset: AssetInfo;
      kind: "asset";
    }
  | {
      job: JobInfo;
      kind: "job";
    }
  | {
      kind: "document";
      snapshot: SnapshotRecord;
    };
export type OperationChannel = "controlJson" | "binaryUpload" | "binaryRead";
export type ServiceOperation =
  | "presentations.import"
  | "capabilities"
  | "schemas.get"
  | "presentations.create"
  | "presentations.apply"
  | "presentations.export"
  | "presentations.read"
  | "assets.beginUpload"
  | "assets.getUpload"
  | "assets.appendUpload"
  | "assets.sealUpload"
  | "assets.cancelUpload"
  | "assets.read"
  | "assets.readRange"
  | "jobs.get"
  | "jobs.cancel";
export type OutputMode = "auto" | "sync" | "job";
export type OperationProfile = "presentations-pptx-resource-delivery-v1-draft" | "presentations-author-model-v01-draft";
export type Permission =
  "create" | "edit" | "export" | "readDocument" | "readJob" | "cancelJob" | "writeAssets" | "readAssets";
export type RevisionPolicy = "newDocument" | "compareCurrentHead" | "immutableHistorical" | "readSelectedOrHead";
export type UnavailableReason = "previewRendererNotConfigured" | "executionUnavailable";
export type JobExecution = "explicitRun" | "hostScheduled";
export type SchemaId =
  | "host-request"
  | "host-response"
  | "operation-request"
  | "operation-job"
  | "host-capabilities"
  | "schema-document"
  | "document"
  | "upload-request"
  | "upload-info"
  | "asset-info";
export type AssetVerification = "bytesSha256";
export type UploadState = "uploading" | "verifying" | "sealed" | "failed" | "cancelled";
export type ModelVersion = "musteroffice.presentation/0.1-draft";
export type Inherited =
  | {
      kind: "inherit";
    }
  | {
      kind: "value";
      value: Fill;
    };
export type Fill =
  | {
      kind: "none";
    }
  | {
      color: Color;
      kind: "solid";
    };
export type Color =
  | {
      kind: "srgb";
      rgba: Rgba;
    }
  | {
      kind: "theme";
      slot: ThemeColor;
    };
export type ThemeColor =
  | "dark1"
  | "light1"
  | "dark2"
  | "light2"
  | "accent1"
  | "accent2"
  | "accent3"
  | "accent4"
  | "accent5"
  | "accent6"
  | "hyperlink"
  | "followedHyperlink";
/**
 * Signed int64 EMU. 1 point = 12700 EMU. Range requires semantic validation.
 */
export type Emu = string;
export type Stroke =
  | {
      kind: "none";
    }
  | {
      /**
       * Absent retains an unresolved declaration, not an implicit flat cap.
       */
      cap?: LineCap | null;
      color: Color;
      /**
       * Absent retains the source/default distinction.
       */
      join?: LineJoin | null;
      kind: "solid";
      width: Emu;
    };
export type LineCap = "flat" | "round" | "square";
export type LineJoin =
  | {
      kind: "round";
    }
  | {
      kind: "bevel";
    }
  | {
      kind: "miter";
      /**
       * Ratio in 1/100000 units: 400000 denotes four times line width.
       * The ratio compares full miter length with the full stroke width.
       */
      limit?: number | null;
    };
export type ObjectContent =
  | {
      children: ObjectId[];
      kind: "retainedSource";
      native_kind: RetainedObjectKind;
      paragraphs: RetainedParagraph[];
    }
  | {
      geometry: Geometry;
      kind: "shape";
      text?: TextBody | null;
    }
  | {
      crop: Crop;
      kind: "picture";
      resource: ResourceId;
    }
  | {
      children: ObjectId[];
      kind: "group";
      viewport: Size;
    }
  | {
      end: ConnectorEndpoint;
      kind: "connector";
      start: ConnectorEndpoint;
    };
export type RetainedObjectKind = "shape" | "picture" | "group" | "connector" | "graphicFrame";
export type RunId = string;
export type RetainedRunKind = "text" | "break" | "field";
export type Geometry =
  | {
      kind: "rectangle";
    }
  | {
      kind: "ellipse";
    }
  | {
      kind: "roundRectangle";
      radius: Emu;
    }
  | {
      commands: PathCommand[];
      kind: "path";
      viewport: Size;
    };
export type PathCommand =
  | {
      kind: "move";
      to: Point;
    }
  | {
      kind: "line";
      to: Point;
    }
  | {
      control: Point;
      kind: "quadratic";
      to: Point;
    }
  | {
      control1: Point;
      control2: Point;
      kind: "cubic";
      to: Point;
    }
  | {
      kind: "close";
    };
export type OverflowPolicy = "report" | "clip" | "growShape";
export type InlineContent =
  | {
      kind: "text";
      text: string;
    }
  | {
      kind: "break";
    }
  | {
      kind: "tab";
    };
export type Alignment = "start" | "center" | "end" | "justify";
export type TextDirection = "leftToRight" | "rightToLeft" | "verticalRightToLeft" | "verticalLeftToRight";
export type ConnectorEndpoint =
  | {
      kind: "free";
      position: Point;
    }
  | {
      kind: "attached";
      object: ObjectId;
      site: number;
    };
export type ContainerId =
  | {
      id: SlideId;
      kind: "slide";
    }
  | {
      id: MasterId;
      kind: "master";
    }
  | {
      id: LayoutId;
      kind: "layout";
    }
  | {
      id: ObjectId;
      kind: "group";
    };
export type ResourceKind = "font" | "picture" | "audio" | "video" | "sourcePackage" | "embeddedWorkbook" | "model3d";
export type NativeEditConstraint =
  | "missingDirectTransform"
  | "retainedTransform"
  | "compatibilityBranch"
  | "structuredLeaf"
  | "dynamicField"
  | "timingReferences"
  | "retainedReferences";
export type SourceBindingProfile = "presentationml-retained-fields-v1-draft";
export type TimelineVersion = "musteroffice.timeline/0.1-draft" | "musteroffice.timeline/0.2-draft";
/**
 * Signed int64 ticks. Range requires semantic validation.
 */
export type Ticks = string;
export type Timescale = number;
export type TimeCondition =
  | {
      kind: "at";
      offset: RationalTime;
    }
  | {
      delay: RationalTime;
      event: NodeEvent;
      kind: "after";
      node: TimingNodeId;
    }
  | {
      delay: RationalTime;
      kind: "click";
      target?: ObjectId | null;
    };
export type NodeEvent = "begin" | "end";
export type TimingNodeId = string;
export type FillMode = ("remove" | "freeze") | "hold";
export type RepeatDuration = "indefinite" | RationalTime;
export type ContainerDuration =
  | {
      kind: "automatic";
    }
  | {
      duration: RationalTime;
      kind: "fixed";
    }
  | {
      kind: "indefinite";
    };
export type ContainerKind = "parallel" | "sequence";

export interface JobInfo {
  cancelRequested: boolean;
  contractVersion: ContractVersion;
  createdAt: UnixMillis;
  documentId: DocumentId;
  executorDigest: Digest;
  fence: JobFence;
  id: RequestId;
  leaseUntil?: UnixMillis | null;
  operation: string;
  requestDigest: Digest;
  requestId: RequestId;
  /**
   * Receipts remain durable until explicit host lifecycle management; there is
   * no advertised expiry or automatic deletion of committed documents.
   */
  result?: TerminalResult | null;
  state: JobState;
  updatedAt: UnixMillis;
}
export interface MutationReceipt {
  documentId: DocumentId;
  revision: Digest;
  semanticDigest: Digest;
  transaction?: TransactionReceipt | null;
}
export interface TransactionReceipt {
  baseRevision: Digest;
  changes: ChangeSet;
  documentId: DocumentId;
  requestDigest: Digest;
  requestId: RequestId;
  revision: Digest;
  semanticDigest: Digest;
}
export interface ChangeSet {
  anchorMaps: AnchorMap[];
  changedFonts: FontId[];
  changedLayouts: LayoutId[];
  changedMasters: MasterId[];
  changedResources: ResourceId[];
  changedThemes: ThemeId[];
  changedTimelines?: SlideId[];
  createdObjects: ObjectId[];
  createdSlides: SlideId[];
  deletedObjects: ObjectId[];
  deletedSlides: SlideId[];
  /**
   * Global page geometry changed. Otherwise use invalidated_slides; metadata
   * and timeline edits have separate change fields and do not dirty layout.
   */
  invalidateAllLayout: boolean;
  /**
   * Surviving/new pages whose static layout dependency closure changed.
   */
  invalidatedSlides?: SlideId[];
  metadataChanged: boolean;
  slideOrderChanged: boolean;
  updatedObjects: ObjectId[];
  updatedSlides: SlideId[];
}
export interface AnchorMap {
  deleted: number;
  inserted: number;
  paragraph: ParagraphId;
  start: number;
}
export interface ExportReceipt {
  bundle: DeliveryBundle;
  documentId: DocumentId;
  revision: Digest;
  semanticDigest: Digest;
}
export interface DeliveryBundle {
  assets: DeliveryAsset[];
  claims: Claim[];
  document: DeliveredDocument;
  pptxAssetId: RequestId;
  previews: Preview[];
  profileId: string;
  version: string;
  versions: Versions;
}
export interface DeliveryAsset {
  byteLength: ByteLength;
  id: RequestId;
  mediaType: string;
  role: AssetRole;
  sha256: Digest;
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
export interface DeliveredDocument {
  documentId: DocumentId;
  modelAssetId: RequestId;
  revision: Digest;
}
export interface Preview {
  height: number;
  imageAssetId: RequestId;
  pageId: SlideId;
  sample: PreviewSample;
  width: number;
}
export interface Versions {
  documentSchema: string;
  engine: string;
  featureRegistrySha256: Digest;
  fontProfileSha256: Digest;
  operationSchema: string;
  rules: string;
}
export interface Failure {
  code: FailureCode;
  detail?: unknown;
  message: string;
}
export interface HostCapabilities {
  completeFeatureCatalogue: boolean;
  contractVersion: ContractVersion;
  executorDigest: Digest;
  fullPresentationAcceptance: boolean;
  limits: ServiceLimits;
  operations: OperationDescriptor[];
  queuedExecution: JobExecution;
  renderer?: RendererIdentity | null;
  schemas: SchemaId[];
}
export interface ServiceLimits {
  assetBytes: ByteLength;
  assetChunkBytes: ByteLength;
  documentsPerScope: number;
  export: ExportLimits;
  jobsPerPrincipal: number;
  leaseMs: number;
  outputsPerJob: number;
  outputsPerScope: number;
  requestBytes: ByteLength;
  scopeReservedBytes: ByteLength;
  uploadTtlMs: number;
  uploadsPerScope: number;
}
export interface ExportLimits {
  artifacts: number;
  assetBytes: ByteLength;
  fontBytes: ByteLength;
  modelBytes: ByteLength;
  pages: number;
  totalBytes: ByteLength;
}
export interface OperationDescriptor {
  available: boolean;
  channel: OperationChannel;
  operation: ServiceOperation;
  /**
   * Empty for immediate queries/resource operations. Job preference applies
   * only to document operations accepted into the durable owner.
   */
  outputModes: OutputMode[];
  profileId?: OperationProfile | null;
  requiredPermissions: Permission[];
  revisionPolicy?: RevisionPolicy | null;
  unavailableReason?: UnavailableReason | null;
}
export interface RendererIdentity {
  implementationSha256: Digest;
  profile: string;
}
export interface SchemaDocument {
  /**
   * Canonical domain-bound ID+schema digest, not a digest of formatted JSON.
   */
  digest: string;
  id: SchemaId;
  schema: unknown;
}
export interface UploadInfo {
  asset?: AssetInfo | null;
  chunkBytes: number;
  createdAt: UnixMillis;
  descriptor: AssetDescriptor;
  error?: Failure | null;
  expiresAt?: UnixMillis | null;
  id: RequestId;
  receivedBytes: ByteLength;
  requestId: RequestId;
  state: UploadState;
  updatedAt: UnixMillis;
}
export interface AssetInfo {
  descriptor: AssetDescriptor;
  id: RequestId;
  verification: AssetVerification;
}
export interface AssetDescriptor {
  byteLength: ByteLength;
  /**
   * A declaration, not evidence of successful decoding or safe execution.
   */
  mediaType: string;
  sha256: Digest;
}
/**
 * Storage envelope owned by an authorized host. A digest is not an access token.
 */
export interface SnapshotRecord {
  document: Document;
  revision: Digest;
  semanticDigest: Digest;
}
/**
 * Document declarations and immutable source provenance. Revisions, compilation
 * caches, clocks and decoder state live outside this model.
 */
export interface Document {
  fonts: {
    [k: string]: FontFace | undefined;
  };
  format: ModelVersion;
  id: DocumentId;
  layouts: {
    [k: string]: Layout | undefined;
  };
  masters: {
    [k: string]: Master | undefined;
  };
  objects: {
    [k: string]: Object | undefined;
  };
  pageSize: Size;
  resources: {
    [k: string]: Resource | undefined;
  };
  slideOrder: SlideId[];
  slides: {
    [k: string]: Slide | undefined;
  };
  /**
   * Immutable native addresses/constraints. Known fields are edited in
   * objects; this provenance never becomes a separate mutable document.
   */
  sourceBindings?: SourceBindings | null;
  themes: {
    [k: string]: Theme | undefined;
  };
  /**
   * Slide-owned animation graphs; empty storage preserves older author digests.
   */
  timelines?: {
    [k: string]: Timeline | undefined;
  };
  title: string;
}
/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 */
export interface FontFace {
  faceIndex: number;
  family: string;
  id: FontId;
  italic: boolean;
  resource: ResourceId;
  weight: number;
}
/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 */
export interface Layout {
  background: Inherited;
  defaultText: CharacterStyle;
  id: LayoutId;
  master: MasterId;
  name: string;
  objects: ObjectId[];
}
export interface Rgba {
  alpha: number;
  blue: number;
  green: number;
  red: number;
}
export interface CharacterStyle {
  bold?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: boolean;
      };
  color?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: Color;
      };
  font?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: FontId;
      };
  italic?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: boolean;
      };
  language?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: string;
      };
  size?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: Emu;
      };
  underline?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: boolean;
      };
}
/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 */
export interface Master {
  background: Inherited;
  defaultText: CharacterStyle;
  id: MasterId;
  objects: ObjectId[];
  theme: ThemeId;
}
/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 */
export interface Object {
  accessibility: Accessibility;
  appearance: Appearance;
  content: ObjectContent;
  id: ObjectId;
  parent: ContainerId;
  /**
   * Missing only for retained native coordinates that cannot be represented
   * as a complete direct declaration. Never substitute a resolved identity.
   */
  transform?: Transform | null;
}
export interface Accessibility {
  decorative: boolean;
  description: string;
  title: string;
}
export interface Appearance {
  fill?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: Fill;
      };
  stroke?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: Stroke;
      };
}
export interface RetainedParagraph {
  id: ParagraphId;
  runs: RetainedTextRun[];
}
export interface RetainedTextRun {
  id: RunId;
  kind: RetainedRunKind;
  text: string;
}
export interface Point {
  x: Emu;
  y: Emu;
}
export interface Size {
  height: Emu;
  width: Emu;
}
export interface TextBody {
  insets: Insets;
  overflow: OverflowPolicy;
  paragraphs: Paragraph[];
  style: CharacterStyle;
  wrap: boolean;
}
export interface Insets {
  bottom: Emu;
  left: Emu;
  right: Emu;
  top: Emu;
}
export interface Paragraph {
  defaultRunStyle: CharacterStyle;
  id: ParagraphId;
  runs: TextRun[];
  style: ParagraphStyle;
}
export interface TextRun {
  content: InlineContent;
  id: RunId;
  style: CharacterStyle;
}
export interface ParagraphStyle {
  alignment?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: Alignment;
      };
  direction?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: TextDirection;
      };
  spaceAfter?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: Emu;
      };
  spaceBefore?:
    | {
        kind: "inherit";
      }
    | {
        kind: "value";
        value: Emu;
      };
}
export interface Crop {
  bottom: number;
  left: number;
  right: number;
  top: number;
}
export interface Transform {
  flipHorizontal: boolean;
  flipVertical: boolean;
  origin: Point;
  /**
   * Units of 1/60000 degree. Author direction is preserved.
   */
  rotation: number;
  size: Size;
}
/**
 * Opaque authorized handle and declared content identity; byte verification belongs to the host.
 *
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 */
export interface Resource {
  id: ResourceId;
  kind: ResourceKind;
  mediaType: string;
  sha256: Digest;
}
/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 */
export interface Slide {
  background: Inherited;
  hidden: boolean;
  id: SlideId;
  layout?: LayoutId | null;
  name: string;
  objects: ObjectId[];
}
export interface SourceBindings {
  layouts: {
    /**
     * This interface was referenced by `undefined`'s JSON-Schema definition
     * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
     */
    [k: string]: string | undefined;
  };
  masters: {
    /**
     * This interface was referenced by `undefined`'s JSON-Schema definition
     * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
     */
    [k: string]: string | undefined;
  };
  objects: {
    [k: string]: NativeObjectBinding | undefined;
  };
  profile: SourceBindingProfile;
  resource: ResourceId;
  slides: {
    /**
     * This interface was referenced by `undefined`'s JSON-Schema definition
     * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
     */
    [k: string]: string | undefined;
  };
  themes: {
    /**
     * This interface was referenced by `undefined`'s JSON-Schema definition
     * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
     */
    [k: string]: string | undefined;
  };
}
/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 */
export interface NativeObjectBinding {
  nativeId: number;
  part: string;
  runs: {
    [k: string]: NativeRunBinding | undefined;
  };
  /**
   * None means the complete direct transform can be changed in place.
   */
  transformConstraint?: NativeEditConstraint | null;
}
/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 */
export interface NativeRunBinding {
  constraint?: NativeEditConstraint | null;
  paragraph: number;
  run: number;
}
/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 */
export interface Theme {
  colors: {
    accent1?: Rgba;
    accent2?: Rgba;
    accent3?: Rgba;
    accent4?: Rgba;
    accent5?: Rgba;
    accent6?: Rgba;
    dark1?: Rgba;
    dark2?: Rgba;
    followedHyperlink?: Rgba;
    hyperlink?: Rgba;
    light1?: Rgba;
    light2?: Rgba;
  };
  defaultText: CharacterStyle;
  id: ThemeId;
  name: string;
}
/**
 * Behaviors plus an optional explicit timing forest. The legacy graph retains
 * its byte representation; version 0.2 owns every behavior through tree roots.
 *
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 */
export interface Timeline {
  format: TimelineVersion;
  nodes: TimingNode[];
  tree?: TimingTree | null;
}
export interface TimingNode {
  duration: RationalTime;
  /**
   * The current graph activates each node once (native restart="never").
   */
  effect: {
    from: number;
    kind: "rotation";
    target: ObjectId;
    to: number;
  };
  /**
   * Earliest resolved eligible end; absent conditions add no end constraint.
   */
  endConditions?: TimeCondition[];
  fill: FillMode;
  id: TimingNodeId;
  /**
   * Additional bound in local active time, before speed scaling.
   */
  repeatDuration?: RepeatDuration | null;
  /**
   * Native count in thousandths, or explicit indefinite repetition.
   */
  repeatMilli: "indefinite" | number;
  start: TimeCondition;
  timeTransform?: TimeTransform | null;
}
/**
 * Exact wire representation. Equality compares author values; compare_time compares instants.
 */
export interface RationalTime {
  ticks: Ticks;
  timescale: Timescale;
}
/**
 * Local behavior clock. Percentages use native thousandths of one percent;
 * 100000 speed is normal playback. The clock is independent of effect values.
 */
export interface TimeTransform {
  accelerationMilliPercent: number;
  autoReverse: boolean;
  decelerationMilliPercent: number;
  speedMilliPercent: number;
}
export interface TimingTree {
  containers: TimingContainer[];
  roots: TimingNodeId[];
}
export interface TimingContainer {
  children: TimingNodeId[];
  duration: ContainerDuration;
  endConditions?: TimeCondition[];
  fill: FillMode;
  id: TimingNodeId;
  kind: ContainerKind;
  start: TimeCondition;
}
