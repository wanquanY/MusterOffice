/** Thin typed computation facade. Use in a host Worker; persistence, history
 * membership, authorization, cancellation and publication belong to the host. */
export * from './page.js';
import type { Document, SnapshotRecord, Transaction, HistoryTransaction, TextEditCommand, KernelRequest } from '../../contracts/src/generated/kernel-request.js';
import type { KernelResponse, KernelError } from '../../contracts/src/generated/kernel-response.js';
import type { PagePlacementRequest } from '../../contracts/src/generated/page-placement-request.js';
import type { PagePlacementResponse } from '../../contracts/src/generated/page-placement-response.js';
import type { TextAnalysisResponse } from '../../contracts/src/generated/text-analysis-response.js';
import type { ParagraphInteractionRequest } from '../../contracts/src/generated/paragraph-interaction-request.js';
import type { ParagraphInteractionResponse } from '../../contracts/src/generated/paragraph-interaction-response.js';
import type { ShapingPort } from '../../playback-client/src/ports.js';

export type { Document, SnapshotRecord, Transaction, HistoryTransaction, TextEditCommand, KernelError };
export type PreparedEdit = Extract<KernelResponse, { status: 'prepared' }>;
export type PreparedTextEdit = Extract<KernelResponse, { status: 'textPrepared' }>['result'];
export type PagePlacements = Extract<PagePlacementResponse, { status: 'evaluated' }>['result'];
export type TextSegmentation = Extract<TextAnalysisResponse, { status: 'analyzed' }>['texts'][number];
export type ParagraphInteraction = Extract<ParagraphInteractionResponse, { status: 'evaluated' }>['result'];
export type { ParagraphInteractionRequest };

/** Structural subset of the matching trusted WASM binding, never a remote
 * untrusted transport. The host verifies the module's release manifest. */
export interface EditorModule {
  dispatch_json(request: string): string;
  page_placements(request: string): string;
  analyze_text(request: string): string;
  paragraph_interaction(request: string, fonts: Uint8Array, shaping: ShapingPort): string;
}

export class EditorComputationError extends Error {
  constructor(readonly diagnostic: KernelError | { code: string; message: string }) {
    super(diagnostic.message);
    this.name = 'EditorComputationError';
  }
}

export class PresentationEditor {
  constructor(private readonly kernel: EditorModule) {}

  initialize(document: Document): SnapshotRecord {
    const result = this.dispatch({ operation: 'initialize', document });
    if (result.status !== 'initialized') throw new Error('Unexpected initialization reply');
    return result.snapshot;
  }

  prepare(snapshot: SnapshotRecord, transaction: Transaction): PreparedEdit {
    const result = this.dispatch({ operation: 'prepare', snapshot, transaction });
    if (result.status !== 'prepared') throw new Error('Unexpected preparation reply');
    return result;
  }

  /** Reversal is a fresh CAS candidate, never a restored old current snapshot.
   * The authorized host supplies original material from its durable history. */
  prepareHistory(snapshot: SnapshotRecord, transaction: HistoryTransaction): PreparedEdit {
    const result = this.dispatch({ operation: 'prepareHistory', snapshot, transaction });
    if (result.status !== 'prepared') throw new Error('Unexpected history reply');
    return result;
  }

  prepareText(snapshot: SnapshotRecord, command: TextEditCommand): PreparedTextEdit {
    const result = this.dispatch({ operation: 'prepareText', snapshot, command });
    if (result.status !== 'textPrepared') throw new Error('Unexpected text preparation reply');
    return result.result;
  }

  placements(request: PagePlacementRequest): PagePlacements {
    const result = JSON.parse(this.kernel.page_placements(JSON.stringify(request))) as PagePlacementResponse;
    if (result.status === 'error') throw new EditorComputationError(result.error);
    if (result.status !== 'evaluated') throw new Error('Unexpected placement reply');
    return result.result;
  }

  /** Uses the kernel's pinned Unicode tables. Boundaries contain scalar,
   * UTF-8 and UTF-16 offsets; the product never splits a surrogate pair. */
  segment(text: string): TextSegmentation {
    const result = JSON.parse(this.kernel.analyze_text(JSON.stringify({texts: [text], characters: []}))) as TextAnalysisResponse;
    if (result.status === 'error') throw new EditorComputationError(result);
    if (result.status !== 'analyzed' || result.texts.length !== 1 || !result.texts[0]) {
      throw new Error('Unexpected segmentation reply');
    }
    return result.texts[0];
  }

  /** Paragraph-local caret, selection and hit geometry from the renderer's
   * exact line plan and glyph pen. Font bytes are explicit verified resources. */
  paragraphInteraction(request: ParagraphInteractionRequest, fonts: Uint8Array, shaping: ShapingPort): ParagraphInteraction {
    const result = JSON.parse(this.kernel.paragraph_interaction(JSON.stringify(request), fonts, shaping)) as ParagraphInteractionResponse;
    if (result.status === 'error') throw new EditorComputationError(result.error);
    if (result.status !== 'evaluated') throw new Error('Unexpected paragraph interaction reply');
    return result.result;
  }

  private dispatch(request: KernelRequest): Exclude<KernelResponse, { status: 'error' }> {
    const result = JSON.parse(this.kernel.dispatch_json(JSON.stringify(request))) as KernelResponse;
    if (result.status === 'error') throw new EditorComputationError(result.error);
    return result;
  }
}
