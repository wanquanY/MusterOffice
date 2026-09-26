/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */
import type { ByteLength, ColorSlot, Digest, Emu, NativeBlackWhiteMode, NativeBlipCompression, NativeCompoundLine, NativeCoordinate, NativeLineCap, NativeLineEnd, NativeLineEndSize, NativePathFill, NativePattern, NativePenAlignment, NativePercentage, NativeTileFlip, PlaceholderKind, PlaceholderOrientation, PlaceholderSize, SourceAdjustHandle, SourceBackgroundDefinition, SourceColorMapping, SourceColorTransform, SourceColorValue, SourceEffectDefinition, SourceEffectPropertiesDefinition, SourceFillDefinition, SourceGeometryCommand, SourceGeometryDefinition, SourceGradientShade, SourceLineDash, SourceLineFill, SourceLineJoin, SourceObjectKind, SourceRunKind, SourceTextConstraint, SurfaceKind } from './part-001.js';

export type SourcePlaceholderMatch =
  | {
      status: "notPlaceholder";
    }
  | {
      status: "master";
    }
  | {
      rule: PlaceholderMatchRule;
      status: "matched";
      target: SourceObjectRef1;
    }
  | {
      status: "unmatched";
    }
  | {
      status: "detached";
    }
  | {
      candidates: number;
      part: string;
      status: "ambiguous";
    }
  | {
      status: "unsupportedContext";
    };

export type PlaceholderMatchRule = "slideIndex" | "masterType";

export type SourceVisualIssueKind = "element" | "attribute";

export type SourceTextValue =
  | {
      kind: "container";
    }
  | {
      attributes: SourceTextBodyAttributes;
      kind: "body";
    }
  | {
      attributes: SourceTextParagraphAttributes;
      kind: "paragraph";
    }
  | {
      attributes: SourceTextCharacterAttributes;
      kind: "character";
    }
  | {
      font: SourceTextFont;
      kind: "font";
    }
  | {
      index: NativeFontCollectionIndex;
      kind: "fontReference";
    }
  | {
      fontScale?: NativePercentage | null;
      kind: "autofit";
      lineSpacingReduction?: NativePercentage | null;
    }
  | {
      kind: "percentage";
      value: NativePercentage;
    }
  | {
      kind: "points";
      value: number;
    }
  | {
      kind: "autoNumber";
      scheme: NativeTextAutonumber;
      startAt?: number | null;
    }
  | {
      character: string;
      kind: "bulletCharacter";
    }
  | {
      alignment?: NativeTextTabAlign | null;
      kind: "tab";
      position?: NativeCoordinate | null;
    }
  | {
      fieldType?: string | null;
      id: string;
      kind: "field";
    }
  | {
      attributes: SourceTextHyperlinkAttributes;
      kind: "hyperlink";
    }
  | {
      kind: "rightToLeft";
      value?: boolean | null;
    }
  | {
      fill: SourceFill;
      kind: "fill";
    }
  | {
      kind: "line";
      line: SourceLine;
    }
  | {
      effects: SourceEffectProperties;
      kind: "effects";
    }
  | {
      color: SourceColor;
      kind: "color";
    };

export type NativeTextAnchor = "t" | "ctr" | "b" | "just" | "dist";

export type NativeTextHorizontalOverflow = "overflow" | "clip";

export type NativeTextVertical =
  "horz" | "vert" | "vert270" | "wordArtVert" | "eaVert" | "mongolianVert" | "wordArtVertRtl";

export type NativeTextVerticalOverflow = "overflow" | "ellipsis" | "clip";

export type NativeTextWrap = "none" | "square";

export type NativeTextAlign = "l" | "ctr" | "r" | "just" | "justLow" | "dist" | "thaiDist";

export type NativeTextFontAlign = "auto" | "t" | "ctr" | "base" | "b";

export type NativeTextCaps = "none" | "small" | "all";

export type NativeTextPoint =
  | {
      kind: "hundredthPoints";
      value: number;
    }
  | {
      kind: "universalMeasure";
      value: NativeCoordinate;
    };

export type NativeTextStrike = "noStrike" | "sngStrike" | "dblStrike";

