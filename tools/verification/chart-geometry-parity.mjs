// Explicit, owned geometry inputs. No source-chart style/Office parity claim.
import fs from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';

const [configFile, output] = process.argv.slice(2);
assert.ok(output, 'usage: node chart-geometry-parity.mjs <config.json> <new-report.json>');
const config = JSON.parse(await fs.readFile(configFile, 'utf8'));
const wasm = createRequire(import.meta.url)(path.resolve(config.wasmModule));
const sha = value => createHash('sha256').update(value).digest('hex');
const unit = 1n << 32n;
const point = (x, y) => ({ x: String(BigInt(x) * unit), y: String(BigInt(y) * unit) });
let component;
if (config.cases.some(c => c.render)) {
  const { default: factory } = await import(pathToFileURL(path.resolve(config.componentModule)));
  const { RasterComponent } = await import(pathToFileURL(path.resolve(config.adapterModule)));
  component = await RasterComponent.create(factory, new WebAssembly.Module(await fs.readFile(config.componentWasm)));
}
async function raster(item, request, geometry, stroke) {
  // This test viewport has explicit scale and origin, never chart layout defaults.
  assert.deepEqual(request.center, point(0, 0));
  assert.equal(request.outerRadius, String(1000n * unit));
  const q = {
    viewport: { width: 512, height: 512, origin: point(-1280, -1280), scale: { numerator: 1, denominator: 5 }, coordinateTolerance: String(1 << 24), background: [0, 0, 0, 0] },
    paths: geometry.paths.map(p => ({ fillRule: geometry.fillRule, commands: p.commands })),
    draws: geometry.paths.flatMap((p, i) => p.commands.length ? [{ path: i, origin: point(0, 0), brush: { kind: 'solid', rgba: [[41,91,160,255],[19,166,179,255],[240,163,49,255],[177,80,151,255]][i % 4] }, ...(stroke ? { stroke: { width: String(5n * unit), cap: 'butt', join: { kind: 'round' } } } : {}) }] : []),
  };
  const raw = JSON.stringify(q), header = Buffer.alloc(4);
  header.writeUInt32LE(Buffer.byteLength(raw));
  const reply = execFileSync(config.worker, [], { input: Buffer.concat([header, Buffer.from(raw)]), timeout: 30000, maxBuffer: 80 * 1024 * 1024, env: {} });
  const metaLength = reply.readUInt32LE(), pixels = reply.subarray(8 + metaLength);
  assert.equal(pixels.length, reply.readUInt32LE(4));
  const metadata = JSON.parse(reply.subarray(8, 8 + metaLength));
  assert.equal(metadata.status, 'rendered');
  const result = wasm.render_paths(raw, component);
  assert.deepEqual(JSON.parse(result.metadata), metadata);
  assert.deepEqual(Buffer.from(result.take_pixels()), pixels);
  assert.ok(pixels.some((v, i) => i % 4 === 3 && v > 0), 'nonempty geometry must paint pixels');
  // The low-level raster contract only reports lowering error: explicitly add
  // upstream geometry error here rather than presenting it as already included.
  const upstream = geometry.paths.reduce((a, p) => a > BigInt(p.coordinateErrorBound) ? a : BigInt(p.coordinateErrorBound), 0n);
  const total = (upstream + 4n) / 5n + BigInt(metadata.info.work.coordinateErrorBound);
  assert.ok(total <= BigInt(q.viewport.coordinateTolerance));
  let verifiedPixels = 0;
  const inner = Number(BigInt(request.innerRadius)) / Number(unit) / 5;
  const single = geometry.paths.filter(p => p.commands.length).length === 1;
  for (let y = 0; y < 512; y++) for (let x = 0; x < 512; x++) {
    const r = Math.hypot(x + .5 - 256, y + .5 - 256), alpha = pixels[(y * 512 + x) * 4 + 3];
    if (r < inner - 3 || r > 203) { assert.equal(alpha, 0); verifiedPixels++; }
    if (single && r > inner + 3 && r < 197) { assert.equal(alpha, stroke ? 0 : 255); verifiedPixels++; }
  }
  const filename = path.join(path.dirname(output), `${item.name}.${stroke ? 'stroke' : 'fill'}.rgba`);
  await fs.writeFile(filename, pixels, { flag: 'wx' });
  return { mode: stroke ? 'stroke' : 'fill', width: 512, height: 512, pixelsSha256: sha(pixels), bytes: pixels.length, requestSha256: sha(raw), metadata, totalCoordinateErrorPixelsQ32: String(total), verifiedPixels, exactNativeWasmPixels: true };
}
const records = [];
for (const item of config.cases) {
  const raw = await fs.readFile(item.request, 'utf8'), request = JSON.parse(raw);
  const native = JSON.parse(execFileSync(config.cli, ['compile-chart-geometry', item.request], { encoding: 'utf8', timeout: 30000, maxBuffer: 32 * 1024 * 1024 }));
  assert.deepEqual(JSON.parse(wasm.compile_chart_geometry(raw)), native, item.name);
  assert.equal(native.status, item.expectedStatus, item.name);
  if (item.expectedCode) assert.equal(native.error.code, item.expectedCode, item.name);
  const rendered = [];
  if (item.render) {
    assert.equal(native.status, 'computed');
    rendered.push(await raster(item, request, native.geometry, false));
    if (item.stroke) rendered.push(await raster(item, request, native.geometry, true));
  }
  records.push({ name: item.name, requestSha256: sha(raw), response: native, rendered });
}
await fs.writeFile(output, JSON.stringify({ profile: 'chart-geometry-native-wasm-parity/1', cases: records, completeResponsesEqual: true, sourceChartRenderingProven: false, officeWpsProven: false }, null, 2) + '\n', { flag: 'wx' });
console.log(JSON.stringify({ cases: records.length, rendered: records.flatMap(r => r.rendered).length, completeResponsesEqual: true }));
