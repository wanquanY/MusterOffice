import { AudioSampleSink, type AudioSample, type InputAudioTrack } from 'mediabunny';
import { requireMedia, type ExactRatio, type SampleInterval } from './contract.js';
import { compare, interval } from './time.js';

/** Forward reads reuse actual decoder state and one boundary sample. A backward
 * read rebuilds from the first packet until certified codec recovery points are
 * available. No guessed preroll, no complete audio file kept in memory. */
export class BrowserAudioCursor {
  private iterator: AsyncGenerator<AudioSample, void, unknown> | undefined;
  private sample: AudioSample | undefined;
  private last: SampleInterval | undefined;
  private closed = false;
  private exhausted = false;
  constructor(private readonly track: InputAudioTrack, private readonly firstTimestamp: number) {}
  private async reset(): Promise<void> {
    const iterator = this.iterator;
    this.iterator = undefined;
    this.sample?.close(); this.sample = undefined; this.last = undefined; this.exhausted = false;
    await iterator?.return();
  }
  async read(start: ExactRatio, end: ExactRatio, consume: (sample: AudioSample, interval: SampleInterval) => Promise<boolean>): Promise<number> {
    requireMedia(!this.closed, 'closed');
    if (this.last && (compare(start, this.last.start) < 0 || (this.exhausted && compare(start, this.last.end) < 0))) await this.reset();
    requireMedia(!this.closed, 'closed');
    this.iterator ??= new AudioSampleSink(this.track).samples(this.firstTimestamp);
    let count = 0;
    for (;;) {
      requireMedia(!this.closed, 'closed');
      if (!this.sample) {
        const next = await this.iterator.next();
        if (this.closed) { next.value?.close(); requireMedia(false, 'closed'); }
        if (next.done) { this.exhausted = true; return count; }
        this.sample = next.value;
        this.last = interval(this.sample.timestamp, this.sample.numberOfFrames / this.sample.sampleRate, this.sample.sampleRate);
      }
      const sample = this.sample, span = this.last!;
      if (compare(span.start, end) >= 0) return count;
      if (compare(span.end, start) > 0) {
        // The cursor owns its boundary sample; consumers own only this clone.
        const borrowed = sample.clone();
        let keepGoing;
        try { count++; keepGoing = await consume(borrowed, span); }
        finally { borrowed.close(); }
        if (!keepGoing || compare(span.end, end) >= 0) return count;
      }
      sample.close(); this.sample = undefined;
    }
  }
  async close(): Promise<void> {
    if (this.closed) return;
    this.closed = true;
    await this.reset();
  }
}