export type NativeTextUnderline =
  | "none"
  | "words"
  | "sng"
  | "dbl"
  | "heavy"
  | "dotted"
  | "dottedHeavy"
  | "dash"
  | "dashHeavy"
  | "dashLong"
  | "dashLongHeavy"
  | "dotDash"
  | "dotDashHeavy"
  | "dotDotDash"
  | "dotDotDashHeavy"
  | "wavy"
  | "wavyHeavy"
  | "wavyDbl";

export type NativeFontCollectionIndex = "major" | "minor" | "none";

export type NativeTextAutonumber =
  | "alphaLcParenBoth"
  | "alphaUcParenBoth"
  | "alphaLcParenR"
  | "alphaUcParenR"
  | "alphaLcPeriod"
  | "alphaUcPeriod"
  | "arabicParenBoth"
  | "arabicParenR"
  | "arabicPeriod"
  | "arabicPlain"
  | "romanLcParenBoth"
  | "romanUcParenBoth"
  | "romanLcParenR"
  | "romanUcParenR"
  | "romanLcPeriod"
  | "romanUcPeriod"
  | "circleNumDbPlain"
  | "circleNumWdBlackPlain"
  | "circleNumWdWhitePlain"
  | "arabicDbPeriod"
  | "arabicDbPlain"
  | "ea1ChsPeriod"
  | "ea1ChsPlain"
  | "ea1ChtPeriod"
  | "ea1ChtPlain"
  | "ea1JpnChsDbPeriod"
  | "ea1JpnKorPlain"
  | "ea1JpnKorPeriod"
  | "arabic1Minus"
  | "arabic2Minus"
  | "hebrew2Minus"
  | "thaiAlphaPeriod"
  | "thaiAlphaParenR"
  | "thaiAlphaParenBoth"
  | "thaiNumPeriod"
  | "thaiNumParenR"
  | "thaiNumParenBoth"
  | "hindiAlphaPeriod"
  | "hindiNumPeriod"
  | "hindiNumParenR"
  | "hindiAlpha1Period";

export type NativeTextTabAlign = "l" | "ctr" | "r" | "dec";

export type SourceThemeKind = "theme" | "override";

export type PptxFailureCode =
  | "INPUT_INVALID"
  | "SOURCE_CONFLICT"
  | "PRESERVATION_CONFLICT"
  | "MAPPING_NOT_IMPLEMENTED"
  | "RESOURCE_REQUIRED"
  | "LIMIT_EXCEEDED"
  | "CANCELLED"
  | "READ_FAILED";

export interface SourceIndex {
  byteLength: ByteLength;
  compatibilityProfile: string;
  containsSignatures: boolean;
  mainCompatibility: SourceCompatibility;
  mainContentType: string;
  mainPart: string;
  /**
   * Unresolved constructs are reported, never counted as semantic support.
   */
  notices: string[];
  /**
   * Explicit native dimensions, absent if the source omits sldSz.
   */
  pageSize?: Size | null;
  slides: SourceSlide[];
  sourceSha256: Digest;
  surfaces: {
    [k: string]: SourceSurface | undefined;
  };
  text?: SourceTextCatalog;
  themes: {
    [k: string]: SourceThemePart | undefined;
  };
}

export interface SourceCompatibility {
  ignoredAttributes: number;
  ignoredElements: number;
  selections: SourceCompatibilitySelection[];
  unwrappedElements: number;
}

export interface SourceCompatibilitySelection {
  branches: SourceCompatibilityBranch[];
  sourceOrdinal: number;
}

export interface SourceCompatibilityBranch {
  fallback: boolean;
  requires: string[];
  selected: boolean;
  sourceOrdinal: number;
}

export interface Size {
  height: Emu;
  width: Emu;
}

export interface SourceSlide {
  nativeId: number;
  part: string;
}

export interface SourceSurface {
  background?: SourceBackground | null;
  colorMapping?: SourceColorMapping | null;
  compatibility: SourceCompatibility;
  effectNodes?: {
    [k: string]: SourceEffectNode | undefined;
  };
  effectiveTheme: SourceThemeStack;
  hidden: boolean;
  kind: SurfaceKind;
  links: SourceSurfaceLinks;
  name?: string | null;
  notices: string[];
  /**
   * Preorder paint-tree objects; IDs are part-scoped, not domain/global IDs.
   */
  objects: SourceObject[];
  /**
   * Closest explicit map in the owning slide/layout/master hierarchy.
   */
  resolvedColorMapping?: SourceColorMapRef | null;
  rootGroupEffects?: SourceEffectProperties | null;
  rootGroupFill?: SourceFill | null;
  /**
   * Shape-tree declaration; preserved independently from child transforms.
   */
  rootGroupTransform?: SourceTransform | null;
  rootObjectId: number;
  sha256: Digest;
  showMasterShapes?: boolean | null;
  text?: SourceTextCatalog;
  /**
   * These block text mutation until reference/compatibility semantics exist.
   */
  textEditBarriers: string[];
  themeSelection: SourceThemeSelection;
  /**
   * Visual declarations not yet projected by a semantic source reader.
   */
  visualIssues?: SourceVisualIssue[];
}

