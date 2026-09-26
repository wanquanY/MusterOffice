import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { resolve, dirname } from 'node:path';
import { spawnSync } from 'node:child_process';

const [cliPath, wasmPath, manifestPath] = process.argv.slice(2);
if (!manifestPath) throw new Error('usage: node opc-parity.mjs <cli> <wasm-node-js> <manifest.json>');
const cli = resolve(cliPath);
const modulePath = resolve(wasmPath);
const { inspect_package } = createRequire(import.meta.url)(modulePath);
const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
function sha256(bytes) { return createHash('sha256').update(bytes).digest('hex'); }
const cases = [];
for (const fixture of manifest.cases) {
  const path = resolve(dirname(manifestPath), fixture.file);
  const bytes = readFileSync(path);
  assert.equal(sha256(bytes), fixture.sha256, `${fixture.name}: fixture drift`);
  const native = spawnSync(cli, ['opc-inspect', path], { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  assert.equal(native.status, 0, native.stderr);
  const wasm = inspect_package(bytes);
  assert.equal(wasm, native.stdout.trimEnd(), `${fixture.name}: native/WASM byte mismatch`);
  const response = JSON.parse(wasm);
  assert.equal(response.status, fixture.status, `${fixture.name}: unexpected status: ${wasm}`);
  if (fixture.code) assert.equal(response.error.code, fixture.code, fixture.name);
  else {
    assert.equal(response.report.format, 'musteroffice.opc-inspection/1');
    for (const field of ['sha256', 'byteLength', 'parts']) {
      assert.deepEqual(response.report[field], fixture.expected[field], `${fixture.name}: independent ${field} mismatch`);
    }
  }
  cases.push({ name: fixture.name, sourceSha256: fixture.sha256, response });
}
console.log(JSON.stringify({
  format: 'musteroffice.opc-parity/1',
  scope: 'OPC ZIP/XML graph, hashes and binary admission only. No PPTX semantics, rendering, playback, Office/WPS, performance or replacement claim.',
  platform: process.platform, architecture: process.arch, node: process.version,
  independentWriter: manifest.independentWriter, independentVerification: manifest.independentVerification,
  nativeSha256: sha256(readFileSync(cli)), wasmSha256: sha256(readFileSync(modulePath.replace(/\.js$/, '_bg.wasm'))),
  passed: cases.length, cases,
}, null, 2));
