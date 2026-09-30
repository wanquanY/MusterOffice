// Caller-owned source inputs; compare complete native/WASM label plans/errors.
import fs from 'node:fs/promises';
import path from 'node:path';
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';
const [cli, wasmModule, casesFile, output] = process.argv.slice(2);
assert.ok(output, 'usage: chart-label-parity.mjs <cli> <wasm.js> <cases.json> <new-report.json>');
const wasm = createRequire(import.meta.url)(path.resolve(wasmModule));
const cases = JSON.parse(await fs.readFile(casesFile, 'utf8'));
const sha = value => createHash('sha256').update(value).digest('hex');
const records = [];
for (const item of cases) {
  const request = await fs.readFile(item.request, 'utf8'), source = await fs.readFile(item.source);
  const native = JSON.parse(execFileSync(cli, ['pptx-chart-labels', item.request, item.source], { encoding: 'utf8', timeout: 30000, maxBuffer: 64 * 1024 * 1024 }));
  assert.deepEqual(JSON.parse(wasm.compute_pptx_chart_labels(request, source)), native, item.name);
  assert.equal(native.status, item.expectedStatus, item.name);
  records.push({ name: item.name, sourceSha256: sha(source), responseSha256: sha(JSON.stringify(native)), status: native.status,
    labels: native.labels?.labels.length ?? 0, normalizedSeries: native.labels?.normalizations.length ?? 0 });
}
await fs.writeFile(output, JSON.stringify({ profile: 'source-chart-label-native-wasm/1', cases: records,
  completeResponsesEqual: true, formattedLabelsProven: false, renderingProven: false, officeWpsProven: false }, null, 2) + '\n', { flag: 'wx' });
console.log(JSON.stringify({ cases: records.length, completeResponsesEqual: true }));
