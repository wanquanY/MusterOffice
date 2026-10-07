import { InputAudioTrack, InputVideoTrack, type InputTrack } from 'mediabunny';
import { requireMedia, type BrowserMediaLimits, type BrowserTrack } from './contract.js';
import { fromSeconds, validateScale } from './time.js';

export async function probeTrack(track: InputTrack, limits: BrowserMediaLimits): Promise<BrowserTrack> {
  const timescale = await track.getTimeResolution();
  validateScale(timescale);
  const first = await track.getFirstTimestamp(), end = await track.computeDuration();
  requireMedia(Math.abs(first) <= limits.maxDurationSeconds && Math.abs(end) <= limits.maxDurationSeconds
    && end >= first, 'resource_limit');
  const timing = {
    id: track.id, codec: await track.getCodec(), timescale,
    first: fromSeconds(first, timescale), end: fromSeconds(end, timescale),
  };
  if (track instanceof InputVideoTrack) {
    const codedWidth = await track.getCodedWidth(), codedHeight = await track.getCodedHeight(),
      displayWidth = await track.getDisplayWidth(), displayHeight = await track.getDisplayHeight();
    for (const value of [codedWidth, codedHeight, displayWidth, displayHeight])
      requireMedia(Number.isSafeInteger(value) && value > 0, 'unsupported_media');
    requireMedia(codedWidth * codedHeight <= limits.maxPixels && displayWidth * displayHeight <= limits.maxPixels, 'resource_limit');
    return {
      ...timing, kind: 'video', codedWidth, codedHeight, displayWidth, displayHeight,
      pixelAspectRatio: await track.getPixelAspectRatio(), transformationMatrix: await track.getTransformationMatrix(),
      rotation: await track.getRotation(), flip: await track.getFlip(), colorSpace: await track.getColorSpace(),
      decodeConfigurationSupported: await track.canDecode(),
    };
  }
  requireMedia(track instanceof InputAudioTrack, 'unsupported_media');
  const sampleRate = await track.getSampleRate(), channels = await track.getNumberOfChannels();
  requireMedia(Number.isSafeInteger(sampleRate) && sampleRate > 0 && sampleRate <= limits.maxAudioSampleRate
    && Number.isSafeInteger(channels) && channels > 0 && channels <= limits.maxAudioChannels, 'resource_limit');
  return { ...timing, kind: 'audio', sampleRate, channels, decodeConfigurationSupported: await track.canDecode() };
}
