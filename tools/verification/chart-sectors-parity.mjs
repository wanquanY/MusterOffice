// Compare complete native/WASM results, preserving caller-selected evidence.
import fs from 'node:fs/promises';
import path from 'node:path';
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';

const [cli, wasmModule, casesFile, output] = process.argv.slice(2);
assert.ok(output, 'usage: node chart-sectors-parity.mjs <cli> <wasm-node.js> <cases.json> <new-report.json>');
const wasm = createRequire(import.meta.url)(path.resolve(wasmModule));
const records = [];
for (const item of JSON.parse(await fs.readFile(casesFile, 'utf8'))) {
  const request = await fs.readFile(item.request, 'utf8');
  const native = JSON.parse(execFileSync(cli, ['layout-chart-sectors', item.request], {
    encoding: 'utf8', timeout: 30000, maxBuffer: 16 * 1024 * 1024,
  }));
  const browser = JSON.parse(wasm.layout_chart_sectors(request));
  assert.deepEqual(browser, native, item.name);
  assert.equal(native.status, item.expectedStatus, item.name);
  records.push({ name: item.name, requestSha256: createHash('sha256').update(request).digest('hex'), response: native });
}
await fs.writeFile(output, JSON.stringify({
  profile: 'chart-sector-native-wasm-parity/1', cases: records,
  completeResponsesEqual: true, chartRenderingProven: false, officeWpsProven: false,
}, null, 2) + '\n', { flag: 'wx' });
console.log(JSON.stringify({ cases: records.length, completeResponsesEqual: true }));
