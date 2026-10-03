import {sameViewport} from './viewport.js';
import type { PlaybackPrepareRequest, EventHistory, RationalTime, RasterViewport } from '../../contracts/src/generated/playback-session-request.js';
import type { PlaybackSessionInfo, PlaybackSessionResponse, PlaybackRasterInfo, PlaybackTimingInfo } from '../../contracts/src/generated/playback-session-response.js';
import type { Frame, PlaybackModule, RasterPort, WasmOwner } from './ports.js';
import type {SteppedRasterPort, WasmFrame} from './ports.js';
import {PlaybackExecution} from './execution.js';
import { Owner, PlaybackComputationError, decode, encode, frameSize, initialize, requireResponse, sameBinding, sameTime, takeFrame } from './owner.js';

function response(value: PlaybackSessionResponse, invalidates = false): Exclude<PlaybackSessionResponse, {status: 'error'}> {
  if (value.status === 'error') throw new PlaybackComputationError('author', value.error, invalidates);
  return value;
}

/** One author snapshot and viewport. No component or caller document is retained. */
export class AuthorPlayback {
  #info: PlaybackSessionInfo;
  readonly #owner: Owner<WasmOwner>;
  #size: readonly [number, number];
  private constructor(owner: Owner<WasmOwner>, info: PlaybackSessionInfo, size: readonly [number, number]) {
    this.#owner = owner; this.#info = info; this.#size = size;
  }

  static prepare(module: PlaybackModule, request: PlaybackPrepareRequest): AuthorPlayback {
    return initialize(new module.PlaybackSession(), owner => owner.run(raw => {
      const binding = { ...request.binding }, slide = request.slide, digest = request.snapshot.semanticDigest;
      const viewport = structuredClone(request.viewport);
      const size = [viewport.width, viewport.height] as const;
      const reply = response(decode(raw.command(encode({operation: 'prepare', request}))));
      requireResponse(reply.status === 'prepared' && sameBinding(reply.info.binding, binding) &&
        reply.info.slide === slide && reply.info.documentSha256 === digest &&
        reply.info.viewportRevision === 0 && sameViewport(reply.info.viewport, viewport));
      return new AuthorPlayback(owner, reply.info, size);
    }));
  }
  get info(): PlaybackSessionInfo { return structuredClone(this.#info); }
  get closed(): boolean { return this.#owner.closed; }

  sample(at: RationalTime, raster: RasterPort, history: EventHistory | null = null): Frame<PlaybackRasterInfo> {
    return this.#owner.run(raw => {
      const instant = { ...at };
      const request = encode({operation: 'render', sample: {binding: this.#info.binding, at: instant, history}});
      return this.#readFrame(raw.render(request, raster), instant, false);
    });
  }
  beginSample(at: RationalTime, raster: SteppedRasterPort, history: EventHistory | null = null): PlaybackExecution<PlaybackRasterInfo> {
    const instant = {...at};
    const request = encode({operation: 'render', sample: {binding: this.#info.binding, at: instant, history}});
    return PlaybackExecution.start(this.#owner, request, 'author', raster,
      (frame, invalidates) => this.#readFrame(frame, instant, invalidates));
  }
  #readFrame(frame: WasmFrame, instant: RationalTime, invalidates: boolean): Frame<PlaybackRasterInfo> {
    const result = takeFrame<PlaybackSessionResponse>(frame);
    if (result.response.status === 'error') requireResponse(result.pixels.length === 0);
    const reply = response(result.response, invalidates);
    requireResponse(!invalidates);
    requireResponse(reply.status === 'rendered' && sameBinding(reply.info.frame.state.binding, this.#info.binding) &&
      sameTime(reply.info.frame.state.time, instant));
    const image = reply.info.page.scene.raster;
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
  resize(viewport: RasterViewport): PlaybackSessionInfo {
    return this.#owner.run(raw => {
      const view = structuredClone(viewport);
      const request = encode({operation: 'resize', binding: this.#info.binding,
        expectedViewportRevision: this.#info.viewportRevision, viewport: view});
      const reply = response(decode(raw.command(request)));
      requireResponse(reply.status === 'resized' && sameBinding(reply.info.binding, this.#info.binding) &&
        reply.info.profile === this.#info.profile && reply.info.slide === this.#info.slide &&
        reply.info.documentSha256 === this.#info.documentSha256 &&
        reply.info.viewportRevision === this.#info.viewportRevision + 1 && sameViewport(reply.info.viewport, view));
      this.#info = reply.info;
      this.#size = [view.width, view.height];
      return this.info;
    });
  }
  advance(generation: string): PlaybackSessionInfo {
    return this.#owner.run(raw => {
      const reply = response(decode(raw.command(encode({operation: 'advance', binding: this.#info.binding, generation}))));
      requireResponse(reply.status === 'advanced' &&
        sameBinding(reply.info.binding, {...this.#info.binding, generation}) &&
        reply.info.profile === this.#info.profile && reply.info.planId === this.#info.planId &&
        reply.info.viewportRevision === this.#info.viewportRevision && sameViewport(reply.info.viewport, this.#info.viewport) &&
        reply.info.slide === this.#info.slide && reply.info.documentSha256 === this.#info.documentSha256);
      this.#info = reply.info;
      return this.info;
    });
  }
  /** Validates the core's disposal response and always frees the WASM owner. */
  dispose(): void {
    this.#owner.dispose(raw => {
      const reply = response(decode(raw.command(encode({operation: 'dispose', binding: this.#info.binding}))));
      requireResponse(reply.status === 'disposed' && sameBinding(reply.binding, this.#info.binding));
    });
  }
  close(): void { this.#owner.close(); }
}
