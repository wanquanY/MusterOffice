// Compare actual shared MCP, native CLI and WASM responses including failures.
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { createRequire } from 'node:module';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';

const [casesPath, cliPath, wasmPath, destination] = process.argv.slice(2).map(path => resolve(path));
const cases = JSON.parse(readFileSync(casesPath, 'utf8'));
const wasm = createRequire(import.meta.url)(wasmPath);
mkdirSync(destination, { recursive: false });
const empty = join(destination, 'inputs.json');
writeFileSync(empty, '[]');
const spool = join(destination, 'temporary');
mkdirSync(spool);
const results = [];
for (const entry of cases) {
  const raw = JSON.stringify(entry.invocation);
  const input = join(destination, entry.name + '.json');
  writeFileSync(input, raw);
  const output = join(destination, entry.name);
  const native = spawnSync(cliPath, ['compute', input, empty, spool, output], { encoding: 'utf8', maxBuffer: 8 * 1024 * 1024 });
  writeFileSync(join(destination, entry.name + '.stdout'), native.stdout);
  writeFileSync(join(destination, entry.name + '.stderr'), native.stderr);
  assert.ifError(native.error);
  let actual, failure;
  try { actual = JSON.parse(wasm.compute_document(raw)); }
  catch (error) { failure = JSON.parse(String(error)); }
  if (entry.failure) {
    assert.notEqual(native.status, 0, entry.name);
    assert.equal(native.stdout, '', entry.name);
    const failed = JSON.parse(native.stderr);
    assert.equal(failed.outcome, 'failed');
    assert.deepEqual(failed.error, entry.failure, entry.name + ': CLI failure');
    assert.deepEqual(failure, entry.failure, entry.name + ': WASM failure');
  } else {
    assert.equal(native.status, 0, native.stderr);
    assert.equal(native.stderr, '');
    assert.equal(failure, undefined, entry.name);
    assert.deepEqual(JSON.parse(readFileSync(join(output, 'result.json'), 'utf8')), entry.result, entry.name + ': CLI');
    assert.deepEqual(actual, entry.result, entry.name + ': WASM');
  }
  results.push({ name: entry.name, expectedFailure: !!entry.failure, exactResponse: true });
}
const sha = path => createHash('sha256').update(readFileSync(path)).digest('hex');
const report = { format: 'musteroffice.template-protocol-parity/1', status: 'passed',
  casesSha256: sha(casesPath), cliSha256: sha(cliPath), wasmWrapperSha256: sha(wasmPath),
  wasmSha256: sha(wasmPath.replace(/\.js$/, '_bg.wasm')), results };
writeFileSync(join(destination, 'report.json'), JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify(results));
