/** Compare real native and WASM capacity measurements and rendered pixels.
 * Inputs are owned PPTX/expected-capacity pairs emitted by source_capacity.rs.
 * This checks shape-local capacity, not visual design or Office compatibility.
 */
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, mkdirSync, writeFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { createRequire } from 'node:module';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';

const [corpus, worker, wasmPath, textPath, rasterPath, hbRoot, skiaRoot, output] = process.argv.slice(2).map(path => resolve(path));
assert.ok(output, 'expected corpus worker wasm.js text.js raster.js harfbuzzDir skiaDir output');
const wasm = createRequire(import.meta.url)(wasmPath);
const { ShapingComponent } = await import(pathToFileURL(textPath));
const { RasterComponent } = await import(pathToFileURL(rasterPath));
const { default: hbFactory } = await import(pathToFileURL(join(hbRoot, 'mo-hb.mjs')));
const { default: skiaFactory } = await import(pathToFileURL(join(skiaRoot, 'mo-skia.mjs')));
const text = await ShapingComponent.create(hbFactory, new WebAssembly.Module(readFileSync(join(hbRoot, 'mo-hb.wasm'))));
const raster = await RasterComponent.create(skiaFactory, new WebAssembly.Module(readFileSync(join(skiaRoot, 'mo-skia.wasm'))));
const fonts = readFileSync('fixtures/fonts/owned.ttf');
const manifest = JSON.parse(readFileSync('fixtures/fonts/manifest-paragraph.json')).manifest;
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const file = path => ({ sha256: hash(readFileSync(path)), byteLength: readFileSync(path).length });
mkdirSync(output, { recursive: false });
const cases = [];
try {
  for (const name of readdirSync(corpus).filter(n => n.endsWith('.pptx')).sort()) {
    const source = readFileSync(join(corpus, name));
    const expected = JSON.parse(readFileSync(join(corpus, name.replace('.pptx', '.json'))));
    for (const mode of ['text', 'resource']) {
      const request = {
        profile: mode === 'text' ? 'drawingml-solid-text-page-q32-draft-v1' : 'drawingml-resource-page-q32-v1-draft',
        page: {
          expectedSourceSha256: hash(source), slide: '/ppt/slides/slide1.xml', profile: 'drawingml-static-solid-page-v1-draft',
          colorContext: { systemColors: {}, placeholder: null },
          viewport: { width: 640, height: 360, origin: { x: '0', y: '0' }, scale: { numerator: 1, denominator: 19050 }, coordinateTolerance: '16777216', background: [255,255,255,255] },
        }, fonts: manifest,
        ...(mode === 'resource' ? { imageSource: 'embeddedSnapshot', sampling: 'nearest' } : {}),
      };
      const json = JSON.stringify(request), bytes = Buffer.from(json), header = Buffer.alloc(12);
      [bytes.length, source.length, fonts.length].forEach((n, i) => header.writeUInt32LE(n, 4 * i));
      const native = spawnSync(worker, [`--pptx-${mode}-page`], { input: Buffer.concat([header, bytes, source, fonts]), timeout: 60000, maxBuffer: 80 << 20 });
      assert.ifError(native.error); assert.equal(native.status, 0, native.stderr.toString());
      const size = native.stdout.readUInt32LE(), pixelSize = native.stdout.readUInt32LE(4);
      assert.equal(native.stdout.length, 8 + size + pixelSize);
      const metadata = native.stdout.subarray(8, 8 + size).toString(), pixels = native.stdout.subarray(8 + size);
      const remote = mode === 'text'
        ? wasm.render_pptx_text_page(json, source, fonts, text, raster)
        : wasm.render_pptx_resource_page(json, source, fonts, raster, text, raster);
      assert.deepEqual(JSON.parse(remote.metadata), JSON.parse(metadata));
      assert.deepEqual(Buffer.from(remote.take_pixels()), pixels);
      const response = JSON.parse(metadata);
      assert.equal(response.status, 'rendered', metadata);
      assert.equal(response.info.textCapacity.profile, 'drawingml-text-capacity-q32-v1-draft');
      assert.equal(response.info.textCapacity.frames.length, response.info.textFrames);
      assert.deepEqual(response.info.textCapacity.frames.find(f => f.object.part === expected.object.part && f.object.nativeId === expected.object.nativeId), expected);
      const stem = `${name.slice(0, -5)}-${mode}`;
      writeFileSync(join(output, stem + '.json'), metadata, { flag: 'wx' });
      cases.push({ sourceSha256: hash(source), mode, responseSha256: hash(Buffer.from(metadata)), pixelSha256: hash(pixels), pixelBytes: pixels.length, capacity: response.info.textCapacity });
    }
  }
  assert.ok(cases.length >= 12);
  const report = { profile: 'native-wasm-text-capacity/1', cases, worker: file(worker), wasm: file(wasmPath.replace(/\.js$/, '_bg.wasm')), font: file('fixtures/fonts/owned.ttf'), qualityProven: false };
  writeFileSync(join(output, 'report.json'), JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
  console.log(JSON.stringify({ pairedCalls: cases.length, exactMeasurementsAndPixels: true, qualityProven: false }));
} finally {
  text.invalidate(); raster.invalidate();
}
