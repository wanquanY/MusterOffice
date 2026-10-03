/* Generated from Rust → JSON Schema → TypeScript. Do not edit. Draft computation contracts only. */
import type { ItemizationNoticeKind, TimelineFailureCode } from './part-002.js';

export interface SourceThemeSchemeRef {
  part: string;
  sourceOrdinal: number;
}

export interface ItemizationNotice {
  end: number;
  kind: ItemizationNoticeKind;
  start: number;
}

export interface EffectiveVariation {
  /**
   * Effective IEEE754 binary32 design coordinate. Rounding is explicit.
   */
  effectiveF32Bits: number;
  requested1616: number;
  tag: string;
}

export interface TimelineFailure {
  code: TimelineFailureCode;
  message: string;
}