export interface SourceBackground {
  blackWhiteMode?: NativeBlackWhiteMode | null;
  definition: SourceBackgroundDefinition;
  retainedOrdinals: number[];
  sourceOrdinal: number;
}

export interface SourceEffectProperties {
  definition: SourceEffectPropertiesDefinition;
  retainedOrdinals: number[];
  sourceOrdinal: number;
}

export interface SourceFill {
  definition: SourceFillDefinition;
  /**
   * Physical nodes in the immutable source part. Consumers must resolve or
   * diagnose these before claiming a complete brush.
   */
  retainedOrdinals: number[];
  sourceOrdinal: number;
}

export interface SourceColor {
  sourceOrdinal: number;
  /**
   * XML application order, including repeated transforms.
   */
  transforms: SourceColorTransform[];
  value: SourceColorValue;
}

export interface SourceFillRect {
  bottom?: NativePercentage | null;
  left?: NativePercentage | null;
  right?: NativePercentage | null;
  sourceOrdinal: number;
  top?: NativePercentage | null;
}

export interface SourceGradientStops {
  /**
   * Native order is significant, including repeated stop positions.
   */
  entries: SourceGradientStop[];
  sourceOrdinal: number;
}

export interface SourceGradientStop {
  color: SourceColor;
  position: NativePercentage;
  sourceOrdinal: number;
}

export interface SourceFillColor {
  color: SourceColor;
  sourceOrdinal: number;
}

export interface SourceFillBlip {
  compression?: NativeBlipCompression | null;
  effectNodes?: number[];
  /**
   * Relationship IDs only: the core never opens a network URL or file path.
   */
  embed?: string | null;
  link?: string | null;
  /**
   * Uninterpreted properties only; known effects use the part catalog.
   */
  retainedOrdinals: number[];
  sourceOrdinal: number;
}

export interface SourceColorMap {
  accent1: ColorSlot;
  accent2: ColorSlot;
  accent3: ColorSlot;
  accent4: ColorSlot;
  accent5: ColorSlot;
  accent6: ColorSlot;
  bg1: ColorSlot;
  bg2: ColorSlot;
  folHlink: ColorSlot;
  hlink: ColorSlot;
  tx1: ColorSlot;
  tx2: ColorSlot;
}

/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^\d+$".
 *
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^\d+$".
 *
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^\d+$".
 */
export interface SourceEffectNode {
  definition: SourceEffectDefinition;
  /**
   * Only this node's uninterpreted properties. Children have their own nodes.
   */
  retainedOrdinals: number[];
  sourceOrdinal: number;
}

export interface SourceThemeStack {
  base?: SourcePartRef | null;
  /**
   * Ordered from master toward slide. Parts remain source-bound; colors,
   * fonts and effects are not flattened or resolved by this stack alone.
   */
  overrides: SourcePartRef[];
}

export interface SourcePartRef {
  part: string;
  sha256: Digest;
}

export interface SourceSurfaceLinks {
  layout?: string | null;
  master?: string | null;
  theme?: SourcePartRef | null;
  themeOverride?: SourcePartRef | null;
}

export interface SourceObject {
  effectReference?: SourceEffectReference | null;
  effects?: SourceEffectProperties | null;
  fill?: SourceFill | null;
  fillReference?: SourceFillReference | null;
  geometry?: SourceGeometry | null;
  hidden?: boolean | null;
  kind: SourceObjectKind;
  /**
   * Direct declaration only. Absence is distinct from an empty a:ln.
   */
  line?: SourceLine | null;
  lineReference?: SourceLineReference | null;
  name: string;
  nativeId: number;
  paragraphs: SourceRun[][];
  parentGroup?: number | null;
  /**
   * Picture image content is independent of the shape properties fill.
   */
  pictureFill?: SourceFill | null;
  placeholder?: SourcePlaceholder | null;
  resolution: SourceObjectResolution;
  textBodyOrdinal?: number | null;
  transform?: SourceTransform | null;
  useBackgroundFill?: SourceBackgroundFillUsage | null;
  visualIssues?: SourceVisualIssue[];
}

