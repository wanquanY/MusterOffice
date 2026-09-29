/** Verify caller-owned `mo-cli compute` output through native and WASM paths.
 * Re-renders every actual PPTX page; compares metadata, text measurements and
 * RGBA bytes. This is cross-runtime evidence, not Office/WPS visual acceptance.
 */
import assert from 'node:assert/strict';
import { readFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { createRequire } from 'node:module';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';

const args = process.argv.slice(2);
assert.equal(args.length, 9, 'delivery cli rasterWorker wasm.js text.js raster.js hbDir skiaDir output');
const [directory, cli, worker, modulePath, textPath, rasterPath, hbRoot, skiaRoot, output] = args.map(p => resolve(p));
const read = p => readFileSync(p);
const json = p => JSON.parse(read(p));
const hash = b => createHash('sha256').update(b).digest('hex');
const identity = p => { const b = read(p); return { sha256: hash(b), byteLength: b.length }; };
const saved = json(join(directory, 'result.json')).result;
assert.equal(saved.kind, 'exported');
const { bundle } = saved.receipt;
const inspection = json(join(directory, 'inspection.json'));
const files = json(join(directory, 'files.json'));
const assets = new Map(files.map(({ file, asset }) => {
  assert.equal(file, file.split('/').at(-1));
  const bytes = read(join(directory, file));
  assert.equal(hash(bytes), asset.sha256);
  assert.equal(String(bytes.length), asset.byteLength);
  assert.deepEqual(bundle.assets.find(a => a.id === asset.id), asset);
  return [asset.id, bytes];
}));
assert.equal(assets.size, bundle.assets.length);
const context = JSON.parse(assets.get(bundle.assets.find(a => a.mediaType === 'application/vnd.musteroffice.presentation-context+json').id));
const model = JSON.parse(assets.get(bundle.document.modelAssetId));
const source = assets.get(bundle.pptxAssetId);
const fonts = assets.get(context.fontBundleAssetId);
const evidenceByPage = new Map(bundle.assets.filter(a => a.role === 'quality-report')
  .map(a => JSON.parse(assets.get(a.id)))
  .filter(a => a.format === 'musteroffice.preview-evidence/3-draft')
  .map(a => [a.pageId, a]));
assert.ok(fonts, 'this verifier requires an explicit font bundle');
let offset = 0;
const request = {
  bundle,
  expected: {
    documentId: model.document.id, revision: model.revision, semanticDigest: model.semanticDigest,
    settingsDigest: inspection.settingsDigest, renderer: context.previewRenderer,
  },
  contents: [...assets].map(([assetId, bytes]) => {
    const entry = { assetId, byteOffset: String(offset), byteLength: String(bytes.length) };
    offset += bytes.length;
    return entry;
  }),
};
const bytes = Buffer.concat([...assets.values()]);
mkdirSync(output);
const inputPath = join(output, 'inspect.json'), bytesPath = join(output, 'assets.bin');
writeFileSync(inputPath, JSON.stringify(request), { flag: 'wx' });
writeFileSync(bytesPath, bytes, { flag: 'wx' });
const native = spawnSync(cli, ['delivery-inspect', inputPath, bytesPath], { encoding: 'utf8', timeout: 60000, maxBuffer: 8 << 20 });
assert.ifError(native.error); assert.equal(native.status, 0, native.stderr);
const wasm = createRequire(import.meta.url)(modulePath);
const response = JSON.parse(native.stdout);
assert.deepEqual(JSON.parse(wasm.inspect_delivery(JSON.stringify(request), bytes)), response);
assert.equal(response.status, 'inspected', native.stdout);
assert.deepEqual(response.report, inspection);
const { ShapingComponent } = await import(pathToFileURL(textPath));
const { RasterComponent } = await import(pathToFileURL(rasterPath));
const { default: hbFactory } = await import(pathToFileURL(join(hbRoot, 'mo-hb.mjs')));
const { default: skiaFactory } = await import(pathToFileURL(join(skiaRoot, 'mo-skia.mjs')));
const text = await ShapingComponent.create(hbFactory, new WebAssembly.Module(read(join(hbRoot, 'mo-hb.wasm'))));
const raster = await RasterComponent.create(skiaFactory, new WebAssembly.Module(read(join(skiaRoot, 'mo-skia.wasm'))));
const size = model.document.pageSize, width = context.settings.previewWidth;
const w = Number(size.width), h = Number(size.height);
let a = w, b = width;
while (b) [a, b] = [b, a % b];
const viewport = {
  width, height: Math.ceil(h * width / w), origin: { x: '0', y: '0' },
  scale: { numerator: width / a, denominator: w / a }, coordinateTolerance: String(1 << 20), background: [0, 0, 0, 0],
};
const pages = [];
try {
  for (const preview of bundle.previews) {
    const evidence = evidenceByPage.get(preview.pageId);
    assert.ok(evidence, 'missing actual page evidence');
    const input = JSON.stringify({
      profile: 'drawingml-resource-page-q32-v1-draft',
      page: {
        expectedSourceSha256: hash(source), slide: evidence.render.page.page.slide,
        profile: 'drawingml-static-solid-page-v1-draft', colorContext: context.settings.colorContext, viewport,
      },
      fonts: context.settings.fonts, imageSource: context.settings.imageSource, sampling: context.settings.sampling,
    });
    const header = Buffer.alloc(12), encoded = Buffer.from(input);
    [encoded.length, source.length, fonts.length].forEach((n, i) => header.writeUInt32LE(n, i * 4));
    const result = spawnSync(worker, ['--pptx-resource-page'], { input: Buffer.concat([header, encoded, source, fonts]), timeout: 60000, maxBuffer: 100 << 20 });
    assert.ifError(result.error); assert.equal(result.status, 0, result.stderr.toString());
    const n = result.stdout.readUInt32LE(), pixelLength = result.stdout.readUInt32LE(4);
    assert.equal(result.stdout.length, 8 + n + pixelLength);
    const metadata = JSON.parse(result.stdout.subarray(8, 8 + n).toString());
    const pixels = result.stdout.subarray(8 + n);
    assert.equal(metadata.status, 'rendered', JSON.stringify(metadata));
    const remote = wasm.render_pptx_resource_page(input, source, fonts, raster, text, raster);
    assert.deepEqual(JSON.parse(remote.metadata), metadata);
    assert.deepEqual(Buffer.from(remote.take_pixels()), pixels);
    assert.deepEqual(metadata.info.textCapacity, evidence.render.textCapacity);
    pages.push({ pageId: preview.pageId, pixelBytes: pixels.length, pixelSha256: hash(pixels), textCapacity: metadata.info.textCapacity });
  }
} finally { text.invalidate(); raster.invalidate(); }
const report = {
  profile: 'computed-delivery-native-wasm-parity/1', sourceSha256: hash(source),
  cli: identity(cli), worker: identity(worker), wasm: identity(modulePath.replace(/\.js$/, '_bg.wasm')),
  inspection, pages, officeWpsProven: false, modelAutonomousRepairProven: false,
};
writeFileSync(join(output, 'report.json'), JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
console.log(JSON.stringify({ pages: pages.length, exactInspectionMeasurementsPixels: true, officeWpsProven: false }));
