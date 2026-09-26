import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const [cliPath, wasmModulePath] = process.argv.slice(2);
if (!cliPath || !wasmModulePath) throw new Error('usage: node native-wasm-parity.mjs <cli> <wasm-node-js>');
const cli = resolve(cliPath);
const modulePath = resolve(wasmModulePath);
const { dispatch_json } = createRequire(import.meta.url)(modulePath);
const document = JSON.parse(readFileSync(new URL('../../fixtures/presentations/basic-shape.json', import.meta.url), 'utf8'));
const cases = [];

function compare(name, input, expectedStatus, expectedCode) {
  const raw = typeof input === 'string' ? input : JSON.stringify(input);
  const native = spawnSync(cli, { input: raw, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  assert.equal(native.status, 0, native.stderr);
  const nativeResponse = native.stdout.trimEnd();
  const wasmResponse = dispatch_json(raw);
  assert.equal(wasmResponse, nativeResponse, `${name}: native/WASM byte mismatch`);
  const response = JSON.parse(nativeResponse);
  assert.equal(response.status, expectedStatus, `${name}: unexpected status`);
  if (expectedCode) assert.equal(response.error.code, expectedCode, name);
  cases.push({ name, input: raw, response });
  return response;
}

assert.equal(compare('validate', { operation: 'validate', document }, 'validated').report.issues.length, 0);
const initial = compare('initialize', { operation: 'initialize', document }, 'initialized').snapshot;
function transaction(operations, snapshot = initial) {
  return { operation: 'prepare', snapshot, transaction: {
    documentId: document.id, requestId: `request:${cases.length}`, baseRevision: snapshot.revision,
    operations: operations.map((operation, index) => ({ operationId: `operation:${index}`, operation })),
  }};
}

const splice = transaction([{ kind: 'spliceText', object: 'shape:1', paragraph: 'paragraph:1', run: 'run:1', start: 1, delete: 1, insert: '火箭🚀' }]);
const edited = compare('unicode-scalar-splice', splice, 'prepared');
assert.equal(edited.snapshot.document.objects['shape:1'].content.text.paragraphs[0].runs[0].content.text, 'A火箭🚀é中');
assert.deepEqual(compare('deterministic-replay-preparation', splice, 'prepared'), edited);
const stale = structuredClone(splice);
stale.snapshot = edited.snapshot;
compare('stale-revision', stale, 'error', 'REVISION_CONFLICT');
const tampered = structuredClone(splice);
tampered.snapshot.document.title = 'tampered without new digest';
compare('tampered-snapshot', tampered, 'error', 'INPUT_INVALID');
compare('atomic-operation-failure', transaction([{ kind: 'setTitle', title: 'unpublished' }, { kind: 'deleteObject', object: 'missing', policy: 'rejectDependencies' }]), 'error', 'INPUT_INVALID');
compare('atomic-global-validation-failure', transaction([{ kind: 'setTitle', title: 'unpublished' }, { kind: 'setLayout', slide: 'slide:1', layout: 'missing' }]), 'error', 'INPUT_INVALID');
compare('out-of-range-splice', transaction([{ kind: 'spliceText', object: 'shape:1', paragraph: 'paragraph:1', run: 'run:1', start: 5, delete: 1, insert: '' }]), 'error', 'INPUT_INVALID');
compare('deletion', transaction([{ kind: 'deleteObject', object: 'shape:1', policy: 'rejectDependencies' }]), 'prepared');
compare('duplicate-json-keys', '{"operation":"validate","operation":"initialize"}', 'error', 'INPUT_INVALID');
compare('unknown-operation', { operation: 'renderUnsupportedNow', document }, 'error', 'INPUT_INVALID');
for (const coordinate of ['-0', '01', '+1', '9223372036854775808', 9007199254740992]) {
  const invalid = structuredClone(document);
  invalid.pageSize.width = coordinate;
  compare(`invalid-coordinate-${coordinate}`, { operation: 'initialize', document: invalid }, 'error', 'INPUT_INVALID');
}
for (const text of ['é', 'e\u0301', 'مرحبا بالعالم', '中文「段落」', '👩🏽‍💻🚀']) {
  const body = structuredClone(document.objects['shape:1'].content.text);
  body.paragraphs[0].runs[0].content.text = text;
  const response = compare(`unicode-${text}`, transaction([{ kind: 'setText', object: 'shape:1', text: body }]), 'prepared');
  assert.equal(response.snapshot.document.objects['shape:1'].content.text.paragraphs[0].runs[0].content.text, text);
}
assert.notEqual(cases.find(c => c.name === 'unicode-é').response.snapshot.semanticDigest, cases.find(c => c.name === 'unicode-e\u0301').response.snapshot.semanticDigest);

function sha256(path) { return createHash('sha256').update(readFileSync(path)).digest('hex'); }
console.log(JSON.stringify({
  format: 'musteroffice.foundation-parity/1',
  scope: 'Document computation only. No rendering, PPTX, Office/WPS, performance or product integration claims.',
  platform: process.platform, architecture: process.arch, node: process.version,
  nativeSha256: sha256(cli), wasmSha256: sha256(modulePath.replace(/\.js$/, '_bg.wasm')),
  passed: cases.length, cases,
}, null, 2));
