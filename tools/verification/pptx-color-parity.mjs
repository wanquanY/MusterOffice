import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync, mkdtempSync } from 'node:fs';
import { createRequire } from 'node:module';
import { resolve, relative, join } from 'node:path';
import { spawnSync } from 'node:child_process';

const [cliArg, wasmArg, manifestArg, inheritanceArg] = process.argv.slice(2);
if (!manifestArg) throw new Error('usage: pptx-color-parity.mjs <cli> <wasm-js> <manifest>');
const cli = resolve(cliArg), wasmPath = resolve(wasmArg);
const wasm = createRequire(import.meta.url)(wasmPath);
const manifest = JSON.parse(readFileSync(manifestArg));
const directory = mkdtempSync(resolve('.codex-work/pptx-color-query-'));
const hash = b => createHash('sha256').update(b).digest('hex');
const cases = [];

function query(name, sample, request, expected, validRequest = true) {
  const source = readFileSync(sample.path);
  assert.equal(hash(source), sample.sha256);
  const path = join(directory, name + '.json');
  writeFileSync(path, request);
  const native = spawnSync(cli, ['pptx-colors', path, resolve(sample.path)], { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  assert.equal(native.status, 0, native.stderr);
  const raw = wasm.resolve_pptx_colors(request, source);
  assert.equal(raw, native.stdout.trimEnd(), `${name}: Native/WASM color response differs`);
  const response = JSON.parse(raw);
  if (typeof expected === 'string' && /^[A-Z_]+$/.test(expected)) {
    assert.equal(response.status, 'error', name);
    assert.equal(response.error.code, expected, name);
  } else {
    assert.equal(response.status, 'evaluated', raw);
    const first = response.palette.colors[0].outcome;
    if (Array.isArray(expected)) assert.deepEqual(first.rgba8, expected, name);
    else if (expected) assert.equal(first.reason.kind, expected, name);
  }
  assert.equal(hash(readFileSync(sample.path)), sample.sha256, 'query modified source');
  cases.push({ name, source: relative(process.cwd(), resolve(sample.path)), sourceSha256: sample.sha256, request, validRequest, response });
}

for (const sample of manifest.cases) query(sample.name, sample, JSON.stringify(sample.request), sample.expectedFirst);
if (inheritanceArg) {
  const hierarchy = JSON.parse(readFileSync(inheritanceArg));
  for (const sample of hierarchy.cases.filter(c => !c.expect.error)) {
    const request = { expectedSourceSha256: sample.sha256, surface: '/ppt/slides/slide1.xml', profile: 'ecma376-2016-draft-v1',
      colors: ['bg1','tx1','bg2','tx2','accent1','accent2','accent3','accent4','accent5','accent6','hlink','folHlink','phClr','dk1','lt1','dk2','lt2'],
      context: { systemColors: {}, placeholder: null } };
    query('hierarchy-' + sample.name, sample, JSON.stringify(request));
  }
}
const base = manifest.cases[0];
for (const [name, changes, code] of [
  ['digest-conflict', { expectedSourceSha256: '0'.repeat(64) }, 'SOURCE_CONFLICT'],
  ['unknown-surface', { surface: '/ppt/slides/missing.xml' }, 'INPUT_INVALID'],
  ['query-budget', { colors: Array(257).fill('accent1') }, 'LIMIT_EXCEEDED'],
]) query(name, base, JSON.stringify({ ...base.request, ...changes }), code);
for (const [name, input] of [
  ['unknown-profile', JSON.stringify({ ...base.request, profile: 'implicit-host' })],
  ['unknown-field', JSON.stringify({ ...base.request, systemPath: '/private' })],
  ['duplicate-field', JSON.stringify(base.request).replace('"colors":', '"colors":[],"colors":')],
  ['bad-system-color', JSON.stringify({ ...base.request, context: { systemColors: { invented: [0,0,0] }, placeholder: null } })],
]) query(name, base, input, 'INPUT_INVALID', false);
console.log(JSON.stringify({ format: 'musteroffice.color-parity/1', nativeSha256: hash(readFileSync(cli)),
  wasmSha256: hash(readFileSync(join(resolve(wasmPath, '..'), 'mo_wasm_bg.wasm'))), cases }, null, 2));
