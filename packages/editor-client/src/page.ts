/** Retained, immutable native page geometry. The host owns confirmed document
 * revisions, Worker scheduling and publication; this facade owns one Rust view. */
import type {EditorPagePreparation, EditorPageRequest, PageTextQuery} from '../../contracts/src/generated/editor-page-request.js';
import type {EditorPageInfo, EditorPageResponse, PageTextQueryResult, PptxResourcePageFailure} from '../../contracts/src/generated/editor-page-response.js';
import type {DecoderPort, RasterPort, ShapingPort, WasmFrame} from '../../playback-client/src/ports.js';
import {sameViewport} from '../../playback-client/src/viewport.js';
export type {EditorPagePreparation, EditorPageInfo, PageTextQuery, PageTextQueryResult};
export interface EditorPageOwner {
  prepare(request: string, material: Uint8Array, fonts: Uint8Array,
    decoder: DecoderPort, shaping: ShapingPort, raster: RasterPort): WasmFrame;
  command(request: string): string;
  free(): void;
}
export interface EditorPageModule { EditorPageSession: new () => EditorPageOwner; }
export interface EditorPageInputs {
  material: Uint8Array;
  fonts: Uint8Array;
  decoder: DecoderPort;
  shaping: ShapingPort;
  raster: RasterPort;
}
export class EditorPageComputationError extends Error {
  constructor(readonly diagnostic: PptxResourcePageFailure) {
    super(diagnostic.error.message);
    this.name = 'EditorPageComputationError';
  }
}
function encode(request: EditorPageRequest): string {
  const json = JSON.stringify(request);
  if (new TextEncoder().encode(json).byteLength > 32 * 1024 * 1024) throw new RangeError('Editor request exceeds byte limit');
  return json;
}
function response(json: string): Exclude<EditorPageResponse, {status: 'error'}> {
  const result = JSON.parse(json) as EditorPageResponse;
  if (result.status === 'error') throw new EditorPageComputationError(result.error);
  return result;
}
export class PresentationEditorPage {
  readonly #owner: EditorPageOwner;
  #view: string | null = null;
  #closed = false;
  #busy = false;
  #freed = false;
  constructor(module: EditorPageModule) { this.#owner = new module.EditorPageSession(); }
  get closed(): boolean { return this.#closed; }
  get view(): string | null { return this.#view; }
  #run<T>(call: () => T): T {
    if (this.#closed || this.#busy) throw new Error('Editor page is closed or busy');
    this.#busy = true;
    try {
      const result = call();
      if (this.#closed) throw new Error('Editor page closed during calculation');
      return result;
    }
    catch (error) {
      // A typed calculation rejection preserves the prior immutable page.
      // A bridge trap or malformed reply makes the owner's state uncertain.
      if (!(error instanceof EditorPageComputationError)) this.close();
      throw error;
    } finally {
      this.#busy = false;
      if (this.#closed) { this.#view = null; this.#release(); }
    }
  }
  prepare(request: EditorPagePreparation, inputs: EditorPageInputs): {view: string; info: EditorPageInfo; pixels: Uint8Array} {
    for (const bytes of [inputs.material, inputs.fonts]) {
      if (!(bytes instanceof Uint8Array) || bytes.byteLength > 128 * 1024 * 1024) throw new RangeError('Editor material exceeds byte limit');
    }
    const json = encode({operation: 'prepare', request});
    const viewport = structuredClone(request.page.page.viewport);
    const {width, height} = viewport;
    const {expectedSourceSha256, slide} = request.page.page;
    return this.#run(() => {
      const frame = this.#owner.prepare(json, inputs.material, inputs.fonts, inputs.decoder, inputs.shaping, inputs.raster);
      let consumed = false;
      try {
        const metadata = frame.metadata;
        consumed = true;
        const pixels = frame.take_pixels();
        const raw = JSON.parse(metadata) as EditorPageResponse;
        if (raw.status === 'error' && pixels.length !== 0) throw new Error('Editor error included pixels');
        const reply = response(metadata);
        if (reply.status !== 'prepared' || !/^[0-9a-f]{64}$/.test(reply.view) ||
          !sameViewport(reply.info.viewport, viewport) ||
          reply.info.page.sourceSha256 !== expectedSourceSha256 || reply.info.page.slide !== slide ||
          !Number.isSafeInteger(width * height * 4) || pixels.length !== width * height * 4) {
          throw new Error('Invalid editor page frame');
        }
        this.#view = reply.view;
        return {view: reply.view, info: reply.info, pixels};
      } finally { if (!consumed) frame.free(); }
    });
  }
  query(queries: PageTextQuery[]): PageTextQueryResult[] {
    if (!this.#view) throw new Error('Editor page has not been prepared');
    const view = this.#view;
    const request = encode({operation: 'query', view, queries});
    return this.#run(() => {
      const reply = response(this.#owner.command(request));
      if (reply.status !== 'queried' || reply.view !== view || reply.results.length !== queries.length) {
        throw new Error('Invalid editor page query reply');
      }
      return reply.results;
    });
  }
  clear(): void {
    if (!this.#view) return;
    const view = this.#view;
    this.#run(() => {
      const reply = response(this.#owner.command(encode({operation: 'clear', view})));
      if (reply.status !== 'cleared' || reply.view !== view) throw new Error('Invalid editor page clear reply');
      this.#view = null;
    });
  }
  close(): void {
    this.#closed = true;
    this.#view = null;
    // Component callbacks may close the facade while Rust holds a mutable
    // borrow. Releasing that owner must wait until the outer call returns.
    if (!this.#busy) this.#release();
  }
  #release(): void {
    if (this.#freed) return;
    this.#freed = true;
    this.#owner.free();
  }
}
