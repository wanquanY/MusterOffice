import type { InputVideoTrack, VideoSample } from 'mediabunny';
import { requireMedia, type BrowserMediaLimits } from './contract.js';
import type { MovieVideoSelection } from './movie-contract.js';
import { BrowserMovieIndex } from './movie-index.js';
import { IndexedVideoPackets } from './movie-packets.js';
import { BudgetedVideoStream } from './video-stream.js';
import { BrowserVideoBudget } from './video-budget.js';
import { fromSeconds } from './time.js';
type Selection = Extract<MovieVideoSelection, { type: 'sample' }>;
interface DecodeSession {
  stream: BudgetedVideoStream;
  iterator: AsyncGenerator<VideoSample, void, unknown>;
  current?: { frame: VideoSample; pts: bigint };
  selectedOrdinal?: number;
}

/** One current frame and the existing codec-bounded stream. No complete-file
 * cache, second timeline mapper, uncharged clone or guessed frame-rate seek. */
export class BrowserVideoCursor {
  private session: DecodeSession | undefined;
  private closed = false;
  constructor(private readonly blob: Blob, private readonly index: BrowserMovieIndex, private readonly track: InputVideoTrack,
    private readonly timescale: number, private readonly budget: BrowserVideoBudget,
    private readonly limits: BrowserMediaLimits, private readonly signal: AbortSignal) {}
  private alive(): void { this.signal.throwIfAborted(); requireMedia(!this.closed, 'closed'); }
  check(): void { this.alive(); this.session?.stream.check(); }
  private async retire(): Promise<void> {
    const session = this.session; this.session = undefined;
    if (!session) return;
    session.stream.close(); // Also interrupts a pending iterator.next().
    try { await session.iterator.return(); }
    finally { await session.stream.retire(); }
  }
  private async open(selection: Selection): Promise<DecodeSession> {
    const packets = await IndexedVideoPackets.open(this.blob, this.index, this.track, selection, this.limits.maxEncodedPacketBytes, this.signal, this.budget);
    this.check();
    const stream = await BudgetedVideoStream.open(this.track, 0, this.budget, this.limits.maxEncodedPacketBytes, this.limits.maxPixels, this.signal, packets);
    if (this.closed || this.signal.aborted) { await stream.retire(); this.check(); }
    const session = { stream, iterator: stream[Symbol.asyncIterator]() };
    this.session = session; return session;
  }
  async read(selection: Selection, consume: (sample: VideoSample, surfaceBytes: number) => Promise<void>): Promise<void> {
    const target = BigInt(selection.rawStart), current = this.session?.current;
    const lastOrdinal = this.session?.selectedOrdinal;
    try {
      this.check();
      if (current && (target < current.pts || (lastOrdinal !== undefined
        && selection.sample - lastOrdinal > this.limits.maxSequentialVideoSamples))) await this.retire();
      this.check();
      const session = this.session ?? await this.open(selection);
      for (;;) {
        this.check();
        if (session.current) {
          requireMedia(session.current.pts <= target, 'invalid_timing');
          if (session.current.pts === target) break;
        }
        // Advancing retires the previous yielded frame and its stream credit.
        delete session.current;
        const next = await session.iterator.next(); this.alive();
        requireMedia(!next.done, 'invalid_timing'); this.check();
        session.current = { frame: next.value, pts: BigInt(fromSeconds(next.value.timestamp, this.timescale).ticks) };
      }
      session.selectedOrdinal = selection.sample;
      await consume(session.current!.frame, session.stream.surfaceBytes);
      this.check();
    } catch (error) { await this.retire(); throw error; }
  }
  close(): Promise<void> { this.closed = true; return this.retire(); }
}
