/** Interleaved, warm actual WASM calls; byte-identical responses are mandatory. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import {resolve} from 'node:path';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {performance} from 'node:perf_hooks';

const [output, previousPath, currentPath] = process.argv.slice(2);
assert(output && previousPath && currentPath);
assert(!fs.existsSync(output), 'preserve earlier measurements');
const require = createRequire(import.meta.url);
const modules = [require(resolve(previousPath)), require(resolve(currentPath))];
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const entry = path => {
  const bytes = fs.readFileSync(path);
  return {path, byteLength: bytes.length, sha256: sha(bytes)};
};
const summary = values => {
  const sorted = values.toSorted((a, b) => a - b);
  return {median: sorted[20], p95: sorted[38], min: sorted[0], max: sorted.at(-1)};
};
const inputs = [
  ['authored-source', '.codex-work/pptx-source-fixtures/authored.pptx'],
  ['opaque-source', '.codex-work/pptx-source-fixtures/unknown-content.pptx'],
  ['utf16-source', '.codex-work/pptx-source-fixtures/utf16.pptx'],
].map(([name, path]) => {
  const bytes = fs.readFileSync(path);
  return {name, inputs: [entry(path)], call: module => module.inspect_pptx(bytes), status: 'inspected'};
});
const requestPath = 'fixtures/presentations/delivery-receive/request.json';
const bytesPath = 'fixtures/presentations/delivery-receive/assets.bin';
const request = fs.readFileSync(requestPath, 'utf8'), bytes = fs.readFileSync(bytesPath);
inputs.push({name: 'delivery-reception', inputs: [entry(requestPath), entry(bytesPath)],
  call: module => module.inspect_delivery(request, bytes), status: 'inspected'});
const cases = [];
for (const input of inputs) {
  const expected = input.call(modules[0]);
  assert.equal(input.call(modules[1]), expected);
  assert.equal(JSON.parse(expected).status, input.status);
  const warmups = 5, rounds = 41, batch = 4, samples = [[], []];
  for (let i = 0; i < warmups; i++) for (const module of modules) assert.equal(input.call(module), expected);
  const measure = index => {
    const responses = new Array(batch);
    const start = performance.now();
    for (let i = 0; i < batch; i++) responses[i] = input.call(modules[index]);
    samples[index].push((performance.now() - start) / batch);
    // Assertions are outside the timed interval. Every response is still checked.
    for (const response of responses) assert.equal(response, expected);
  };
  for (let i = 0; i < rounds; i++) for (const index of i % 2 ? [1, 0] : [0, 1]) measure(index);
  const before = summary(samples[0]), after = summary(samples[1]);
  cases.push({name: input.name, inputs: input.inputs, responseSha256: sha(expected), warmups,
    rounds, batch, beforeMs: samples[0], afterMs: samples[1], before, after,
    medianRatio: after.median / before.median});
}
const artifacts = [previousPath, currentPath].flatMap(path => [entry(path), entry(resolve(path, '../mo_wasm_bg.wasm'))]);
const report = {format: 'musteroffice.package-read-cost/1',
  environment: {platform: os.platform(), release: os.release(), arch: os.arch(),
    cpu: os.cpus()[0].model, logicalCpus: os.cpus().length, memoryBytes: os.totalmem(), node: process.version,
    isolation: 'Ordinary desktop session; other applications remain running. Task builds and regression runs finished before this measurement; OS scheduling and background activity are uncontrolled.'},
  scope: 'Warm same-process WASM, alternating before/after batches. Input bytes loaded before timing; every call reopens and validates the package. Includes JS/WASM transfer, computation and response serialization; assertions excluded. Source inspection uses no fonts or external renderer. Delivery reception validates its bundled synthetic font bytes and actual PNG pixels, without font shaping or drawing. No browser, native storage/IPC, cold start, RSS, FPS, general workload or installation-size claim.',
  dependencies: ['Rust WASM and wasm-bindgen JS listed here', 'Node built-ins only; no raster/shaper component loaded'],
  artifacts, cases};
fs.writeFileSync(output, JSON.stringify(report, null, 2) + '\n', {flag: 'wx'});
console.log(JSON.stringify(cases.map(c => ({name: c.name, before: c.before, after: c.after, ratio: c.medianRatio}))));
