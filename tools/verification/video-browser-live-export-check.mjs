/** Public shared plan -> actual VideoEncoder -> shared AAC/mux -> OPFS. */
import assert from 'node:assert/strict';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { resolve } from 'node:path';
import { chromium } from 'playwright-core';
import { createHash } from 'node:crypto';
const [encodeArg, planArg, vectorsArg, outputArg] = process.argv.slice(2);
assert(encodeArg && planArg && vectorsArg && outputArg, 'ENCODE_COMPONENT PLAN_COMPONENT VECTORS NEW_OUTPUT required');
const output = resolve(outputArg); await mkdir(output);
const routes = new Map([['/vectors.json', await readFile(vectorsArg)]]), manifests = {};
for (const [prefix, arg] of [['encode',encodeArg], ['plan',planArg]]) {
  const root = resolve(arg), bytes = await readFile(root + '/bundle-manifest.json');
  manifests[prefix] = createHash('sha256').update(bytes).digest('hex');
  for (const row of JSON.parse(bytes).files) {
    if (!/\.(wasm|m?js)$/.test(row.path)) continue;
    const value = await readFile(root + '/' + row.path);
    assert.equal(value.length, row.byteLength); assert.equal(createHash('sha256').update(value).digest('hex'), row.sha256);
    routes.set('/' + prefix + '/' + row.path, value);
  }
}
routes.set('/worker.mjs', Buffer.from(`(${workerMain.toString()})()`));
const server = createServer((req, res) => {
  if (req.url === '/') { res.writeHead(200, { 'Content-Type':'text/html' }); res.end('<!doctype html><link rel="icon" href="data:,"><title>Live browser export evidence</title>'); return; }
  const value = routes.get(req.url); if (!value) { res.writeHead(404); res.end(); return; }
  const contentType = req.url.endsWith('.wasm') ? 'application/wasm' : /\.m?js$/.test(req.url) ? 'text/javascript' : req.url.endsWith('.mp4') ? 'video/mp4' : 'application/json';
  const range = /^bytes=(\d+)-(\d*)$/.exec(req.headers.range ?? '');
  if (range) {
    const start = Number(range[1]), end = Math.min(range[2] ? Number(range[2]) : value.length - 1, value.length - 1);
    if (start > end) { res.writeHead(416); res.end(); return; }
    res.writeHead(206, { 'Content-Type':contentType, 'Accept-Ranges':'bytes', 'Content-Range':`bytes ${start}-${end}/${value.length}`, 'Content-Length':end-start+1 }); res.end(value.subarray(start, end+1));
  } else { res.writeHead(200, { 'Content-Type':contentType, 'Accept-Ranges':'bytes', 'Content-Length':value.length }); res.end(value); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const browser = await chromium.launch({ executablePath:'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', headless:true });
try {
  const page = await browser.newPage(), errors = []; page.on('pageerror', e => errors.push(String(e)));
  await page.goto(`http://127.0.0.1:${server.address().port}`);
  const result = await page.evaluate(() => new Promise((yes, no) => {
    const worker = new Worker('/worker.mjs', { type:'module' });
    const timer = setTimeout(() => { worker.terminate(); no(Error('live export deadline')); }, 60000);
    worker.onmessage = e => { clearTimeout(timer); worker.terminate(); e.data.error ? no(Error(e.data.error)) : yes(e.data); };
    worker.onerror = e => { clearTimeout(timer); worker.terminate(); no(Error(e.message)); };
  }));
  assert.deepEqual(errors, []);
  for (const row of result.cases) {
    const bytes = Buffer.from(row.bytes); routes.set('/outputs/' + row.name + '.mp4', bytes);
    await writeFile(output + '/' + row.name + '.mp4', bytes); delete row.bytes;
  }
  const playback = await page.evaluate(async rows => {
    const results = [];
    for (const row of rows) {
      const video = document.createElement('video'); video.controls = true; video.muted = true; document.body.append(video);
      const event = (name, action) => new Promise((yes, no) => {
        const finish = fn => { clearTimeout(timer); video.removeEventListener(name, ok); video.removeEventListener('error', bad); fn(); };
        const ok = () => finish(yes), bad = () => finish(() => no(Error('video playback failed')));
        const timer = setTimeout(() => finish(() => no(Error('video event deadline: ' + name))), 10000);
        video.addEventListener(name, ok); video.addEventListener('error', bad); action?.();
      });
      try {
        await event('loadeddata', () => { video.src = '/outputs/' + row.name + '.mp4'; });
        const expected = Math.max(Number(row.info.videoDuration) / row.info.videoTimeScale, Number(row.info.audioSamples) / row.info.audioSampleRate);
        if (Math.abs(video.duration - expected) > 1e-6) throw Error('browser duration differs from exact extent');
        const target = expected * 0.6;
        await event('seeked', () => { video.currentTime = target; });
        if (Math.abs(video.currentTime - target) > 1e-6) throw Error('browser seek failed');
        const ended = event('ended'); await video.play(); await ended;
        results.push({ name:row.name, duration:video.duration, seekTarget:target, end:video.currentTime, ended:video.ended });
      } finally { video.pause(); video.removeAttribute('src'); video.load(); video.remove(); }
    }
    return results;
  }, result.cases);
  const report = { format:'musteroffice.browser-live-export-check/1', browser:browser.version(), manifests, ...result, playback };
  await writeFile(output + '/report.json', JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify(report));
} catch (error) { await writeFile(output + '/failure.txt', String(error.stack ?? error)); throw error; }
finally { await browser.close(); await new Promise(resolve => server.close(resolve)); }

async function workerMain() {
  try {
    const { createEncodeRuntime } = await import('/encode/index.mjs'), { encodeProject } = await import('/encode/project.mjs');
    const { createExportRuntime } = await import('/plan/index.mjs');
    const runtime = await createEncodeRuntime(await WebAssembly.compileStreaming(fetch('/encode/runtime/encode.wasm')));
    const planning = await createExportRuntime(await WebAssembly.compileStreaming(fetch('/plan/runtime/mo_video_export_wasm_bg.wasm')));
    const vectors = await (await fetch('/vectors.json')).json(), root = await navigator.storage.getDirectory(), results = [], negative = [];
    const limits = { maxFileBytes:'16777216', maxWriteBytes:'33554432', maxPendingBytes:'4194304', maxFrameBytes:'4194304',
      maxPixels:1048576, maxPackets:10000, maxPacketBytes:1048576, maxPendingPackets:256, maxSteps:10000, maxWriteCalls:100000 };
    const need = (condition, message) => { if (!condition) throw Error(message); };
    const sample = (at, channel, rate) => 0.25 * Math.sin(2 * Math.PI * (channel ? 937 : 431) * at / rate)
      * (at < 256 || at % 8192 < 128 ? 0 : 1);
    async function execute(vector, name, failureCase) {
      const plan = planning.ProjectExport.open(JSON.stringify(vector.document), vector.documentDigest, JSON.stringify(vector.options));
      const info = JSON.parse(plan.info()), abort = new AbortController(), retainedFrames = [];
      const file = await root.getFileHandle(name, { create:true }), handle = await file.createSyncAccessHandle();
      handle.truncate(0);
      let writes = 0, progress = 0, stats, completed = false;
      try {
        const result = await encodeProject({ plan, runtime, signal:abort.signal, limits,
          budgets:failureCase === 'video-byte-budget' ? { maxVideoInputBytes:1 } : failureCase === 'pcm-byte-budget' ? { maxBufferedPcmBytes:1 } : {},
          sink:{ cancelled:() => false, writeAt(at, data) {
            writes++;
            if (failureCase === 'write-exception' && writes > 1) throw Error('injected storage failure');
            return handle.write(data.subarray(0, 997), { at:Number(at) });
          } },
          onProgress() { if (++progress === 4 && failureCase === 'cancelled') abort.abort(new Error('injected cancellation')); },
          async createFrame(work, transport, preparationSignal) {
            const pixels = new Uint8Array(info.width * info.height * 3 / 2);
            for (let y = 0; y < info.height; y++) for (let x = 0; x < info.width; x++) pixels[y * info.width + x] = 16 + ((x + y + work.index * 5) % 220);
            pixels.fill(100, info.width * info.height, info.width * info.height * 5 / 4); pixels.fill(150, info.width * info.height * 5 / 4);
            const frame = new VideoFrame(pixels, { format:'I420', codedWidth:info.width, codedHeight:info.height,
              timestamp:Number(transport.timestampUs) + (failureCase === 'wrong-timestamp' ? 1 : 0), duration:Number(transport.durationUs),
              colorSpace:{ primaries:'bt709', transfer:'bt709', matrix:'bt709', fullRange:failureCase === 'wrong-color' } });
            retainedFrames.push(frame);
            if (failureCase === 'cancel-during-frame') {
              abort.abort(new Error('injected in-flight cancellation'));
              await new Promise(resolve => setTimeout(resolve, 10));
              need(preparationSignal.aborted, 'host preparation was not cancelled');
            }
            return frame;
          },
          async createPcm(work) {
            const data = new Float32Array(work.frames * info.audioChannels), start = Number(work.firstSample);
            for (let i = 0; i < work.frames; i++) for (let ch = 0; ch < info.audioChannels; ch++) data[i * info.audioChannels + ch] = sample(start + i, ch, info.audioSampleRate);
            return data;
          },
        });
        completed = true; need(!failureCase, 'failure was silently accepted'); stats = result.stats;
        need(stats.videoFrames === String(info.videoFrames) && stats.audioSamples === info.audioSamples, 'completion count');
        need(handle.getSize() === Number(stats.byteLength), 'file length');
        handle.flush(); const bytes = new Uint8Array(handle.getSize()); need(handle.read(bytes, { at:0 }) === bytes.length, 'test readback');
        results.push({ name, info, ...result, writes, bytes:Array.from(bytes) });
      } catch (error) {
        if (!failureCase) throw error;
        need(!completed && !stats, 'failed job acquired a completion receipt'); negative.push({ name:failureCase, message:error.message, progress, writes });
      } finally {
        if (failureCase === 'cancel-during-frame') await new Promise(resolve => setTimeout(resolve, 30));
        need(retainedFrames.every(frame => frame.format === null), 'frame not closed');
        plan.free(); handle.close(); await root.removeEntry(name);
      }
    }
    for (const [index, vector] of vectors.cases.entries()) await execute(vector, 'case-' + index);
    for (const name of ['cancelled','cancel-during-frame','write-exception','wrong-timestamp','wrong-color','video-byte-budget','pcm-byte-budget']) await execute(vectors.cases[0], 'negative-' + name, name);
    postMessage({ cases:results, negative });
  } catch (error) { postMessage({ error:String(error.stack ?? error) + ' status=' + error.status + ' upstream=' + error.upstreamCode }); }
}
