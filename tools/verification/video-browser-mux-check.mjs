/** Real browser worker + OPFS + shared AAC/MP4 leaf, with captured actual AVC.
 * The fixture packet capture is test-only. Product execution must be streaming.
 */
import assert from 'node:assert/strict';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { resolve } from 'node:path';
import { chromium } from 'playwright-core';
import { createHash } from 'node:crypto';
const [componentArg, probeArg, vectorsArg, outputArg] = process.argv.slice(2);
assert(componentArg && probeArg && vectorsArg && outputArg, 'COMPONENT CODEC_PROBE VECTORS NEW_OUTPUT required');
const root = resolve(componentArg), output = resolve(outputArg); await mkdir(output);
const manifestBytes = await readFile(root + '/bundle-manifest.json'), vectors = JSON.parse(await readFile(vectorsArg));
const probe = JSON.parse(await readFile(probeArg));
const input = probe.cases.map((c, i) => ({ info:c.info, video:c.video, config:c.videoConfig, steps:vectors.cases[i].steps }));
const routes = new Map([['/input.json', Buffer.from(JSON.stringify(input))]]);
for (const row of JSON.parse(manifestBytes).files) {
  if (!/\.(wasm|mjs)$/.test(row.path)) continue;
  const bytes = await readFile(root + '/' + row.path);
  assert.equal(bytes.length, row.byteLength); assert.equal(createHash('sha256').update(bytes).digest('hex'), row.sha256);
  routes.set('/sdk/' + row.path, bytes);
}
routes.set('/worker.mjs', Buffer.from(`(${workerMain.toString()})()`));
const server = createServer((req, res) => {
  if (req.url === '/') { res.writeHead(200, { 'Content-Type':'text/html' }); res.end('<!doctype html><title>Browser mux evidence</title>'); return; }
  const value = routes.get(req.url); if (!value) { res.writeHead(404); res.end(); return; }
  res.writeHead(200, { 'Content-Type': req.url.endsWith('.wasm') ? 'application/wasm' : req.url.endsWith('.mjs') ? 'text/javascript' : 'application/json' }); res.end(value);
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const browser = await chromium.launch({ executablePath:'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', headless:true });
try {
  const page = await browser.newPage(), errors = []; page.on('pageerror', e => errors.push(String(e)));
  page.on('console', message => console.log('browser:', message.text()));
  await page.goto(`http://127.0.0.1:${server.address().port}`);
  const result = await page.evaluate(() => new Promise((yes, no) => {
    const worker = new Worker('/worker.mjs', { type:'module' });
    const timer = setTimeout(() => { worker.terminate(); no(Error('mux verification deadline')); }, 60000);
    worker.onmessage = e => { clearTimeout(timer); worker.terminate(); e.data.error ? no(Error(e.data.error)) : yes(e.data); };
    worker.onerror = e => { clearTimeout(timer); worker.terminate(); no(Error(e.message)); };
  }));
  assert.deepEqual(errors, []);
  for (const row of result.cases) { await writeFile(output + '/' + row.name + '.mp4', Buffer.from(row.bytes)); delete row.bytes; }
  const report = { format:'musteroffice.browser-mux-check/1', browser:browser.version(),
    manifestSha256:createHash('sha256').update(manifestBytes).digest('hex'), ...result };
  await writeFile(output + '/report.json', JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify(report));
} catch (error) { await writeFile(output + '/failure.txt', String(error.stack ?? error)); throw error; }
finally { await browser.close(); await new Promise(resolve => server.close(resolve)); }

async function workerMain() {
  try {
    const { createEncodeRuntime } = await import('/sdk/index.mjs');
    const runtime = await createEncodeRuntime(await WebAssembly.compileStreaming(fetch('/sdk/runtime/encode.wasm')));
    const cases = await (await fetch('/input.json')).json(), root = await navigator.storage.getDirectory(), results = [];
    const limits = { maxFileBytes:'16777216', maxWriteBytes:'33554432', maxPendingBytes:'4194304', maxFrameBytes:'4194304',
      maxPixels:1048576, maxPackets:10000, maxPacketBytes:1048576, maxPendingPackets:256, maxSteps:10000, maxWriteCalls:100000 };
    const sample = (at, channel, rate) => 0.25 * Math.sin(2 * Math.PI * (channel ? 937 : 431) * at / rate)
      * (at < 256 || at % 8192 < 128 ? 0 : 1);
    const need = (condition, message) => { if (!condition) throw Error(message); };
    for (const [index, c] of cases.entries()) {
      const name = 'case-' + index, file = await root.getFileHandle(name, { create:true }), handle = await file.createSyncAccessHandle();
      handle.truncate(0);
      let encoder, writes = 0;
      try {
        encoder = runtime.open(c.info, limits, { cancelled:() => false,
          writeAt(at, data) { writes++; return handle.write(data.subarray(0, 997), { at:Number(at) }); } }, new Uint8Array(c.config.description));
        for (const { work } of c.steps) {
          if (work.kind === 'video') {
            const packet = c.video[work.index];
            encoder.video(work.pts, work.duration, new Uint8Array(packet.data), packet.type === 'key');
          } else if (work.kind === 'audio') {
            const pcm = new Float32Array(work.frames * c.info.audioChannels), start = Number(work.firstSample);
            for (let i = 0; i < work.frames; i++) for (let ch = 0; ch < c.info.audioChannels; ch++) pcm[i * c.info.audioChannels + ch] = sample(start + i, ch, c.info.audioSampleRate);
            encoder.audio(work.firstSample, pcm);
          }
        }
        const stats = encoder.finish();
        need(stats.videoFrames === String(c.info.videoFrames) && stats.audioSamples === c.info.audioSamples, 'completion counts');
        need(handle.getSize() === Number(stats.byteLength), 'actual file extent');
        handle.flush();
        const bytes = new Uint8Array(handle.getSize()); need(handle.read(bytes, { at:0 }) === bytes.length, 'test readback');
        results.push({ name, info:c.info, stats, writes, bytes:Array.from(bytes) });
      } finally { encoder?.close(); handle.close(); await root.removeEntry(name); }
    }
    const negative = [];
    async function rejects(name, overrides, action, cancelled = () => false, writeAt = () => { throw Error('unexpected write'); }) {
      let encoder, rejected = false;
      try {
        encoder = runtime.open(cases[0].info, { ...limits, ...overrides }, { cancelled, writeAt }, new Uint8Array(cases[0].config.description));
        action(encoder);
      } catch (error) { rejected = true; negative.push({ name, status:error.status, message:error.message }); }
      finally { encoder?.close(); }
      need(rejected, 'did not reject ' + name);
    }
    const discard = (_at, bytes) => bytes.length, first = cases[0].video[0], w = cases[0].steps[0].work;
    await rejects('cancel-before-open', {}, () => {}, () => true);
    await rejects('write-exception', {}, () => {});
    await rejects('zero-write', {}, () => {}, () => false, () => 0);
    await rejects('fractional-write', {}, () => {}, () => false, () => 0.5);
    await rejects('incomplete-finish', {}, e => e.finish(), () => false, discard);
    await rejects('wrong-first-clock', {}, e => e.video('1', w.duration, new Uint8Array(first.data), true), () => false, discard);
    await rejects('wrong-duration', {}, e => e.video('0', '1', new Uint8Array(first.data), true), () => false, discard);
    await rejects('wrong-key', {}, e => e.video('0', w.duration, new Uint8Array(first.data), false), () => false, discard);
    await rejects('corrupt-avc', {}, e => e.video('0', w.duration, new Uint8Array([1,2,3]), true), () => false, discard);
    await rejects('corrupt-after-valid-avc', {}, e => {
      e.video('0', w.duration, new Uint8Array(first.data), true);
      e.video(w.duration, w.duration, new Uint8Array([0,0,0,1,65]), false);
    }, () => false, discard);
    await rejects('duplicate-video', {}, e => {
      e.video('0', w.duration, new Uint8Array(first.data), true);
      e.video('0', w.duration, new Uint8Array(first.data), true);
    }, () => false, discard);
    await rejects('packet-budget', { maxPacketBytes:8 }, e => e.video('0', w.duration, new Uint8Array(first.data), true), () => false, discard);
    await rejects('file-budget', { maxFileBytes:'8', maxWriteBytes:'8' }, () => {}, () => false, discard);
    await rejects('non-finite-pcm', {}, e => e.audio('0', new Float32Array([NaN, 0])), () => false, discard);
    postMessage({ cases:results, negative });
  } catch (error) { postMessage({ error:String(error.stack ?? error) + ' status=' + error.status + ' upstream=' + error.upstreamCode }); }
}
