/** Current Native/WASM rendering and thin-client parity across viewport changes.
 * Inputs are local owned fixture manifests with digest-pinned bytes. */
import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {spawn} from 'node:child_process';
import {createRequire} from 'node:module';
import {pathToFileURL, fileURLToPath} from 'node:url';

const [output, wasmDir, nativeWorker, skiaDir, hbDir, adapters, authorManifest, sourceManifest, inputMode] = process.argv.slice(2);
assert(sourceManifest, 'output wasm-dir native-worker skia-dir hb-dir adapters-dir author-manifest source-manifest');
assert(inputMode === undefined || inputMode === 'presentation-step', 'optional input mode: presentation-step');
await fs.mkdir(output, {recursive: false});
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const evidence = {format:'musteroffice.playback-viewport-parity/1', inputMode:inputMode ?? 'fixture', inputs:{}, cases:[]};
async function read(file) {
  const bytes = await fs.readFile(file);
  const actual = {byteLength:bytes.length, sha256:sha(bytes)};
  if (evidence.inputs[file]) assert.deepEqual(actual, evidence.inputs[file], `input changed during run: ${file}`);
  else evidence.inputs[file] = actual;
  return bytes;
}
async function pinned(record) {
  const bytes = await read(record.path);
  assert.equal(bytes.length, record.byteLength); assert.equal(sha(bytes), record.sha256);
  return bytes;
}
for (const file of [nativeWorker, path.join(wasmDir,'mo_wasm.js'), path.join(wasmDir,'mo_wasm_bg.wasm'),
  path.join(skiaDir,'mo-skia.mjs'), path.join(hbDir,'mo-hb.mjs'), fileURLToPath(import.meta.url)]) await read(file);
async function recordAdapters(directory) {
  for (const item of await fs.readdir(directory, {withFileTypes:true})) {
    const file = path.join(directory, item.name);
    if (item.isDirectory()) await recordAdapters(file);
    else if (item.name.endsWith('.js')) await read(file);
  }
}
await recordAdapters(adapters);
const load = file => import(pathToFileURL(path.resolve(file)));
const wasm = createRequire(import.meta.url)(path.resolve(wasmDir, 'mo_wasm.js'));
const {WasmPlayback} = await load(path.join(adapters, 'playback/playback-client/src/index.js'));
const {RasterComponent} = await load(path.join(adapters, 'raster/index.js'));
const {ShapingComponent} = await load(path.join(adapters, 'text/index.js'));
const skiaFactory = (await load(path.join(skiaDir, 'mo-skia.mjs'))).default;
const hbFactory = (await load(path.join(hbDir, 'mo-hb.mjs'))).default;
const raster = await RasterComponent.create(skiaFactory, new WebAssembly.Module(await read(path.join(skiaDir, 'mo-skia.wasm'))));
const shaper = await ShapingComponent.create(hbFactory, new WebAssembly.Module(await read(path.join(hbDir, 'mo-hb.wasm'))));
let textCalls = 0, imageCalls = 0;
const text = Object.fromEntries(['shapeBatch','outlineBatch','measureBatch','registerFont','unregisterFont','shapeRegistered','measureRegistered','outlineRegistered'].map(name => [name, (...args) => {
  textCalls++; return shaper[name](...args);
}]));
text.invalidate = () => shaper.invalidate();
const decoder = {decodeImage(...args) { imageCalls++; return raster.decodeImage(...args); }, invalidate() { raster.invalidate(); }};
const sdk = new WasmPlayback(wasm);

