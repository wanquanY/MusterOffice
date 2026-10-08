/** Real browser input/output color kernels, owned frames and GPU presentation. */
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { createServer } from 'node:http';
import { resolve } from 'node:path';
import { chromium } from 'playwright-core';
import { createHash } from 'node:crypto';
const [deliveryPath, vectorsPath, outputPath, rawVectorsPath] = process.argv.slice(2);
assert(deliveryPath && vectorsPath && outputPath, 'usage: delivery vectors output');
const root = resolve(deliveryPath), output = resolve(outputPath); await mkdir(output);
const routes = new Map([['/vectors.json', await readFile(vectorsPath)]]);
if (rawVectorsPath) routes.set('/raw-vectors.json', await readFile(rawVectorsPath));
const manifest = JSON.parse(await readFile(root + '/bundle-manifest.json'));
for (const file of manifest.files) if (/\.(js|mjs|wasm)$/.test(file.path)) routes.set('/sdk/' + file.path, await readFile(root + '/' + file.path));
const server = createServer((req, res) => {
  if (req.url === '/') { res.writeHead(200, { 'Content-Type': 'text/html' }); res.end('<!doctype html><title>GPU color verification</title>'); return; }
  const data = routes.get(req.url); if (!data) { res.writeHead(404); res.end(); return; }
  res.writeHead(200, { 'Content-Type': req.url.endsWith('.wasm') ? 'application/wasm' : /\.m?js$/.test(req.url) ? 'text/javascript' : 'application/json' }); res.end(data);
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const browser = await chromium.launch({ executablePath: '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', headless: true, args: ['--enable-unsafe-webgpu'] });
try {
  const page = await browser.newPage(); const errors = []; page.on('pageerror', e => errors.push(String(e)));
  await page.goto(`http://127.0.0.1:${server.address().port}`);
  const result = await page.evaluate(async hasRawVectors => {
    const { createVisualRuntime } = await import('/sdk/index.mjs');
    const { BrowserGpuExecutor } = await import('/sdk/client/index.js');
    const runtime = await createVisualRuntime(await WebAssembly.compileStreaming(fetch('/sdk/runtime/mo_video_visual_wasm_bg.wasm')));
    const v = await (await fetch('/vectors.json')).json();
    const ensure = (value, message) => { if (!value) throw new Error(message); };
    const sorted = x => Array.isArray(x) ? x.map(sorted) : x && typeof x==='object' ? Object.fromEntries(Object.keys(x).sort().map(k=>[k,sorted(x[k])])) : x;
    const same = (a, b, message) => ensure(JSON.stringify(sorted(a)) === JSON.stringify(sorted(b)), message);
    same(JSON.parse(runtime.color_shaders()), v.shaders, 'native/WASM shader difference');
    same(JSON.parse(runtime.srgb_output_parameters(6, 4, false, undefined, false)), v.straightParameters, 'output ABI');
    const adapter = await navigator.gpu.requestAdapter({ forceFallbackAdapter: false }); ensure(!!adapter, 'real GPU unavailable');
    ensure(!adapter.info.isFallbackAdapter, 'fallback GPU');
    const gpu = await BrowserGpuExecutor.create(adapter, runtime.composition_shader());
    await gpu.configureColor(JSON.parse(runtime.color_shaders()));
    let closes = 0;
    const cases = [];
    const pixels = new Uint8Array(8 * 6 * 4);
    for (let y = 0; y < 4; y++) for (let x = 0; x < 6; x++) pixels.set(v.rgba[y * 6 + x], ((y + 2) * 8 + x + 2) * 4);
    const color = { primaries: 'bt709', transfer: 'iec61966-2-1', matrix: 'rgb', fullRange: true };
    try {
      for (const rotation of [0, 90, 180, 270]) for (const flip of [false, true]) {
        const frame = new VideoFrame(pixels, { format: 'RGBA', codedWidth: 8, codedHeight: 6, visibleRect: { x: 2, y: 2, width: 6, height: 4 },
          displayWidth: 18, displayHeight: 8, rotation, flip, timestamp: 0, colorSpace: color });
        const lease = { frame, close() { frame.close(); closes++; } };
        const imported = await gpu.importVideo(lease, JSON.parse(runtime.video_input_plan(JSON.stringify({ width: 6, height: 4, format: frame.format, colorSpace: frame.colorSpace.toJSON() }))));
        ensure(frame.codedWidth === 0, 'consumed lease not closed');
        const linear = await gpu.readback(imported), reference = v.linear.flat();
        let maximum = 0, squared = 0;
        for (let i = 0; i < linear.length; i++) { const d = Math.abs(linear[i] - reference[i]); maximum = Math.max(maximum, d); squared += d * d; }
        const rms = Math.sqrt(squared / linear.length);
        ensure(maximum < 2e-5 && rms < 2e-6, `input colors ${rotation}/${flip}: ${maximum}/${rms}; actual=${JSON.stringify(Array.from(linear))}; expected=${JSON.stringify(reference)}`);
        const rgba = await gpu.readSrgb8(imported, v.straightParameters);
        const maxCode = Math.max(...rgba.map((b, i) => Math.abs(b - v.straight[i])));
        ensure(maxCode <= 1, `output RGBA8: ${maxCode}`);
        const bitmap = await gpu.toBitmap(imported, v.premultipliedParameters);
        const canvas = new OffscreenCanvas(6, 4), context = canvas.getContext('2d', { colorSpace: 'srgb', willReadFrequently: true });
        context.drawImage(bitmap.bitmap, 0, 0);
        const displayed = context.getImageData(0, 0, 6, 4).data;
        let displayError = 0;
        for (let p = 0; p < displayed.length; p += 4) {
          ensure(Math.abs(displayed[p + 3] - v.premultiplied[p + 3]) <= 1, 'canvas alpha');
          for (let c = 0; c < 3; c++) displayError = Math.max(displayError, Math.abs(displayed[p + c] * displayed[p + 3] / 255 - v.premultiplied[p + c]));
        }
        ensure(displayError <= 1.5, `canvas premultiplied error ${displayError}`);
        bitmap.close(); bitmap.close(); imported.close();
        ensure(gpu.allocatedBytes === 0 && gpu.retainedSurfaceBytes === 6 * 4 * 4, 'input/output allocations leaked');
        cases.push({ rotation, flip, maximum, rms, maxCode, displayError });
      }
      const rawCases = [];
      if (hasRawVectors) for (const row of await (await fetch('/raw-vectors.json')).json()) {
        const plan = JSON.parse(runtime.video_input_plan(JSON.stringify(row.plan.source)));
        same(plan,row.plan,'native/WASM raw plan');
        const frame = new VideoFrame(new Uint8Array(row.bytes), {format:plan.source.format,codedWidth:plan.source.width,codedHeight:plan.source.height,
          colorSpace:plan.source.colorSpace,layout:plan.planes,timestamp:0});
        const imported = await gpu.importVideo({frame,close(){frame.close();closes++;}},plan);
        const actual = await gpu.readback(imported), expected = row.linear.flat();
        let maximum=0, scaled=0,square=0;
        for(let i=0;i<actual.length;i++){const d=Math.abs(actual[i]-expected[i]);maximum=Math.max(maximum,d);scaled=Math.max(scaled,d/Math.max(1,Math.abs(expected[i])));square+=d*d;}
        ensure(scaled<0.0001, row.name+' raw input mismatch '+maximum+'/'+scaled);
        imported.close();
        rawCases.push({name:row.name,maximum,scaled,rms:Math.sqrt(square/actual.length)});
        if(row.name==='I420P10/bt709') {
          const bytes=new Uint8Array(row.bytes);bytes[0]=255;bytes[1]=255;
          const corrupt=new VideoFrame(bytes,{format:plan.source.format,codedWidth:plan.source.width,codedHeight:plan.source.height,colorSpace:plan.source.colorSpace,layout:plan.planes,timestamp:0});
          let rejected=false;try{(await gpu.importVideo({frame:corrupt,close(){corrupt.close();closes++;}},plan)).close();}catch(e){rejected=String(e).includes('bit depth');}
          ensure(rejected && gpu.allocatedBytes===0,'high-bit-depth rejection/resource cleanup');
        }
      }
      const hdr = await gpu.upload(2, 2, true, new Float32Array(v.hdr.linear.flat()));
      const actualHdr = await gpu.readSrgb8(hdr, v.hdr.parameters);
      const hdrError = Math.max(...actualHdr.map((b, i) => Math.abs(b - v.hdr.rgba[i]))); ensure(hdrError <= 1, 'HDR tone map output'); hdr.close();
      let rejected = 0;
      try { runtime.srgb_output_parameters(2, 2, true, undefined, false); } catch { rejected++; }
      const closingFrame=new VideoFrame(new Uint8Array(v.rgba.flat()),{format:'RGBA',codedWidth:6,codedHeight:4,timestamp:0,colorSpace:color});
      const closingPlan=JSON.parse(runtime.video_input_plan(JSON.stringify({width:6,height:4,format:'RGBA',colorSpace:color})));
      let closeRejected=false;
      try {await gpu.importVideo({frame:closingFrame,close(){closingFrame.close();closes++;throw Error('lease close failure');}},closingPlan);}catch(e){closeRejected=String(e).includes('lease close failure');}
      ensure(closeRejected && gpu.allocatedBytes===0,'failed lease close retained unpublished image');
      const controller = new AbortController(); controller.abort();
      const frame = new VideoFrame(pixels, { format: 'RGBA', codedWidth: 8, codedHeight: 6, timestamp: 0, colorSpace: color });
      try { await gpu.importVideo({ frame, close() { frame.close(); closes++; } }, null, controller.signal); } catch { rejected++; }
      ensure(frame.codedWidth === 0 && rejected === 2, 'cancelled lease or missing tone policy accepted');
      gpu.close(); ensure(gpu.allocatedBytes === 0 && gpu.retainedSurfaceBytes === 0, 'closed GPU retained charges');
      return { adapter: { vendor: adapter.info.vendor, architecture: adapter.info.architecture, isFallbackAdapter: adapter.info.isFallbackAdapter }, cases, rawCases, hdrError, rejected, closedLeases: closes };
    } finally { gpu.close(); }
  }, !!rawVectorsPath);
  assert.deepEqual(errors, []);
  const sha = value => createHash('sha256').update(value).digest('hex');
  const report = { format: 'musteroffice.video-browser-color-check/1', status: 'passed', browser: browser.version(), manifestSha256: sha(await readFile(root + '/bundle-manifest.json')),
    rawVectorsSha256: rawVectorsPath ? sha(await readFile(rawVectorsPath)) : null, vectorsSha256: sha(await readFile(vectorsPath)), ...result, scope: 'Actual shared WASM color ABI + Chrome GPU VideoFrame visible crop/orientation/SAR/straight-alpha normalization, sRGB output, HDR explicit output tone map and GPU ImageBitmap presentation vs native CPU vectors. Synthetic RGBA and raw YUV/alpha/8-12 bit/HDR frame vectors; actual decoded codec video checked separately.' };
  await writeFile(output + '/report.json', JSON.stringify(report, null, 2) + '\n'); console.log(JSON.stringify(report));
} catch (e) { await writeFile(output + '/failure.txt', String(e.stack ?? e)); throw e; }
finally { await browser.close(); await new Promise(resolve => server.close(resolve)); }
