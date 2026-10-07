import type { VideoSample, AudioSample } from 'mediabunny';
import { requireMedia, type BrowserVideoSample, type BrowserAudioSample } from './contract.js';
import { BrowserVideoBudget, videoSurfaceBytes } from './video-budget.js';

export function videoView(sample: VideoSample, check: () => void, budget: BrowserVideoBudget, admittedSurfaceBytes: number): BrowserVideoSample {
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
    cloneFrame() {
      check();
      // A cropped frame can retain its full coded surface after source close.
      const release = budget.reserve(Math.max(admittedSurfaceBytes, videoSurfaceBytes(sample.visibleRect.width, sample.visibleRect.height)));
      try {
        const frame = sample.toVideoFrame();
        let closed = false;
        return Object.freeze({ frame, close() { if (!closed) { closed = true; try { frame.close(); } finally { release(); } } } });
      } catch (error) { release(); throw error; }
    },
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
