// Public source fill/color transport parity. This does not render table pages.
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, mkdirSync, writeFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { createRequire } from 'node:module';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
const [corpus, cli, wasmPath, output] = process.argv.slice(2).map(p => resolve(p));
assert(output, 'usage: <corpus> <mo-cli> <wasm-node.js> <new-output-directory>');
mkdirSync(output, { recursive: false });
const wasm = createRequire(import.meta.url)(wasmPath);
const hash = b => createHash('sha256').update(b).digest('hex');
const save = (name, value) => {
  const path = join(output, name + '.json');
  writeFileSync(path, JSON.stringify(value, null, 2) + '\n');
  return path;
};
const cases = [];
for (const file of readdirSync(corpus).filter(n => n.endsWith('.pptx')).sort()) {
  const name = file.slice(0, -5), sourcePath = join(corpus, file), source = readFileSync(sourcePath);
  const q = JSON.parse(readFileSync(join(corpus, name + '.request.json')));
  assert.equal(q.expectedSourceSha256, hash(source));
  const expected = JSON.parse(readFileSync(join(corpus, name + '.response.json')));
  const variants = [q];
  // Query the real declarations explicitly as well as their cell instances.
  const owners = new Map();
  const walk = v => {
    if (!v || typeof v !== 'object') return;
    if (v.target?.kind === 'tableStyleFill') owners.set(JSON.stringify(v.target), v.target);
    for (const child of Object.values(v)) walk(child);
  };
  walk(expected);
  if (owners.size) variants.push({ ...q, targets: [...owners.values()] });
  let comparisons = 0, targetCount = 0;
  for (const [i, request] of variants.entries()) {
    const requestPath = save(name + '-' + i + '.request', request);
    const native = spawnSync(cli, ['pptx-fill-colors', requestPath, sourcePath], { maxBuffer: 32 * 1024 * 1024 });
    assert.equal(native.status, 0, native.stderr.toString());
    assert.equal(native.stderr.length, 0);
    const result = JSON.parse(native.stdout);
    assert.equal(result.status, 'evaluated');
    assert.deepEqual(JSON.parse(wasm.resolve_pptx_fill_colors(JSON.stringify(request), source)), result, name);
    assert.equal(result.colors.targets.length, request.targets.length);
    if (i === 0) assert.deepEqual(result.colors, expected, 'public result differs from Rust library');
    save(name + '-' + i + '.response', result);
    targetCount += request.targets.length;
    comparisons++;
  }
  const stale = { ...q, expectedSourceSha256: '0'.repeat(64) };
  const stalePath = save(name + '.stale-request', stale);
  const native = spawnSync(cli, ['pptx-fill-colors', stalePath, sourcePath], { maxBuffer: 32 * 1024 * 1024 });
  const rejected = JSON.parse(native.stdout);
  assert.equal(rejected.status, 'error');
  assert.equal(rejected.error.code, 'SOURCE_CONFLICT');
  assert.deepEqual(JSON.parse(wasm.resolve_pptx_fill_colors(JSON.stringify(stale), source)), rejected);
  save(name + '.stale-response', rejected);
  cases.push({ name, sourceSha256: hash(source), comparisons: comparisons + 1, targetCount });
}
assert(cases.length >= 6);
const report = { scope: 'native/WASM source table fill and color queries; no pixel rendering or application acceptance',
  node: process.version, cliSha256: hash(readFileSync(cli)), wasmJsSha256: hash(readFileSync(wasmPath)),
  wasmSha256: hash(readFileSync(wasmPath.replace(/\.js$/, '_bg.wasm'))), cases,
  comparisons: cases.reduce((n, c) => n + c.comparisons, 0), targets: cases.reduce((n, c) => n + c.targetCount, 0) };
save('verification', report);
console.log(JSON.stringify({ presentations: cases.length, comparisons: report.comparisons, targets: report.targets }));
