import type { VideoSample, AudioSample } from 'mediabunny';
import { requireMedia, type BrowserVideoSample, type BrowserAudioSample } from './contract.js';

export function videoView(sample: VideoSample, check: () => void): BrowserVideoSample {
  return Object.freeze({
    width: sample.visibleRect.width, height: sample.visibleRect.height,
    displayWidth: sample.displayWidth, displayHeight: sample.displayHeight,
    rotation: sample.rotation, flip: sample.flip, pixelAspectRatio: Object.freeze({ ...sample.pixelAspectRatio }),
    async copyRgba(destination: Uint8Array<ArrayBuffer>) {
      check();
      requireMedia(destination.byteLength === sample.visibleRect.width * sample.visibleRect.height * 4, 'invalid_request');
      await sample.copyTo(destination, { format: 'RGBA', colorSpace: 'srgb' });
      check();
    },
    cloneFrame() { check(); return sample.toVideoFrame(); },
  });
}
export function audioView(sample: AudioSample, check: () => void): BrowserAudioSample {
  return Object.freeze({
    sampleRate: sample.sampleRate, numberOfFrames: sample.numberOfFrames, numberOfChannels: sample.numberOfChannels,
    copyInterleaved(destination: Float32Array<ArrayBuffer>) {
      check();
      requireMedia(destination.length === sample.numberOfFrames * sample.numberOfChannels, 'invalid_request');
      sample.copyTo(destination, { format: 'f32', planeIndex: 0 });
    },
  });
}
