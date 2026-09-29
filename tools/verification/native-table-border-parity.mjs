// Exercise public transports and the shared fill API, not just Rust internals.
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, mkdirSync, writeFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { createRequire } from 'node:module';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
const args = process.argv.slice(2);
assert.equal(args.length, 4, 'usage: <corpus> <mo-cli> <wasm-node.js> <new-output-directory>');
const [corpus, cli, wasmPath, output] = args.map(p => resolve(p));
mkdirSync(output, { recursive: false });
const wasm = createRequire(import.meta.url)(wasmPath);
const hash = b => createHash('sha256').update(b).digest('hex');
const save = (name, value) => {
  const path = join(output, name + '.json');
  writeFileSync(path, JSON.stringify(value, null, 2) + '\n', { flag: 'wx' });
  return path;
};
const cases = [];
for (const file of readdirSync(corpus).filter(n => n.endsWith('.pptx')).sort()) {
  const name = file.slice(0, -5), sourcePath = join(corpus, file), source = readFileSync(sourcePath);
  const q = JSON.parse(readFileSync(join(corpus, name + '.request.json')));
  assert.equal(q.expectedSourceSha256, hash(source));
  const expected = JSON.parse(readFileSync(join(corpus, name + '.response.json')));
  const queries = [];
  const compare = (suffix, api, request) => {
    const stem = name + '-' + suffix;
    const requestPath = save(stem + '.request', request);
    const command = api === 'border' ? 'pptx-table-borders' : 'pptx-fill-colors';
    const entry = api === 'border' ? 'resolve_pptx_table_borders' : 'resolve_pptx_fill_colors';
    const native = spawnSync(cli, [command, requestPath, sourcePath], { maxBuffer: 32 * 1024 * 1024 });
    assert.equal(native.status, 0, native.stderr.toString());
    assert.equal(native.stderr.length, 0);
    const result = JSON.parse(native.stdout);
    assert.deepEqual(JSON.parse(wasm[entry](JSON.stringify(request), source)), result, stem);
    const responsePath = save(stem + '.response', result);
    queries.push({ stem, api, requestSha256: hash(readFileSync(requestPath)), responseSha256: hash(readFileSync(responsePath)), status: result.status, targets: request.targets.length });
    return result;
  };
  const primary = compare('borders', 'border', q);
  assert.equal(primary.status, 'evaluated');
  assert.deepEqual(primary.borders, expected, 'public result differs from Rust library');
  const fq = { expectedSourceSha256: q.expectedSourceSha256, surface: q.surface,
    fillProfile: q.fillProfile, colorProfile: q.colorProfile, context: q.context,
    targets: q.targets.map(t => ({ kind: 'tableCellBorder', ...t })) };
  const fills = compare('fills', 'fill', fq);
  assert.equal(fills.status, 'evaluated');
  assert.deepEqual(fills.colors.targets, primary.borders.targets.map(t => t.fill));
  const owners = new Map();
  const walk = value => {
    if (!value || typeof value !== 'object') return;
    if (value.target?.kind === 'tableStyleBorder') owners.set(JSON.stringify(value.target), value.target);
    for (const child of Object.values(value)) walk(child);
  };
  walk(expected);
  if (owners.size) {
    const result = compare('styles', 'fill', { ...fq, targets: [...owners.values()] });
    assert.equal(result.status, 'evaluated');
    assert.equal(result.colors.targets.length, owners.size);
  }
  const stale = compare('stale', 'border', { ...q, expectedSourceSha256: '0'.repeat(64) });
  assert.equal(stale.status, 'error');
  assert.equal(stale.error.code, 'SOURCE_CONFLICT');
  cases.push({ name, sourceSha256: hash(source), queries });
}
assert(cases.length >= 8);
const report = { scope: 'native/WASM source table borders, topology and full line fill; no page pixels or Office/WPS acceptance',
  node: process.version, cliSha256: hash(readFileSync(cli)), wasmJsSha256: hash(readFileSync(wasmPath)),
  wasmSha256: hash(readFileSync(wasmPath.replace(/\.js$/, '_bg.wasm'))), cases,
  comparisons: cases.reduce((n, c) => n + c.queries.length, 0),
  borderTargets: cases.reduce((n, c) => n + c.queries.filter(q => q.api === 'border' && q.status === 'evaluated').reduce((m, q) => m + q.targets, 0), 0) };
save('verification', report);
console.log(JSON.stringify({ presentations: cases.length, comparisons: report.comparisons, borderTargets: report.borderTargets }));