export interface SourceEffectReference {
  color?: SourceColor | null;
  index: number;
  retainedOrdinals: number[];
  sourceOrdinal: number;
}

export interface SourceFillReference {
  color?: SourceColor | null;
  /**
   * 0/1000 mean no fill; 1..999 select fills; 1001+ select background fills.
   * Selection and inherited color substitution belong to resolution.
   */
  index: number;
  retainedOrdinals: number[];
  sourceOrdinal: number;
}

export interface SourceGeometry {
  definition: SourceGeometryDefinition;
  /**
   * Unknown properties retain physical bindings and block complete evaluation.
   */
  retainedOrdinals: number[];
  sourceOrdinal: number;
}

export interface SourceGeometryList {
  /**
   * Native order, including valid empty lists and duplicate guide names.
   */
  entries: SourceGuide[];
  sourceOrdinal: number;
}

export interface SourceGuide {
  /**
   * Original formula text. Syntax/name resolution belongs to evaluation.
   */
  formula: string;
  name: string;
  sourceOrdinal: number;
}

export interface SourceGeometryList3 {
  /**
   * Native order, including valid empty lists and duplicate guide names.
   */
  entries: SourceConnectionSite[];
  sourceOrdinal: number;
}

export interface SourceConnectionSite {
  angle: string;
  position: SourceGeometryPoint;
  sourceOrdinal: number;
}

export interface SourceGeometryPoint {
  sourceOrdinal: number;
  /**
   * ST_AdjCoordinate union: native lexical coordinates or guide names.
   */
  x: string;
  y: string;
}

export interface SourceGeometryList2 {
  /**
   * Native order, including valid empty lists and duplicate guide names.
   */
  entries: SourceAdjustHandle[];
  sourceOrdinal: number;
}

export interface SourceGeometryList4 {
  /**
   * Native order, including valid empty lists and duplicate guide names.
   */
  entries: SourceGeometryPath[];
  sourceOrdinal: number;
}

export interface SourceGeometryPath {
  commands: SourceGeometryCommand[];
  extrusionOk?: boolean | null;
  fill?: NativePathFill | null;
  height?: Emu | null;
  sourceOrdinal: number;
  stroke?: boolean | null;
  /**
   * Absence and explicit zero remain distinct author declarations.
   */
  width?: Emu | null;
}

export interface SourceGeometryRect {
  bottom: string;
  left: string;
  right: string;
  sourceOrdinal: number;
  top: string;
}

export interface SourceLine {
  alignment?: NativePenAlignment | null;
  cap?: NativeLineCap | null;
  compound?: NativeCompoundLine | null;
  dash?: SourceLineDash | null;
  fill?: SourceLineFill | null;
  head?: SourceLineEnd | null;
  join?: SourceLineJoin | null;
  /**
   * Unsupported attributes/extensions bind to their owning elements. They
   * cannot be discarded or counted as resolved line semantics.
   */
  retainedOrdinals: number[];
  sourceOrdinal: number;
  tail?: SourceLineEnd | null;
  width?: Emu | null;
}

export interface SourceDashStop {
  dash: NativePercentage;
  sourceOrdinal: number;
  space: NativePercentage;
}

export interface SourceGradientFill {
  flip?: NativeTileFlip | null;
  rotateWithShape?: boolean | null;
  shade?: SourceGradientShade | null;
  stops?: SourceGradientStops | null;
  tileRect?: SourceFillRect | null;
}

export interface SourcePatternFill {
  background?: SourceFillColor | null;
  foreground?: SourceFillColor | null;
  preset?: NativePattern | null;
}

export interface SourceLineEnd {
  kind?: NativeLineEnd | null;
  length?: NativeLineEndSize | null;
  sourceOrdinal: number;
  width?: NativeLineEndSize | null;
}

export interface SourceLineReference {
  color?: SourceColor | null;
  index: number;
  retainedOrdinals: number[];
  sourceOrdinal: number;
}

