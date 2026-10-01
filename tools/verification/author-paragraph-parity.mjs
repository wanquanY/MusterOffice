// Use the cases emitted by the author_paragraph_layout Rust integration test.
// Bind explicit runtime inputs; historical evidence and source files stay intact.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
import {pathToFileURL} from 'node:url';

const args = process.argv.slice(2);
assert.equal(args.length, 9,
  'worker wasm-node-dir text-adapter raster-adapter harfbuzz-dir skia-dir cases-dir font-file output-dir');
const [worker, wasmDir, textAdapter, rasterAdapter, hbDir, skiaDir, casesDir, fontFile, output] = args.map(p => path.resolve(p));
assert(!fs.existsSync(output), 'output must be a new directory');
const load = p => import(pathToFileURL(p));
const wasm = createRequire(import.meta.url)(path.join(wasmDir, 'mo_wasm.js'));
const {ShapingComponent} = await load(textAdapter);
const {RasterComponent} = await load(rasterAdapter);
const {default: hbFactory} = await load(path.join(hbDir, 'mo-hb.mjs'));
const {default: skiaFactory} = await load(path.join(skiaDir, 'mo-skia.mjs'));
const shaper = await ShapingComponent.create(hbFactory,
  new WebAssembly.Module(fs.readFileSync(path.join(hbDir, 'mo-hb.wasm'))));
const raster = await RasterComponent.create(skiaFactory,
  new WebAssembly.Module(fs.readFileSync(path.join(skiaDir, 'mo-skia.wasm'))));
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const entry = file => {
  const bytes = fs.readFileSync(file);
  return {path: file, byteLength: bytes.length, sha256: sha(bytes)};
};
const fonts = fs.readFileSync(fontFile);
const names = ['single', 'one-and-half', 'double', 'exact'];
assert.deepEqual(fs.readdirSync(casesDir).filter(n => n.endsWith('.pptx')).sort(), names.map(n => `${n}.pptx`).sort());
fs.mkdirSync(output, {recursive: true});
const records = [];
for (const name of names) {
  const prefix = path.join(casesDir, name);
  const json = fs.readFileSync(`${prefix}.request.json`);
  const source = fs.readFileSync(`${prefix}.pptx`);
  const head = Buffer.alloc(12);
  head.writeUInt32LE(json.length);
  head.writeUInt32LE(source.length, 4);
  head.writeUInt32LE(fonts.length, 8);
  const native = spawnSync(worker, ['--pptx-text-page'], {
    input: Buffer.concat([head, json, source, fonts]), timeout: 60_000, maxBuffer: 80 * 1024 * 1024,
  });
  assert.equal(native.status, 0, native.stderr?.toString());
  assert(native.stdout.length >= 8);
  const metadataLength = native.stdout.readUInt32LE();
  const pixelsLength = native.stdout.readUInt32LE(4);
  assert.equal(native.stdout.length, 8 + metadataLength + pixelsLength);
  const metadata = native.stdout.subarray(8, 8 + metadataLength).toString();
  const pixels = native.stdout.subarray(8 + metadataLength);
  const result = wasm.render_pptx_text_page(json.toString(), source, fonts, shaper, raster);
  let consumed = false;
  try {
    assert.equal(result.metadata, metadata, name);
    // take_pixels consumes the Rust owner, even when a later assertion fails.
    consumed = true;
    assert.deepEqual(Buffer.from(result.take_pixels()), pixels, name);
  } finally {
    if (!consumed) result.free();
  }
  const response = JSON.parse(metadata);
  assert.equal(response.status, 'rendered', name);
  assert.equal(response.info.page.scene.raster.sha256, sha(pixels));
  assert.deepEqual(pixels, fs.readFileSync(`${prefix}.rgba`), `${name}: authored library vs exported file`);
  const responseFile = path.join(output, `${name}.response.json`);
  fs.writeFileSync(responseFile, metadata, {flag: 'wx'});
  records.push({name, request: entry(`${prefix}.request.json`), source: entry(`${prefix}.pptx`),
    expectedPixels: entry(`${prefix}.rgba`), response: entry(responseFile), pixelBytes: pixels.length,
    pixelsSha256: sha(pixels)});
}
assert(!shaper.invalid);
assert(!raster.invalid);
const artifacts = [worker, path.join(wasmDir, 'mo_wasm.js'), path.join(wasmDir, 'mo_wasm_bg.wasm'),
  textAdapter, rasterAdapter, path.join(hbDir, 'mo-hb.mjs'), path.join(hbDir, 'mo-hb.wasm'),
  path.join(skiaDir, 'mo-skia.mjs'), path.join(skiaDir, 'mo-skia.wasm'), fontFile].map(entry);
const report = {format: 'musteroffice.author-paragraph-parity/1', status: 'passed', artifacts, cases: records};
fs.writeFileSync(path.join(output, 'report.json'), JSON.stringify(report, null, 2) + '\n', {flag: 'wx'});
console.log(JSON.stringify({status: report.status, pairedCalls: records.length}));
