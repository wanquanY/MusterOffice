// Native/WASM computation and export parity for the owned table specimen.
// This does not test table rendering or external-application edit roundtrips.
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { createRequire } from 'node:module';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';

const [wasmPath, cliPath, requestPath, outputPath] = process.argv.slice(2).map(value => resolve(value));
assert(outputPath, 'usage: <wasm-node.js> <cli> <request.json> <new-output-directory>');
const wasm = createRequire(import.meta.url)(wasmPath);
const request = JSON.parse(readFileSync(requestPath, 'utf8'));
mkdirSync(outputPath, { recursive: false });
const empty = join(outputPath, 'empty.bin');
writeFileSync(empty, Buffer.alloc(0));
const hash = value => createHash('sha256').update(value).digest('hex');
const cases = [];
function cli(args, input) {
  const result = spawnSync(cliPath, args, { input, maxBuffer: 16 * 1024 * 1024 });
  assert.equal(result.status, 0, result.stderr?.toString());
  assert.equal(result.stderr.length, 0);
  return result.stdout;
}
function dispatch(name, input, status) {
  const raw = JSON.stringify(input);
  const native = JSON.parse(cli([], raw));
  assert.deepEqual(JSON.parse(wasm.dispatch_json(raw)), native, name);
  assert.equal(native.status, status, name);
  cases.push({ name, input, response: native });
  return native;
}
const exports = [];
function exported(name, document) {
  const value = JSON.stringify({ ...request, document });
  const input = join(outputPath, name + '.request.json');
  const output = join(outputPath, name + '.pptx');
  writeFileSync(input, value);
  cli(['pptx-export', input, empty, output]);
  const native = readFileSync(output);
  const cross = Buffer.from(wasm.export_pptx(value, new Uint8Array()));
  assert.deepEqual(cross, native, name);
  exports.push({ name, bytes: native.length, sha256: hash(native) });
}
let snapshot = dispatch('initialize', { operation: 'initialize', document: request.document }, 'initialized').snapshot;
exported('original', snapshot.document);
const tableOperation = operation => ({ kind: 'editTable', object: 'shape:1', operation });
const cell = id => ({ id, text: null, style: {} });
const operations = [
  { kind: 'spliceText', object: 'shape:1', paragraph: 'paragraph:1:1', run: 'run:1:1', start: 0, delete: 4, insert: '更改😀' },
  tableOperation({ kind: 'split', cell: 'cell:1:1' }),
  tableOperation({ kind: 'insertRow', index: 1, row: { id: 'insert-row', height: '457200', cells: [0, 1, 2].map(i => cell('insert-row:cell:' + i)) } }),
  tableOperation({ kind: 'insertColumn', index: 2, column: { id: 'insert-column', width: '914400' }, cells: [0, 1, 2, 3].map(i => cell('insert-column:cell:' + i)) }),
  tableOperation({ kind: 'merge', origin: 'cell:0:0', rows: 3, columns: 3 }),
  tableOperation({ kind: 'deleteRow', row: 'row:0' }),
  tableOperation({ kind: 'deleteColumn', column: 'column:0' }),
  tableOperation({ kind: 'reorderRows', order: ['row:2', 'insert-row', 'row:1'] }),
  tableOperation({ kind: 'reorderColumns', order: ['column:2', 'column:1', 'insert-column'] }),
  tableOperation({ kind: 'setColumnWidth', column: 'column:1', width: '2000000' }),
];
function prepare(operation, current, id) {
  return { operation: 'prepare', snapshot: current, transaction: {
    documentId: current.document.id, requestId: 'request:' + id, baseRevision: current.revision,
    operations: [{ operationId: 'operation:' + id, operation }],
  } };
}
for (let i = 0; i < operations.length; i++) {
  snapshot = dispatch('edit-' + i, prepare(operations[i], snapshot, i), 'prepared').snapshot;
  exported('edit-' + i, snapshot.document);
}
const before = JSON.stringify(snapshot);
for (const [i, operation] of [
  tableOperation({ kind: 'merge', origin: 'cell:2:1', rows: 2, columns: 2 }),
  tableOperation({ kind: 'setRowHeight', row: 'row:1', height: '0' }),
  tableOperation({ kind: 'reorderColumns', order: ['column:2', 'insert-column', 'column:1'] }),
].entries()) {
  dispatch('rejected-' + i, prepare(operation, snapshot, 'invalid-' + i), 'error');
  assert.equal(JSON.stringify(snapshot), before);
}
const report = {
  scope: 'table model, transactions and native export parity; no rendering or application acceptance',
  cliSha256: hash(readFileSync(cliPath)), wasmJsSha256: hash(readFileSync(wasmPath)),
  wasmSha256: hash(readFileSync(wasmPath.replace(/\.js$/, '_bg.wasm'))),
  inputSha256: hash(readFileSync(requestPath)), node: process.version, dispatchCases: cases.length, exports,
};
writeFileSync(join(outputPath, 'cases.json'), JSON.stringify(cases, null, 2) + '\n');
writeFileSync(join(outputPath, 'verification.json'), JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify({ dispatchCases: cases.length, exports: exports.length }));