function native(kind) {
  const child = spawn(nativeWorker, [kind === 'author' ? '--playback-session' : '--pptx-playback-session'], {env:{}, stdio:['pipe','pipe','pipe']});
  let buffer = Buffer.alloc(0), pending, stderr = '';
  child.stderr.on('data', bytes => stderr += bytes);
  const reject = error => { if (pending) { clearTimeout(pending.timer); pending.reject(error); pending = undefined; } };
  const exited = new Promise((resolve, fail) => {
    child.on('error', error => { reject(error); fail(error); });
    child.on('close', code => { reject(new Error(`native exited: ${code}: ${stderr}`)); resolve(code); });
  });
  child.stdout.on('data', bytes => {
    buffer = Buffer.concat([buffer, bytes]);
    if (!pending || buffer.length < 8) return;
    const metadata = buffer.readUInt32LE(), pixels = buffer.readUInt32LE(4);
    if (metadata > 64 * 1024 * 1024 || pixels > 256 * 1024 * 1024) {
      reject(new Error('native response limit')); child.kill(); return;
    }
    if (buffer.length < 8 + metadata + pixels) return;
    const result = {response:JSON.parse(buffer.subarray(8, 8 + metadata)), pixels:buffer.subarray(8 + metadata, 8 + metadata + pixels)};
    buffer = buffer.subarray(8 + metadata + pixels);
    const waiting = pending; pending = undefined; clearTimeout(waiting.timer); waiting.resolve(result);
  });
  return {
    send(request, source = Buffer.alloc(0), fonts = Buffer.alloc(0)) {
      assert.equal(pending, undefined);
      const json = Buffer.from(JSON.stringify(request));
      const header = Buffer.alloc(kind === 'author' ? 4 : 12); header.writeUInt32LE(json.length);
      if (kind === 'source') { header.writeUInt32LE(source.length, 4); header.writeUInt32LE(fonts.length, 8); }
      return new Promise((resolve, fail) => {
        pending = {resolve, reject:fail, timer:setTimeout(() => { reject(new Error('native timeout')); child.kill(); }, 30000)};
        child.stdin.write(Buffer.concat([header, json, source, fonts]));
      });
    },
    async close() { child.stdin.end(); assert.equal(await exited, 0, stderr); assert.equal(stderr, ''); assert.equal(buffer.length, 0); }
  };
}
async function run(kind, fixture) {
  const q = JSON.parse(await pinned(fixture.request));
  const request = kind === 'author' ? {
    snapshot:q.playback.snapshot, slide:q.playback.slide, binding:q.playback.binding, viewport:q.viewport, defaults:q.defaults
  } : {page:q.page, binding:q.sample.binding};
  if (kind === 'source') request.page.sampling = 'linear';
  const sample = kind === 'author' ? {binding:q.playback.binding, at:q.playback.at, history:q.playback.history} : q.sample;
  const source = kind === 'source' ? await pinned(fixture.source) : Buffer.alloc(0);
  const fonts = kind === 'source' ? await pinned(fixture.fonts) : Buffer.alloc(0);
  const inputs = {source, fonts, decoder, shaping:text};
  const worker = native(kind);
  let owner;
  const frames = [];
  try {
    owner = kind === 'author' ? sdk.prepareAuthor(request) : sdk.prepareSource(request, inputs);
    const initial = await worker.send({operation:'prepare', request}, source, fonts);
    assert.equal(initial.response.status, 'prepared'); assert.deepEqual(initial.response.info, owner.info);
    async function render(label) {
      const result = owner.sample(sample.at, raster, sample.history ?? null);
      const pair = await worker.send({operation:'render', sample});
      assert.equal(pair.response.status, 'rendered'); assert.deepEqual(result.info, pair.response.info);
      assert.deepEqual(Buffer.from(result.pixels), pair.pixels);
      assert.equal(result.viewportRevision, owner.info.viewportRevision);
      const evaluated = kind === 'author' ? result.info.frame : result.info.playback.evaluated;
      frames.push({label, sha256:sha(result.pixels), byteLength:result.pixels.length});
      return {result, evaluated};
    }
    const receipts = [];
    if (inputMode === 'presentation-step') {
      sample.at = {ticks:'0', timescale:1000};
      sample.history = {binding:sample.binding, through:sample.at, events:[]};
      const initial = await render('initial');
      assert.equal(initial.evaluated.state.presentationStep, undefined);
      for (const [index, direction] of ['next','next','next','next','previous','previous','previous'].entries()) {
        const sequence = index + 1, milliseconds = sequence * 100;
        sample.at = {ticks:String(milliseconds), timescale:1000};
        sample.history.through = sample.at;
        sample.history.events.push({at:sample.at, generation:sample.binding.generation, sequence,
          event:{kind:'presentationStep', direction}});
        const current = await render(`step-${sequence}`), state = current.evaluated.state;
        assert.deepEqual(state.binding, sample.binding);
        assert.equal(state.eventCursor, sequence);
        const receipt = state.presentationStep;
        assert.equal(receipt.sequence, sequence); assert.equal(receipt.direction, direction);
        assert.equal(BigInt(receipt.at.numerator) * 1000n, BigInt(milliseconds) * BigInt(receipt.at.denominator));
        assert(['consumed','pageBoundary'].includes(receipt.outcome.kind));
        if (receipt.outcome.kind === 'pageBoundary') assert.equal(receipt.outcome.entry, 'initial');
        receipts.push(receipt);
      }
    }
    const base = await render('before');
    const viewport = structuredClone(owner.info.viewport), enlarged = structuredClone(viewport);
    enlarged.width *= 2; enlarged.height *= 2; enlarged.scale.numerator *= 2;
    for (const [index, next] of [enlarged, viewport].entries()) {
      const timing = owner.timing(), calls = textCalls;
      const info = index === 0 ? owner.resizeToFit(next.width, next.height, inputs) : owner.resize(next, inputs);
      const paired = await worker.send({operation:index === 0 ? 'resizeToFit' : 'resize', binding:sample.binding, expectedViewportRevision:index,
        ...(index === 0 ? {width:next.width, height:next.height} : {viewport:next})}, source);
      assert.equal(paired.response.status, 'resized'); assert.deepEqual(paired.response.info, info);
      assert.deepEqual(owner.timing(), timing); assert.equal(textCalls, calls, 'resize must not shape, outline or upload fonts');
      const current = await render(index === 0 ? 'enlarged' : 'restored');
      assert.deepEqual(current.evaluated, base.evaluated, 'time, event cursor and animated properties');
      if (index === 1) assert.deepEqual(current.result.pixels, base.result.pixels);
      const freshRequest = structuredClone(request);
      if (kind === 'author') freshRequest.viewport = info.viewport; else freshRequest.page.page.viewport = info.viewport;
      const fresh = kind === 'author' ? sdk.prepareAuthor(freshRequest) : sdk.prepareSource(freshRequest, inputs);
      try {
        assert.equal(fresh.info.planId, owner.info.planId);
        const reference = fresh.sample(sample.at, raster, sample.history ?? null);
        assert.deepEqual(reference.pixels, current.result.pixels, 'resized output equals freshly prepared viewport');
      } finally { fresh.dispose(); }
    }
    evidence.cases.push({kind, name:fixture.name, frames, receipts, viewportRevisions:owner.info.viewportRevision, timing:owner.timing()});
  } finally { owner?.close(); await worker.close(); }
}
const authorCases = JSON.parse(await read(authorManifest)).cases;
const sourceCases = JSON.parse(await read(sourceManifest)).cases;
await run('author', authorCases.find(c => c.name === 'interactive-3'));
await run('source', sourceCases.find(c => c.name === 'image-text-1'));
await run('source', sourceCases.find(c => c.name === 'click-after'));
assert(!raster.invalid); assert(!shaper.invalid);
if (inputMode === 'presentation-step') {
  const outcomes = evidence.cases.flatMap(c => c.receipts.map(r => r.outcome.kind));
  assert(outcomes.includes('consumed')); assert(outcomes.includes('pageBoundary'));
}
for (const file of Object.keys(evidence.inputs)) await read(file);
evidence.componentCalls = {text:textCalls, image:imageCalls};
await fs.writeFile(path.join(output, 'report.json'), JSON.stringify(evidence, null, 2) + '\n');
console.log(JSON.stringify({cases:evidence.cases.length, pairedFrames:evidence.cases.reduce((n,c) => n+c.frames.length,0), freshViewportControls:6, resizeTextCalls:0}));
