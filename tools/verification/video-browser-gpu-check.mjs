/** Actual browser WebGPU execution against native CPU reference pixels. */
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { createServer } from 'node:http';
import { resolve } from 'node:path';
import { createHash } from 'node:crypto';
import { chromium } from 'playwright-core';
const [vectorsPath, outputPath, deliveryPath] = process.argv.slice(2);
assert(vectorsPath && outputPath, 'usage: vectors.json output-directory');
const output = resolve(outputPath); await mkdir(output, { recursive: false });
const client = deliveryPath ? resolve(deliveryPath, 'client') : '.codex-work/video-browser-gpu/build';
const files = new Map([
  ['/executor.js', await readFile(client + '/executor.js')],
  ['/program.js', await readFile(client + '/program.js')],
  ['/color.js', await readFile(client + '/color.js')],
  ['/raw-input.js', await readFile(client + '/raw-input.js')],
  ['/vectors.json', await readFile(vectorsPath)],
  ['/compose.wgsl', await readFile('crates/mo-video-gpu-plan/src/compose.wgsl')],
]);
const server = createServer((req, res) => {
  if (req.url === '/') { res.writeHead(200, { 'Content-Type': 'text/html' }); res.end('<!doctype html><title>GPU verification</title>'); return; }
  const content = files.get(req.url);
  if (!content) { res.writeHead(404); res.end(); return; }
  res.writeHead(200, { 'Content-Type': req.url.endsWith('.js') ? 'text/javascript' : 'text/plain' }); res.end(content);
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const browser = await chromium.launch({ executablePath: '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', headless: true, args: ['--enable-unsafe-webgpu'] });
try {
  const page = await browser.newPage();
  const errors = []; page.on('pageerror', e => errors.push(String(e)));
  await page.goto(`http://127.0.0.1:${server.address().port}`);
  const evidence = await page.evaluate(async () => {
    const { BrowserGpuExecutor } = await import('/executor.js');
    const adapter = await navigator.gpu?.requestAdapter({ powerPreference: 'high-performance', forceFallbackAdapter: false });
    if (!adapter) throw new Error('Actual WebGPU adapter required');
    const info = adapter.info;
    if (info.isFallbackAdapter || /swiftshader|llvmpipe|software/i.test(info.description + info.device)) throw new Error('Software GPU cannot satisfy hardware evidence');
    const shader = await (await fetch('/compose.wgsl')).text();
    const vectors = await (await fetch('/vectors.json')).json();
    const renderer = await BrowserGpuExecutor.create(adapter, shader);
    const records = [];
    function ensure(ok, message) { if (!ok) throw new Error(message); }
    async function rejects(fn) { let rejected = false; try { await fn(); } catch { rejected = true; } ensure(rejected, 'unexpected success'); }
    try {
      for (const [index, v] of vectors.results.entries()) {
        const images = new Map();
        for (const i of v.program.images) {
          const input = v.inputs.find(p => p.id === i.id);
          images.set(i.id, await renderer.upload(i.width, i.height, i.hasHdr, new Float32Array(input.pixels.flat())));
        }
        const pending = renderer.render(v.program, images);
        for (const input of images.values()) input.close(); // actual submitted work must retain its capability
        const frame = await pending;
        const pixels = await renderer.readback(frame);
        let maximum = 0, squared = 0;
        ensure(pixels.length === v.linear.length, 'readback length');
        for (let n = 0; n < pixels.length; n++) {
          const e = Math.abs(pixels[n] - v.linear[n]); maximum = Math.max(maximum, e); squared += e * e;
        }
        const rms = Math.sqrt(squared / pixels.length);
        ensure(maximum <= 2e-5 && rms <= 2e-6, `pixel difference ${maximum}/${rms}`);
        frame.close(); frame.close();
        ensure(renderer.allocatedBytes === 0, 'frame allocation leaked');
        await rejects(() => renderer.readback(frame));
        records.push({ index, width: v.program.width, height: v.program.height, maximum, rms, values: pixels.length });
      }
      await rejects(() => renderer.upload(1, 1, false, new Float32Array([NaN, 0, 0, 1])));
      await rejects(() => renderer.upload(1, 1, false, new Float32Array([1, 0, 0, 0])));
      ensure(renderer.allocatedBytes === 0, 'invalid upload leaked');
      const v = vectors.results[0], images = new Map();
      for (const i of v.program.images) {
        images.set(i.id, await renderer.upload(i.width, i.height, i.hasHdr, new Float32Array(v.inputs.find(p => p.id === i.id).pixels.flat())));
      }
      const baseline = renderer.allocatedBytes;
      const changes = [p => { p.output = 123; }, p => { p.allocatedBytes++; }, p => { p.images[0].width++; },
        p => { p.commands.find(c => c.kind === 'paint').parameters[0]++; }, p => { p.commands.push(p.commands[0]); }];
      for (const change of changes) {
        const bad = structuredClone(v.program); change(bad);
        await rejects(() => renderer.render(bad, images));
        ensure(renderer.allocatedBytes === baseline, 'invalid program leaked');
      }
      const frame = await renderer.render(v.program, images); frame.close();
      for (const i of images.values()) i.close();
      ensure(renderer.allocatedBytes === 0, 'recovery leaked');
      const budgetAdapter = await navigator.gpu.requestAdapter({ forceFallbackAdapter: false });
      ensure(!!budgetAdapter, "budget-test adapter unavailable");
      const small = await BrowserGpuExecutor.create(budgetAdapter, shader, 512);
      const large = new Float32Array(8 * 8 * 4); large.fill(1);
      await rejects(() => small.upload(8, 8, false, large));
      ensure(small.allocatedBytes === 0, 'budget rejection leaked');
      const retained = new Map();
      for (const i of v.program.images) retained.set(i.id, await small.upload(i.width, i.height, i.hasHdr, new Float32Array(v.inputs.find(p => p.id === i.id).pixels.flat())));
      const retainedBytes = small.allocatedBytes;
      await rejects(() => small.render(v.program, retained));
      ensure(small.allocatedBytes === retainedBytes, 'partially allocated frame leaked');
      for (const i of retained.values()) i.close();
      ensure(small.allocatedBytes === 0, 'retained source leaked'); small.close();
      const cancelled = new AbortController(); cancelled.abort();
      await rejects(() => renderer.upload(1, 1, false, new Float32Array([0, 0, 0, 1]), cancelled.signal));
      return { adapter: { vendor: info.vendor, architecture: info.architecture, device: info.device, description: info.description, isFallbackAdapter: info.isFallbackAdapter }, records,
        invalidUploads: 2, invalidPrograms: changes.length, budgetRejection: true, closedImageRejection: true, cancelledBeforeAllocation: true,
        allOwnedBuffersReleased: renderer.allocatedBytes === 0 };
    } finally { renderer.close(); }
  });
  assert.deepEqual(errors, []);
  const sha = v => createHash('sha256').update(v).digest('hex');
  const report = { format: 'musteroffice.video-browser-gpu-check/1', status: 'passed', browser: browser.version(), platform: process.platform, arch: process.arch,
    files: Array.from(files, ([name, data]) => ({ name, sha256: sha(data) })), ...evidence,
    scope: 'Real browser WebGPU execution of Rust GPU programs using shared WGSL and explicit linear input, against native CPU pixels. Includes ownership release, malformed input, bounds and recovery. Not WebCodecs color conversion, actual source video, presentation, zero-copy, device-loss recovery, performance or product acceptance.' };
  await writeFile(output + '/report.json', JSON.stringify(report, null, 2) + '\n'); console.log(JSON.stringify(report));
} catch (e) { await writeFile(output + '/failure.txt', String(e.stack ?? e)); throw e; }
finally { await browser.close(); await new Promise(resolve => server.close(resolve)); }