export interface SourceRun {
  editConstraint?: SourceTextConstraint | null;
  /**
   * Only ordinary run text with a concrete a:t source binding is editable.
   */
  editable: boolean;
  kind: SourceRunKind;
  text: string;
}

export interface SourcePlaceholder {
  customPrompt?: boolean | null;
  index?: number | null;
  /**
   * None preserves absent author attributes, distinct from explicit defaults.
   */
  kind?: PlaceholderKind | null;
  orientation?: PlaceholderOrientation | null;
  size?: PlaceholderSize | null;
}

export interface SourceObjectResolution {
  origin?: SourceResolvedValue | null;
  placeholderMatch: SourcePlaceholderMatch;
  size?: SourceResolvedValue2 | null;
}

export interface SourceResolvedValue {
  declaredBy: SourceObjectRef;
  value: Point;
}

/**
 * Ultimate explicit declaration, not merely the next inheritance hop.
 */
export interface SourceObjectRef {
  nativeId: number;
  part: string;
}

export interface Point {
  x: Emu;
  y: Emu;
}

export interface SourceObjectRef1 {
  nativeId: number;
  part: string;
}

export interface SourceResolvedValue2 {
  declaredBy: SourceObjectRef2;
  value: Size;
}

/**
 * Ultimate explicit declaration, not merely the next inheritance hop.
 */
export interface SourceObjectRef2 {
  nativeId: number;
  part: string;
}

export interface SourceTransform {
  childOrigin?: Point | null;
  childSize?: Size | null;
  flipHorizontal?: boolean | null;
  flipVertical?: boolean | null;
  origin?: Point | null;
  retainedOrdinals?: number[];
  /**
   * Explicit native value, not normalized or resolved through groups.
   */
  rotation?: number | null;
  size?: Size | null;
}

export interface SourceBackgroundFillUsage {
  sourceOrdinal: number;
  value: boolean;
}

export interface SourceVisualIssue {
  kind: SourceVisualIssueKind;
  localName: string;
  namespace: string;
  sourceOrdinal: number;
}

export interface SourceColorMapRef {
  part: string;
  sourceOrdinal: number;
}

export interface SourceTextCatalog {
  effectNodes: {
    [k: string]: SourceEffectNode | undefined;
  };
  nodes: {
    [k: string]: SourceTextNode | undefined;
  };
  /**
   * Physical ordinals in this part, in discovery order. A root is a txBody,
   * txStyles, defaultTextStyle, fontRef or a theme bodyPr/lstStyle root.
   * Relationships are resolved later.
   */
  roots: SourceTextRoot[];
}

/**
 * This interface was referenced by `undefined`'s JSON-Schema definition
 * via the `patternProperty` "^\d+$".
 */
export interface SourceTextNode {
  children: number[];
  /**
   * Native local name. Reader grammar validates its namespace and context.
   */
  element:
    | "txBody"
    | "txStyles"
    | "defaultTextStyle"
    | "titleStyle"
    | "bodyStyle"
    | "otherStyle"
    | "lstStyle"
    | "bodyPr"
    | "p"
    | "pPr"
    | "defPPr"
    | "lvl1pPr"
    | "lvl2pPr"
    | "lvl3pPr"
    | "lvl4pPr"
    | "lvl5pPr"
    | "lvl6pPr"
    | "lvl7pPr"
    | "lvl8pPr"
    | "lvl9pPr"
    | "r"
    | "br"
    | "fld"
    | "t"
    | "rPr"
    | "defRPr"
    | "endParaRPr"
    | "noAutofit"
    | "normAutofit"
    | "spAutoFit"
    | "lnSpc"
    | "spcBef"
    | "spcAft"
    | "spcPct"
    | "spcPts"
    | "buClrTx"
    | "buClr"
    | "buSzTx"
    | "buSzPct"
    | "buSzPts"
    | "buFontTx"
    | "buFont"
    | "buNone"
    | "buAutoNum"
    | "buChar"
    | "tabLst"
    | "tab"
    | "latin"
    | "ea"
    | "cs"
    | "sym"
    | "fontRef"
    | "highlight"
    | "uLnTx"
    | "uLn"
    | "uFillTx"
    | "uFill"
    | "hlinkClick"
    | "hlinkMouseOver"
    | "rtl"
    | "noFill"
    | "solidFill"
    | "gradFill"
    | "blipFill"
    | "pattFill"
    | "grpFill"
    | "ln"
    | "effectLst"
    | "effectDag"
    | "srgbClr"
    | "scrgbClr"
    | "hslClr"
    | "sysClr"
    | "schemeClr"
    | "prstClr";
  parent?: number | null;
  /**
   * Uninterpreted attributes/subtrees cannot become resolved text semantics.
   */
  retainedOrdinals: number[];
  value: SourceTextValue;
}

