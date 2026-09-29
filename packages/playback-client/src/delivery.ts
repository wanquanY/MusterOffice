import type { DeliveryPlaybackRequest } from '../../contracts/src/generated/delivery-playback-request.js';
import type { DeliveryPlaybackInputs, DeliveryPlaybackResponse, PptxFailure } from '../../contracts/src/generated/delivery-playback-response.js';
import type { PlaybackModule } from './ports.js';
import { decode, encode, inputBytes, requireResponse } from './owner.js';

export type { DeliveryPlaybackRequest, DeliveryPlaybackInputs };

export class DeliveryPlaybackError extends Error {
  constructor(readonly diagnostic: PptxFailure) {
    super(diagnostic.message);
    this.name = 'DeliveryPlaybackError';
  }
}

/** A one-time inspection. Native receivers can reuse their ReceivedDelivery;
 * the portable channel takes an explicitly packed, bounded asset collection.
 * No content fetch, implicit font selection, session or raster work happens.
 */
export function prepareDeliveryInputs(
  module: PlaybackModule, request: DeliveryPlaybackRequest, contents: Uint8Array,
): DeliveryPlaybackInputs {
  inputBytes(contents);
  const response = decode<DeliveryPlaybackResponse>(module.prepare_delivery_playback(encode(request), contents));
  if (response.status === 'error') throw new DeliveryPlaybackError(response.error);
  requireResponse(response.status === 'prepared');
  const inputs = response.inputs;
  requireResponse(inputs.profile === 'delivery-playback-inputs-v1-draft' &&
    inputs.documentId === request.delivery.expected.documentId &&
    inputs.revision === request.delivery.expected.revision &&
    inputs.source.id === request.delivery.bundle.pptxAssetId &&
    inputs.pages.length === request.delivery.bundle.previews.length &&
    inputs.pages.every((page, i) => page.pageId === request.delivery.bundle.previews[i]!.pageId &&
      page.request.page.viewport.width === request.width &&
      page.request.page.expectedSourceSha256 === inputs.source.sha256));
  return inputs;
}
