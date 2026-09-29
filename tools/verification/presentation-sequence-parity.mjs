/** Public atomic authoring and editable export through the actual WASM build. */
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';

const [output, wasmDirectory, corpus, cli] = process.argv.slice(2);
assert(output && wasmDirectory && corpus && cli, 'output wasm-directory corpus cli');
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const inputs = {};
async function read(file) {
  const bytes = await fs.readFile(file);
  inputs[file] = {sha256: sha(bytes), byteLength: bytes.length};
  return bytes;
}
const modulePath = path.join(wasmDirectory, 'mo_wasm.js');
await read(modulePath);
const wasm = await import(pathToFileURL(path.resolve(modulePath)));
await wasm.default({module_or_path: await read(path.join(wasmDirectory, 'mo_wasm_bg.wasm'))});
await read(cli);
await fs.mkdir(output, {recursive: false});
const cases = [];
for (const name of ['native-original', 'native-duration', 'parallel', 'author-edited']) {
  const request = await read(path.join(corpus, name + '.edit-request.json'));
  const expected = JSON.parse(await read(path.join(corpus, name + '.edit-response.json')));
  assert.deepEqual(JSON.parse(wasm.dispatch_json(request.toString())), expected, name);
  const exported = wasm.export_pptx((await read(path.join(corpus, name + '.export.json'))).toString(), new Uint8Array());
  assert.deepEqual(Buffer.from(exported), await read(path.join(corpus, name + '.pptx')), name);
  cases.push({name, edit: 'equal', pptxSha256: sha(exported)});
}
const baseline = JSON.parse(await read(path.join(corpus, 'parallel.edit-request.json')));
const failures = [];
for (const kind of ['negative-delay', 'duplicate-id', 'unbounded-predecessor', 'missing-object', 'unknown-field']) {
  const request = structuredClone(baseline);
  const operation = request.transaction.operations[0].operation;
  const batches = operation.sequence.groups[0].batches;
  if (kind === 'negative-delay') batches[0].delay.ticks = '-1';
  if (kind === 'duplicate-id') batches[0].effects[1].id = batches[0].effects[0].id;
  if (kind === 'unbounded-predecessor') batches[0].effects[0].repeatMilli = 'indefinite';
  if (kind === 'missing-object') batches[0].effects[0].effect.target = 'missing';
  if (kind === 'unknown-field') operation.sequence.hiddenOverride = true;
  const native = spawnSync(cli, [], {input: JSON.stringify(request), env: {}, encoding: 'utf8', timeout: 60000, maxBuffer: 40 * 1024 * 1024});
  assert.equal(native.status, 0, native.stderr);
  const result = JSON.parse(native.stdout);
  assert.equal(result.status, 'error');
  assert.equal(result.error.code, 'INPUT_INVALID');
  assert.deepEqual(JSON.parse(wasm.dispatch_json(JSON.stringify(request))), result, kind);
  await fs.writeFile(path.join(output, kind + '.json'), JSON.stringify({request, response: result}), {flag: 'wx'});
  failures.push(kind);
}
await fs.writeFile(path.join(output, 'report.json'), JSON.stringify({status: 'passed', inputs, cases, failures}, null, 2), {flag: 'wx'});
console.log(JSON.stringify({status: 'passed', edits: cases.length, exports: cases.length, rejected: failures.length}));
