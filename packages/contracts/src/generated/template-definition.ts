/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */

/**
 * This interface was referenced by `TemplateDefinition`'s JSON-Schema
 * via the `definition` "TemplateVersion".
 */
export type TemplateVersion = "musteroffice.presentation-template/1-draft";
/**
 * This interface was referenced by `TemplateDefinition`'s JSON-Schema
 * via the `definition` "ParameterTarget".
 */
export type ParameterTarget =
  | {
      kind: "textRun";
      maxScalars: number;
      minScalars: number;
      object: ObjectId;
      paragraph: ParagraphId;
      run: RunId;
    }
  | {
      kind: "resource";
      mediaTypes: string[];
      resource: ResourceId;
    }
  | {
      kind: "themeColor";
      slot: ThemeColor;
      theme: ThemeId;
    }
  | {
      kind: "transform";
      object: ObjectId;
    };
/**
 * This interface was referenced by `TemplateDefinition`'s JSON-Schema
 * via the `definition` "ObjectId".
 */
export type ObjectId = string;
/**
 * This interface was referenced by `TemplateDefinition`'s JSON-Schema
 * via the `definition` "ParagraphId".
 */
export type ParagraphId = string;
/**
 * This interface was referenced by `TemplateDefinition`'s JSON-Schema
 * via the `definition` "RunId".
 */
export type RunId = string;
/**
 * This interface was referenced by `TemplateDefinition`'s JSON-Schema
 * via the `definition` "ResourceId".
 */
export type ResourceId = string;
/**
 * This interface was referenced by `TemplateDefinition`'s JSON-Schema
 * via the `definition` "ThemeColor".
 */
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
 * This interface was referenced by `TemplateDefinition`'s JSON-Schema
 * via the `definition` "ThemeId".
 */
export type ThemeId = string;
/**
 * This interface was referenced by `TemplateDefinition`'s JSON-Schema
 * via the `definition` "DocumentId".
 */
export type DocumentId = string;
/**
 * This interface was referenced by `TemplateDefinition`'s JSON-Schema
 * via the `definition` "Digest".
 */
export type Digest = string;

/**
 * A template is an exact document revision plus typed editable parameters.
 * Catalog identity, ownership, storage and inference are outside this contract.
 */
export interface TemplateDefinition {
  format: TemplateVersion;
  parameters: {
    [k: string]: Parameter | undefined;
  };
  source: TemplateSource;
}
/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$".
 *
 * This interface was referenced by `TemplateDefinition`'s JSON-Schema
 * via the `definition` "Parameter".
 */
export interface Parameter {
  label: string;
  required: boolean;
  target: ParameterTarget;
}
/**
 * This interface was referenced by `TemplateDefinition`'s JSON-Schema
 * via the `definition` "TemplateSource".
 */
export interface TemplateSource {
  documentId: DocumentId;
  revision: Digest;
  semanticDigest: Digest;
}