export interface SourceTextBodyAttributes {
  anchor?: NativeTextAnchor | null;
  bottomInset?: NativeCoordinate | null;
  centerAnchor?: boolean | null;
  columnSpacing?: NativeCoordinate | null;
  columns?: number | null;
  compatibleLineSpacing?: boolean | null;
  forceAntialiasing?: boolean | null;
  fromWordArt?: boolean | null;
  horizontalOverflow?: NativeTextHorizontalOverflow | null;
  leftInset?: NativeCoordinate | null;
  paragraphSpacing?: boolean | null;
  rightInset?: NativeCoordinate | null;
  rightToLeftColumns?: boolean | null;
  rotation?: number | null;
  topInset?: NativeCoordinate | null;
  upright?: boolean | null;
  vertical?: NativeTextVertical | null;
  verticalOverflow?: NativeTextVerticalOverflow | null;
  wrap?: NativeTextWrap | null;
}

export interface SourceTextParagraphAttributes {
  alignment?: NativeTextAlign | null;
  defaultTabSize?: NativeCoordinate | null;
  eastAsianLineBreak?: boolean | null;
  fontAlignment?: NativeTextFontAlign | null;
  hangingPunctuation?: boolean | null;
  indent?: number | null;
  latinLineBreak?: boolean | null;
  leftMargin?: number | null;
  level?: number | null;
  rightMargin?: number | null;
  rightToLeft?: boolean | null;
}

export interface SourceTextCharacterAttributes {
  alternativeLanguage?: string | null;
  baseline?: NativePercentage | null;
  bold?: boolean | null;
  bookmark?: string | null;
  caps?: NativeTextCaps | null;
  dirty?: boolean | null;
  error?: boolean | null;
  italic?: boolean | null;
  kerning?: number | null;
  kumimoji?: boolean | null;
  language?: string | null;
  noProof?: boolean | null;
  normalizeHeight?: boolean | null;
  size?: number | null;
  smartClean?: boolean | null;
  smartId?: number | null;
  spacing?: NativeTextPoint | null;
  strike?: NativeTextStrike | null;
  underline?: NativeTextUnderline | null;
}

export interface SourceTextFont {
  charset?: number | null;
  panose?: string | null;
  pitchFamily?: number | null;
  typeface: string;
}

export interface SourceTextHyperlinkAttributes {
  action?: string | null;
  endSound?: boolean | null;
  highlightClick?: boolean | null;
  history?: boolean | null;
  invalidUrl?: string | null;
  relationshipId?: string | null;
  targetFrame?: string | null;
  tooltip?: string | null;
}

export interface SourceTextRoot {
  /**
   * Physical shape ID in this part; None for presentation/master defaults.
   */
  owner?: number | null;
  sourceOrdinal: number;
}

export interface SourceThemeSelection {
  colors?: SourceThemeSchemeRef | null;
  fonts?: SourceThemeSchemeRef | null;
  format?: SourceThemeSchemeRef | null;
}

export interface SourceThemeSchemeRef {
  part: string;
  sourceOrdinal: number;
}

export interface SourceThemePart {
  colorScheme?: SourceColorScheme | null;
  compatibility: SourceCompatibility;
  effectNodes?: {
    [k: string]: SourceEffectNode | undefined;
  };
  fontScheme?: SourceFontScheme | null;
  formatScheme?: SourceFormatScheme | null;
  kind: SourceThemeKind;
  name?: string | null;
  notices: string[];
  sha256: Digest;
  textDefaults?: SourceThemeTextDefaults | null;
}

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

export interface SourceFontCollection {
  complexScript?: SourceTextFont | null;
  eastAsian?: SourceTextFont | null;
  latin?: SourceTextFont | null;
  /**
   * Preserve order and duplicate declarations for later font-profile policy.
   */
  supplemental: SourceSupplementalFont[];
}

export interface SourceSupplementalFont {
  script: string;
  typeface: string;
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
