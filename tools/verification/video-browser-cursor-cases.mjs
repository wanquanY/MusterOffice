import { BrowserMediaInput, BrowserVideoBudget } from '/media-sdk/index.mjs';
const check = (value, message) => { if (!value) throw Error(message); };
const code = async run => { try { await run(); return null; } catch (error) { return error.code ?? error.message; } };
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
async function until(test) {
  for (let i = 0; i < 200; i++) { if (test()) return; await sleep(5); }
  throw Error('expected resource transition did not occur');
}

/** Real SDK/Blob/WebCodecs ownership and cancellation tests, not fake decoders. */
export async function cursorCases(runtime, fixture, rows, stats) {
  const selected = rows.map((row, i) => ({ row, request: fixture.requests[i].body })).filter(x => x.row.selection.type === 'sample');
  const first = selected[0].request, last = selected.at(-1).request, id = fixture.id;
  const pool = new BrowserVideoBudget(16 << 20);
  const open = (blob = fixture.blob, limits) => BrowserMediaInput.open(blob, { mediaIndexRuntime: runtime, videoBudget: pool, ...(limits && { limits }) });
  const media = await open();
  let previous, owned, expiredCopy, expiredClone, busy;
  const begin = stats.configured;
  try {
    await media.movieVideoAt(id, first, sample => { previous = sample; });
    await media.movieVideoAt(id, first, async sample => {
      expiredCopy = await code(() => previous.copyRgba(new Uint8Array(previous.width * previous.height * 4)));
      expiredClone = await code(() => previous.cloneFrame());
      busy = await code(() => media.releaseMovieVideo());
      const pixels = new Uint8Array(sample.width * sample.height * 4); await sample.copyRgba(pixels);
      owned = sample.cloneFrame();
    });
    check(stats.configured - begin === 1, 'same frame recreated decoder');
    check(expiredCopy === 'closed' && expiredClone === 'closed', 'old borrow reactivated during a later callback');
    check(busy === 'busy', 'nested eviction must not close active borrow');
    // A retained current frame must backpressure the complete decoder stream.
    await sleep(100); const idleChunks = stats.chunks, idleFrames = stats.liveFrames;
    await sleep(100);
    check(stats.chunks === idleChunks && stats.liveFrames === idleFrames && idleFrames <= 9, 'idle cursor read/decoded without bounds');
    await media.releaseMovieVideo();
    check(pool.usedBytes > 0 && owned.frame.codedWidth > 0, 'owned clone lost on cursor eviction');
    const cloneBytes = pool.usedBytes; owned.close(); owned.close(); owned = undefined;
    check(pool.usedBytes === 0, 'eviction leaked decoder or clone admission');
    const restart = stats.configured;
    await media.movieVideoAt(id, first, () => {});
    check(stats.configured === restart + 1, 'evicted cursor did not restart');
    await media.releaseMovieVideo();
    const sentinel = await code(() => media.movieVideoAt(id, first, () => { throw Error('consumer-failure'); }));
    check(sentinel === 'consumer-failure' && pool.usedBytes === 0, 'callback failure leaked cursor or lost original error');
    await media.movieVideoAt(id, first, () => {});
    const afterFirst = stats.configured;
    await media.movieVideoAt(id, last, () => {});
    check(stats.configured === afterFirst, 'bounded forward progression reset decoder');
    await media.movieVideoAt(id, first, () => {});
    check(stats.configured === afterFirst + 1, 'backwards seek reused wrong decoder position');
    await media.releaseMovieVideo();
    // Packet-clock entry points must evict a cached movie cursor, not compete
    // with the same input's retained reservation in a 16 MiB shared pool.
    await media.movieVideoAt(id, first, () => {});
    await media.videoAt(id, { time: { numerator: '0', denominator: '1' }, side: 'after' }, () => {});
    check(pool.usedBytes === 0, 'packet-clock transition left movie cursor live');
    const result = { expiredCopy, expiredClone, busy, idleChunks, idleFrames, cloneBytes, callbackFailure: sentinel };
    const limited = await open(fixture.blob, { maxSequentialVideoSamples: 2 });
    try {
      await limited.movieVideoAt(id, first, () => {}); const beforeJump = stats.configured;
      await limited.movieVideoAt(id, last, () => {});
      check(stats.configured === beforeJump + 1, 'large ordinal jump ignored sequential work limit');
      await limited.releaseMovieVideo(); check(pool.usedBytes === 0, 'large jump leaked admission');
    } finally { limited.close(); }
    const competitor = await open();
    try {
      await media.movieVideoAt(id, first, () => {});
      const configured = stats.configured;
      result.concurrent = await code(() => competitor.movieVideoAt(id, first, () => {}));
      check(result.concurrent === 'resource_limit' && stats.configured === configured, 'cached cursor not charged to shared pool');
      await media.releaseMovieVideo();
      await competitor.movieVideoAt(id, first, () => {}); await competitor.releaseMovieVideo();
      check(pool.usedBytes === 0, 'pool not reusable after explicit eviction');
    } finally { competitor.close(); }
    const damagedBytes = new Uint8Array(await fixture.blob.arrayBuffer());
    const damageOffset = Number(fixture.packets[12].bytes.offset);
    damagedBytes.fill(0xff, damageOffset, damageOffset + 4);
    const damaged = await open(new Blob([damagedBytes]));
    try {
      await damaged.movieVideoAt(id, first, () => {});
      const rejected = await code(() => damaged.movieVideoAt(id, last, () => {}));
      check(rejected === 'unsupported_media' && pool.usedBytes === 0, `mid-stream packet error did not retire cached decoder: ${rejected}, ${pool.usedBytes}`);
      const beforeRecovery = stats.configured;
      await damaged.movieVideoAt(id, first, () => {}); await damaged.releaseMovieVideo();
      check(stats.configured === beforeRecovery + 1 && pool.usedBytes === 0, 'failed cursor prevented independent earlier seek');
      result.damagedPacket = { rejected, damageOffset, recovered: true };
    } finally { damaged.close(); }
    result.blockedRead = await blockedRead(runtime, fixture, first, pool, stats);
    return result;
  } finally { owned?.close(); media.close(); await until(() => pool.usedBytes === 0); }
}

async function blockedRead(runtime, fixture, request, pool, stats) {
  const first = fixture.packets[0], start = Number(first.bytes.offset), end = start + Number(first.bytes.length);
  let enabled = false, entered = false, release;
  const gate = new Promise(resolve => { release = resolve; });
  class DelayedBlob extends Blob {
    slice(a, b, type) {
      const part = super.slice(a, b, type);
      if (enabled && a === start && b === end) {
        enabled = false; const read = part.arrayBuffer.bind(part);
        Object.defineProperty(part, 'arrayBuffer', { value: async () => { entered = true; await gate; return read(); } });
      }
      return part;
    }
  }
  const source = new DelayedBlob([fixture.blob]);
  const media = await BrowserMediaInput.open(source, { mediaIndexRuntime: runtime, videoBudget: pool });
  const configured = stats.configured;
  try {
    enabled = true;
    const pending = code(() => media.movieVideoAt(fixture.id, request, () => { throw Error('closed source delivered frame'); }));
    await until(() => entered);
    media.close(); const rejected = await pending;
    check(rejected === 'closed' && stats.configured === configured, 'pending pre-configure read not cancelled');
    const heldBytes = pool.usedBytes;
    check(heldBytes > 0, 'still-running admitted read released its reservation early');
    release(); await until(() => pool.usedBytes === 0);
    return { rejected, heldBytes, releasedBytes: pool.usedBytes, configured: stats.configured - configured };
  } finally { release(); media.close(); }
}
