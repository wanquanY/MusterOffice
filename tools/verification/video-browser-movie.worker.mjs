import { createMediaIndexRuntime } from '/index-sdk/index.mjs';
import { BrowserMediaInput, BrowserVideoBudget } from '/media-sdk/index.mjs';
import { cursorCases } from '/video-browser-cursor-cases.mjs';

// Test-only platform instrumentation; it does not participate in production IO.
const stats = { configured: 0, liveDecoders: 0, liveFrames: 0, peakFrames: 0, chunks: 0 };
const held = new WeakSet();
function hold(frame) {
  if (held.has(frame)) return frame;
  held.add(frame); stats.liveFrames++; stats.peakFrames = Math.max(stats.peakFrames, stats.liveFrames);
  const close = frame.close.bind(frame), clone = frame.clone.bind(frame);
  frame.close = () => { if (held.delete(frame)) stats.liveFrames--; close(); };
  frame.clone = () => hold(clone()); return frame;
}
const Frame = VideoFrame;
globalThis.VideoFrame = new Proxy(Frame, { construct(Target, args) { return hold(Reflect.construct(Target, args)); } });
const Platform = VideoDecoder;
globalThis.VideoDecoder = new Proxy(Platform, { construct(Target, args) {
  const config = args[0];
  const decoder = new Target({ ...config, output(frame) {
    config.output(hold(frame));
  } });
  stats.configured++; stats.liveDecoders++;
  let closed = false; const close = decoder.close.bind(decoder), decode = decoder.decode.bind(decoder);
  decoder.close = () => { if (!closed) { closed = true; stats.liveDecoders--; } close(); };
  decoder.decode = chunk => { stats.chunks++; decode(chunk); };
  return decoder;
} });

