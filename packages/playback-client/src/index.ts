import type { PlaybackPrepareRequest } from '../../contracts/src/generated/playback-session-request.js';
import type { PptxPlaybackPrepareRequest } from '../../contracts/src/generated/pptx-playback-session-request.js';
import type { PlaybackModule } from './ports.js';
import { AuthorPlayback } from './author.js';
import { SourcePlayback, type SourceInputs } from './source.js';
import { prepareDeliveryInputs, type DeliveryPlaybackRequest, type DeliveryPlaybackInputs } from './delivery.js';
export { DeliveryPlaybackError } from './delivery.js';
export type { DeliveryPlaybackRequest, DeliveryPlaybackInputs } from './delivery.js';
export { AuthorPlayback, SourcePlayback };
export { PlaybackExecution } from './execution.js';
export type { SourceInputs };
export { PlaybackComputationError, PlaybackStateError } from './owner.js';
export type { PlaybackModule, RasterPort, SteppedRasterPort, DecoderPort, ShapingPort, Frame } from './ports.js';
export type { PlaybackPrepareRequest, EventHistory, PlaybackBinding, RationalTime } from '../../contracts/src/generated/playback-session-request.js';
export type { PptxPlaybackPrepareRequest } from '../../contracts/src/generated/pptx-playback-session-request.js';
export type { PlaybackSessionInfo, PlaybackTimingInfo, PlaybackRasterInfo, PlaybackSessionFailure } from '../../contracts/src/generated/playback-session-response.js';
export type { PptxPlaybackSessionInfo, PptxPlaybackRasterInfo, PptxPlaybackSessionFailure } from '../../contracts/src/generated/pptx-playback-session-response.js';
export { RasterComponent } from '../../raster-component/src/index.js';
export { ShapingComponent } from '../../text-component/src/index.js';

/** Thin synchronous facade, intended to run in the product's computation Worker.
 * No loading/fetch, implicit component instance, queue, clock or product state. */
export class WasmPlayback {
  constructor(private readonly module: PlaybackModule) {}
  prepareAuthor(request: PlaybackPrepareRequest): AuthorPlayback {
    return AuthorPlayback.prepare(this.module, request);
  }
  prepareSource(request: PptxPlaybackPrepareRequest, inputs: SourceInputs): SourcePlayback {
    return SourcePlayback.prepare(this.module, request, inputs);
  }
  prepareDeliveryInputs(request: DeliveryPlaybackRequest, contents: Uint8Array): DeliveryPlaybackInputs {
    return prepareDeliveryInputs(this.module, request, contents);
  }
}
