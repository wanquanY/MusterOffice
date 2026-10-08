/** Actual public WASM + hardware WebGPU + WebCodecs frame storage, no mocks. */
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { createServer } from 'node:http';
import { resolve } from 'node:path';
import { chromium } from 'playwright-core';
import { createHash } from 'node:crypto';

const [deliveryPath, vectorsPath, outputPath] = process.argv.slice(2);
assert(deliveryPath && vectorsPath && outputPath, 'DELIVERY NATIVE_VECTORS NEW_OUTPUT required');
const root = resolve(deliveryPath), output = resolve(outputPath); await mkdir(output);
const vectorBytes = await readFile(vectorsPath), manifestBytes = await readFile(root + '/bundle-manifest.json');
const manifest = JSON.parse(manifestBytes), routes = new Map([['/vectors.json', vectorBytes]]);
for (const file of manifest.files) if (/\.(js|mjs|wasm)$/.test(file.path)) routes.set('/sdk/' + file.path, await readFile(root + '/' + file.path));
const server = createServer((req, res) => {
  if (req.url === '/') { res.writeHead(200, { 'Content-Type': 'text/html' }); res.end('<!doctype html><title>I420 output verification</title>'); return; }
  const bytes = routes.get(req.url); if (!bytes) { res.writeHead(404); res.end(); return; }
  res.writeHead(200, { 'Content-Type': req.url.endsWith('.wasm') ? 'application/wasm' : /\.m?js$/.test(req.url) ? 'text/javascript' : 'application/json' }); res.end(bytes);
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const browser = await chromium.launch({ executablePath: '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', headless: true, args: ['--enable-unsafe-webgpu'] });
const diagnostics = [];
try {
  const page = await browser.newPage(), errors = [];
  page.on('pageerror', error => errors.push(String(error)));
  page.on('console', message => { if (message.type() === 'error' || message.type() === 'warning') diagnostics.push(message.text()); });
  await page.goto(`http://127.0.0.1:${server.address().port}`);
  const result = await page.evaluate(async () => {
    const { createVisualRuntime } = await import('/sdk/index.mjs');
    const { BrowserGpuExecutor } = await import('/sdk/client/index.js');
    const runtime = await createVisualRuntime(await WebAssembly.compileStreaming(fetch('/sdk/runtime/mo_video_visual_wasm_bg.wasm')));
    const vectors = await (await fetch('/vectors.json')).json();
    const ensure = (ok, message) => { if (!ok) throw new Error(message); };
    const same = (a, b, message) => ensure(JSON.stringify(a) === JSON.stringify(b), message);
    const rejects = async run => { let failed = false; try { await run(); } catch { failed = true; } ensure(failed, 'unexpected success'); };
    same(runtime.i420_output_shader(), vectors.shader, 'native/WASM shader');
    const adapter = await navigator.gpu.requestAdapter({ forceFallbackAdapter: false });
    ensure(adapter && !adapter.info.isFallbackAdapter, 'hardware GPU required');
    const gpu = await BrowserGpuExecutor.create(adapter, runtime.composition_shader());
    await gpu.configureI420(runtime.i420_output_shader());
    const records = [];
    try {
      for (const v of vectors.cases) {
        const words = JSON.parse(runtime.i420_output_parameters(v.width, v.height, v.hdr, v.peak ?? undefined, new Uint8Array(v.matte)));
        same(words, v.parameters, 'native/WASM output ABI');
        const image = await gpu.upload(v.width, v.height, v.hdr, new Float32Array(v.linear.flat()));
        const baseline = gpu.allocatedBytes;
        for (const change of [w => w[0]++, w => w[40] += 4, w => w[42]++, w => w[24] = 0x7fc00000]) {
          const altered = [...words]; change(altered); await rejects(() => gpu.readI420(image, altered));
          ensure(gpu.allocatedBytes === baseline, 'invalid ABI leaked');
        }
        const pending = gpu.readI420(image, words); image.close();
        const encoded = await pending;
        ensure(gpu.allocatedBytes === 0, 'GPU output allocation leak');
        let maxCode = 0, squared = 0, samples = 0, padding = 0;
        for (let p = 0; p < 3; p++) {
          const width = p === 0 ? v.width : v.width / 2, height = p === 0 ? v.height : v.height / 2;
          const layout = encoded.layout[p];
          for (let y = 0; y < height; y++) for (let x = 0; x < layout.stride; x++) {
            const code = encoded.data[layout.offset + y * layout.stride + x];
            if (x >= width) { ensure(code === 0, 'nonzero padding'); padding++; continue; }
            const diff = Math.abs(code - v.planes[p][y * width + x]); maxCode = Math.max(maxCode, diff); squared += diff * diff; samples++;
          }
        }
        ensure(maxCode <= 1, `${v.name} CPU/GPU output differs by ${maxCode}`);
        const frame = new VideoFrame(encoded.data, { format: 'I420', codedWidth: v.width, codedHeight: v.height,
          layout: encoded.layout, timestamp: 33367, duration: 33366,
          colorSpace: { primaries: 'bt709', transfer: 'bt709', matrix: 'bt709', fullRange: false } });
        try {
          ensure(frame.format === 'I420' && frame.colorSpace.fullRange === false, 'codec frame format');
          const copied = new Uint8Array(frame.allocationSize());
          const copiedLayout = await frame.copyTo(copied);
          for (let p = 0; p < 3; p++) {
            const width = p === 0 ? v.width : v.width / 2, height = p === 0 ? v.height : v.height / 2;
            for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) {
              ensure(copied[copiedLayout[p].offset + y * copiedLayout[p].stride + x] === encoded.data[encoded.layout[p].offset + y * encoded.layout[p].stride + x], 'VideoFrame changed I420 sample');
            }
          }
        } finally { frame.close(); }
        records.push({ name: v.name, samples, padding, maxCode, rms: Math.sqrt(squared / samples) });
      }
      await rejects(() => runtime.i420_output_parameters(2, 2, true, undefined, new Uint8Array([0, 0, 0])));
      await rejects(() => runtime.i420_output_parameters(3, 2, false, undefined, new Uint8Array([0, 0, 0])));
      const smallAdapter = await navigator.gpu.requestAdapter({ forceFallbackAdapter: false });
      const small = await BrowserGpuExecutor.create(smallAdapter, runtime.composition_shader(), 5000);
      try {
        await small.configureI420(runtime.i420_output_shader());
        const v = vectors.cases.find(v => v.width === 18 && v.name.endsWith('/gradient'));
        const image = await small.upload(v.width, v.height, v.hdr, new Float32Array(v.linear.flat()));
        const baseline = small.allocatedBytes;
        await rejects(() => small.readI420(image, v.parameters));
        ensure(small.allocatedBytes === baseline, 'partial allocation failure leak');
        image.close(); ensure(small.allocatedBytes === 0, 'input release leak');
      } finally { small.close(); }
      const v = vectors.cases[2];
      const image = await gpu.upload(v.width, v.height, v.hdr, new Float32Array(v.linear.flat()));
      const controller = new AbortController(), operation = gpu.readI420(image, v.parameters, controller.signal);
      controller.abort(); await rejects(() => operation); image.close();
      ensure(gpu.allocatedBytes === 0, 'cancelled output leak');
      return { adapter: { vendor: adapter.info.vendor, architecture: adapter.info.architecture, isFallbackAdapter: adapter.info.isFallbackAdapter }, records,
        checks: ['actual hardware GPU', 'native/WASM ABI', 'CPU reference I420', 'alpha/matte/HDR/Lanczos/row padding',
          'VideoFrame raw sample preservation', 'invalid ABI', 'bounded partial allocation cleanup', 'closed input retained during output', 'cancellation'] };
    } finally { gpu.close(); }
  });
  assert.deepEqual(errors, []);
  const hash = bytes => createHash('sha256').update(bytes).digest('hex');
  const report = { status: 'passed', browser: browser.version(), manifestSha256: hash(manifestBytes), vectorsSha256: hash(vectorBytes), ...result,
    scope: 'Actual GPU television-output and VideoFrame storage. Does not establish encoded codec packets, mux correctness or product export acceptance.' };
  await writeFile(output + '/report.json', JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify({ status: report.status, browser: report.browser, cases: report.records.length, maxCode: Math.max(...report.records.map(r => r.maxCode)), manifestSha256: report.manifestSha256 }));
} catch (error) { await writeFile(output + '/failure.txt', String(error.stack ?? error) + '\n' + diagnostics.join('\n')); throw error; }
finally { await browser.close(); await new Promise(resolve => server.close(resolve)); }