async function run(data) {
  const runtime = await createMediaIndexRuntime(data.module), budget = new BrowserVideoBudget(16 << 20);
  const results = [];
  for (const fixture of data.fixtures) {
    const media = await BrowserMediaInput.open(fixture.blob, { mediaIndexRuntime: runtime, videoBudget: budget });
    const id = fixture.id, info = media.movieVideoInfo(id), rows = [], configured = stats.configured, chunks = stats.chunks;
    let forward; const randomAccess = [];
    try {
      const reference = new Map(fixture.frames.map((frame, i) => [String(frame.pts), i])), fingerprints = new Map();
      for (const request of fixture.requests) {
        let actual, span, metadata, called = 0; const before = stats.configured;
        const selection = await media.movieVideoAt(id, request.body, async (sample, interval) => {
          called++; span = interval;
          metadata = sample.codecMetadata;
          actual = new Uint8Array(sample.width * sample.height * 4); await sample.copyRgba(actual);
        });
        let mae = null;
        if (actual) {
          const frame = reference.get(selection.rawStart);
          if (frame === undefined) throw Error('selected frame missing from independent raw oracle');
          if (!metadata?.picture || metadata.sample !== selection.sample
            || metadata.chromaLocation?.toLowerCase() !== fixture.frames[frame].chroma) throw Error('per-picture AVC interpretation mismatch');
          const pixels = new Uint8Array(fixture.pixels, frame * actual.length, actual.length);
          let sum = 0; for (let i = 0; i < actual.length; i++) sum += Math.abs(actual[i] - pixels[i]);
          mae = sum / actual.length;
          const digest = Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', actual)), byte => byte.toString(16).padStart(2, '0')).join('');
          if (fingerprints.has(selection.rawStart)) {
            if (fingerprints.get(selection.rawStart) !== digest) throw Error('same raw frame changed across movie edits/seeks');
          } else {
            // A small absolute error alone can accept a neighbouring frame.
            // Require the selected independent frame to be the closest image.
            for (let other = 0; other < fixture.frames.length; other++) {
              if (other === frame) continue;
              const candidate = new Uint8Array(fixture.pixels, other * actual.length, actual.length);
              let delta = 0;
              for (let p = 0; p < actual.length && delta <= sum; p++) delta += Math.abs(actual[p] - candidate[p]);
              if (delta < sum) throw Error('a different raw frame is a closer pixel match');
            }
            fingerprints.set(selection.rawStart, digest);
          }
        }
        rows.push({ selection, called, span, mae, configured: stats.configured - before });
      }
      forward = {configured: stats.configured - configured, chunks: stats.chunks - chunks};
      const valid = rows.map((row,i)=>({row,request:fixture.requests[i]})).filter(x=>x.row.selection.type==='sample');
      for (const target of [valid.at(-1), valid[0], valid[Math.floor(valid.length/2)]]) {
        await media.releaseMovieVideo();
        let digest, metadata;
        const selection = await media.movieVideoAt(id, target.request.body, async sample => {
          const bytes = new Uint8Array(sample.width * sample.height * 4); await sample.copyRgba(bytes);
          digest = Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes)), b=>b.toString(16).padStart(2,'0')).join('');
          metadata = sample.codecMetadata;
        });
        if (selection.rawStart !== target.row.selection.rawStart || digest !== fingerprints.get(selection.rawStart)
          || metadata?.sample !== selection.sample) throw Error('fresh decoder seek differs from sequential decode');
        randomAccess.push({sample:selection.sample,rawPts:selection.rawStart,chromaLocation:metadata.chromaLocation,digest});
      }
    } finally { media.close(); }
    results.push({ info, rows, ...forward, randomAccess });
  }
  // Existing shared admission also protects indexed reads before codec setup.
  const fixture = data.fixtures[0], request = fixture.requests.find((_, i) => results[0].rows[i].selection.type === 'sample');
  const limited = await BrowserMediaInput.open(fixture.blob, { mediaIndexRuntime: runtime, limits: { maxEncodedPacketBytes: 1 } });
  let rejected; const before = stats.configured;
  try { await limited.movieVideoAt(fixture.id, request.body, () => { throw Error('oversized packet consumed'); }); }
  catch (error) { rejected = error.code; } finally { limited.close(); }
  const packetLimit = { rejected, configured: stats.configured - before };
  const broken = await BrowserMediaInput.open(data.missingIdr, { mediaIndexRuntime: runtime, videoBudget: budget });
  const brokenBefore = stats.configured; let missing;
  try { await broken.movieVideoAt(fixture.id, request.body, () => { throw Error('missing IDR consumed'); }); }
  catch (error) { missing = error.code; } finally { broken.close(); }
  const missingIdr = { rejected: missing, configured: stats.configured - brokenBefore };
  const expiring = await BrowserMediaInput.open(fixture.blob, { mediaIndexRuntime: runtime, videoBudget: budget, limits: { operationDeadlineMs: 1000 } });
  let deadline;
  try { await expiring.movieVideoAt(fixture.id, request.body, () => new Promise(() => {})); }
  catch (error) { deadline = error.code; } finally { expiring.close(); }
  const cancelling = await BrowserMediaInput.open(fixture.blob, { mediaIndexRuntime: runtime, videoBudget: budget });
  const controller = new AbortController(); let cancelled;
  try { await cancelling.movieVideoAt(fixture.id, request.body, () => controller.abort(), controller.signal); }
  catch (error) { cancelled = error.code; } finally { cancelling.close(); }
  for (let i = 0; i < 100 && (budget.usedBytes || stats.liveFrames || stats.liveDecoders); i++) await new Promise(resolve => setTimeout(resolve, 10));
  const corpusStats = { ...stats };
  const cursor = await cursorCases(runtime, fixture, results[0].rows, stats);
  return { results, stats, budget: { used: budget.usedBytes, peak: budget.peakBytes, capacity: budget.capacityBytes },
    packetLimit, missingIdr, deadline, cancelled, corpusStats, cursor };
}
self.onmessage = async ({ data }) => {
  try { self.postMessage({ ok: true, ...await run(data) }); }
  catch (error) { self.postMessage({ ok: false, error: String(error), stack: error.stack, stats }); }
};
