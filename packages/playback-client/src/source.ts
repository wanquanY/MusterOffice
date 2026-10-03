import {sameViewport} from './viewport.js';
import type { PptxPlaybackPrepareRequest, EventHistory, RationalTime, RasterViewport } from '../../contracts/src/generated/pptx-playback-session-request.js';
import type { PptxPlaybackSessionInfo, PptxPlaybackSessionResponse, PptxPlaybackRasterInfo, PlaybackTimingInfo } from '../../contracts/src/generated/pptx-playback-session-response.js';
import type { DecoderPort, Frame, PlaybackModule, RasterPort, ShapingPort, WasmSourceOwner } from './ports.js';
import type {SteppedRasterPort, WasmFrame} from './ports.js';
import {PlaybackExecution} from './execution.js';
import { Owner, PlaybackComputationError, decode, encode, frameSize, initialize, inputBytes, requireResponse, sameBinding, sameTime, takeFrame } from './owner.js';

function response(value: PptxPlaybackSessionResponse, invalidates = false): Exclude<PptxPlaybackSessionResponse, {status: 'error'}> {
  if (value.status === 'error') throw new PlaybackComputationError('source', value.error, invalidates);
  return value;
}
export interface SourceInputs {
  source: Uint8Array;
  fonts: Uint8Array;
  decoder: DecoderPort;
  shaping: ShapingPort;
}

/** Owns the prepared Rust plan, never the original buffers or component ports. */
export class SourcePlayback {
  #info: PptxPlaybackSessionInfo;
  readonly #owner: Owner<WasmSourceOwner>;
  #size: readonly [number, number];
  private constructor(owner: Owner<WasmSourceOwner>, info: PptxPlaybackSessionInfo, size: readonly [number, number]) {
    this.#owner = owner; this.#info = info; this.#size = size;
  }

  static prepare(module: PlaybackModule, request: PptxPlaybackPrepareRequest, inputs: SourceInputs): SourcePlayback {
    return initialize(new module.PptxPlaybackSession(), owner => owner.run(raw => {
      const {source, fonts, decoder, shaping} = inputs;
      inputBytes(source); inputBytes(fonts);
      const binding = { ...request.binding }, slide = request.page.page.slide;
      const digest = request.page.page.expectedSourceSha256;
      const viewport = structuredClone(request.page.page.viewport);
      const size = [viewport.width, viewport.height] as const;
      const reply = response(decode(raw.prepare(encode({operation: 'prepare', request}),
        source, fonts, decoder, shaping)));
      requireResponse(reply.status === 'prepared' && sameBinding(reply.info.binding, binding) &&
        reply.info.slide === slide && reply.info.sourceSha256 === digest &&
        reply.info.viewportRevision === 0 && sameViewport(reply.info.viewport, viewport));
      return new SourcePlayback(owner, reply.info, size);
    }));
  }
  get info(): PptxPlaybackSessionInfo { return structuredClone(this.#info); }
  get closed(): boolean { return this.#owner.closed; }

  sample(at: RationalTime, raster: RasterPort, history: EventHistory | null = null): Frame<PptxPlaybackRasterInfo> {
    return this.#owner.run(raw => {
      const instant = { ...at };
      const request = encode({operation: 'render', sample: {binding: this.#info.binding, at: instant, history}});
      return this.#readFrame(raw.render(request, raster), instant, false);
    });
  }
  beginSample(at: RationalTime, raster: SteppedRasterPort, history: EventHistory | null = null): PlaybackExecution<PptxPlaybackRasterInfo> {
    const instant = {...at};
    const request = encode({operation: 'render', sample: {binding: this.#info.binding, at: instant, history}});
    return PlaybackExecution.start(this.#owner, request, 'source', raster,
      (frame, invalidates) => this.#readFrame(frame, instant, invalidates));
  }
  #readFrame(frame: WasmFrame, instant: RationalTime, invalidates: boolean): Frame<PptxPlaybackRasterInfo> {
    const result = takeFrame<PptxPlaybackSessionResponse>(frame);
    if (result.response.status === 'error') requireResponse(result.pixels.length === 0);
    const reply = response(result.response, invalidates);
    requireResponse(!invalidates);
    requireResponse(reply.status === 'rendered' &&
      sameBinding(reply.info.playback.evaluated.state.binding, this.#info.binding) &&
      sameTime(reply.info.playback.evaluated.state.time, instant) &&
      reply.info.playback.sourceSha256 === this.#info.sourceSha256 && reply.info.playback.slide === this.#info.slide);
    const image = reply.info.page.page.scene.raster;
    frameSize(image.width, image.height, image.byteLength, result.pixels, this.#size);
    return {info: reply.info, pixels: result.pixels, viewportRevision: this.#info.viewportRevision};
  }
  timing(): PlaybackTimingInfo {
    return this.#owner.run(raw => {
      const reply = response(decode(raw.command(encode({operation: 'inspectTiming', binding: this.#info.binding}))));
      requireResponse(reply.status === 'timingInspected' && sameBinding(reply.info.binding, this.#info.binding));
      return reply.info;
    });
  }
  /** Changes only the view; playback generation, time and input history remain valid. */
  resize(viewport: RasterViewport, inputs: Pick<SourceInputs, 'source' | 'decoder'>): PptxPlaybackSessionInfo {
    return this.#owner.run(raw => {
      const {source, decoder} = inputs;
      inputBytes(source);
      const view = structuredClone(viewport);
      const request = encode({operation: 'resize', binding: this.#info.binding,
        expectedViewportRevision: this.#info.viewportRevision, viewport: view});
      const reply = response(decode(raw.resize(request, source, decoder)));
      requireResponse(reply.status === 'resized' && sameBinding(reply.info.binding, this.#info.binding) &&
        reply.info.profile === this.#info.profile && reply.info.slide === this.#info.slide &&
        reply.info.sourceSha256 === this.#info.sourceSha256 &&
        reply.info.viewportRevision === this.#info.viewportRevision + 1 && sameViewport(reply.info.viewport, view));
      this.#info = reply.info;
      this.#size = [view.width, view.height];
      return this.info;
    });
  }
  advance(generation: string): PptxPlaybackSessionInfo {
    return this.#owner.run(raw => {
      const reply = response(decode(raw.command(encode({operation: 'advance', binding: this.#info.binding, generation}))));
      requireResponse(reply.status === 'advanced' &&
        sameBinding(reply.info.binding, {...this.#info.binding, generation}) &&
        reply.info.profile === this.#info.profile && reply.info.planId === this.#info.planId &&
        reply.info.viewportRevision === this.#info.viewportRevision && sameViewport(reply.info.viewport, this.#info.viewport) &&
        reply.info.slide === this.#info.slide && reply.info.sourceSha256 === this.#info.sourceSha256);
      this.#info = reply.info;
      return this.info;
    });
  }
  dispose(): void {
    this.#owner.dispose(raw => {
      const reply = response(decode(raw.command(encode({operation: 'dispose', binding: this.#info.binding}))));
      requireResponse(reply.status === 'disposed' && sameBinding(reply.binding, this.#info.binding));
    });
  }
  close(): void { this.#owner.close(); }
}
