import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const [cliPath, wasmModulePath] = process.argv.slice(2);
if (!cliPath || !wasmModulePath) throw new Error('usage: node native-wasm-parity.mjs <cli> <wasm-node-js>');
const cli = resolve(cliPath);
const modulePath = resolve(wasmModulePath);
const wasm = createRequire(import.meta.url)(modulePath);
const { dispatch_json } = wasm;
const document = JSON.parse(readFileSync(new URL('../../fixtures/presentations/basic-shape.json', import.meta.url), 'utf8'));
const cases = [];
const schemaPairs = [];

// The production WASM exposes only computation discovery. The old host/job
// discovery binding is built separately as mo-host-compat-wasm.
assert.equal(wasm.operation_schema_json, undefined);
for (const id of ['computation-request', 'computation-failure', 'computation-mutation-receipt', 'computation-export-receipt', 'computation-invocation', 'computation-receipt']) {
  const discovered = JSON.parse(wasm.computation_schema_json(JSON.stringify(id)));
  const generated = JSON.parse(readFileSync(new URL(`../../contracts/generated/${id}.schema.json`, import.meta.url), 'utf8'));
  assert.equal(discovered.id, id);
  assert.deepEqual(discovered.schema, generated, `${id}: native/WASM schema mismatch`);
  const native = spawnSync(cli, ['compute-schema', id], { encoding: 'utf8', maxBuffer: 4 * 1024 * 1024 });
  assert.equal(native.status, 0, native.stderr);
  assert.deepEqual(JSON.parse(native.stdout), discovered, `${id}: runtime schema mismatch`);
  schemaPairs.push({ id, digest: discovered.digest });
}

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
    documentId: snapshot.document.id, requestId: `request:${cases.length}`, baseRevision: snapshot.revision,
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

// Admit real native bytes into the same domain model, then run the exact
// transaction endpoint above. This additionally covers import binding and
// revision computation; it does not assert rendering or Office interoperability.
const temporary = mkdtempSync(join(tmpdir(), 'mo-import-parity-'));
try {
  const exportRequest = readFileSync(new URL('../../fixtures/presentations/native-export/request.json', import.meta.url), 'utf8');
  const resources = readFileSync(new URL('../../fixtures/presentations/native-export/resources.bin', import.meta.url));
  const bytes = wasm.export_pptx(exportRequest, resources);
  const request = { documentId: 'imported-parity', resourceId: 'original', expectedSourceSha256: createHash('sha256').update(bytes).digest('hex') };
  const input = JSON.stringify(request);
  writeFileSync(join(temporary, 'input.json'), input);
  writeFileSync(join(temporary, 'source.pptx'), bytes);
  const native = spawnSync(cli, ['pptx-import', join(temporary, 'input.json'), join(temporary, 'source.pptx')], { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  assert.equal(native.status, 0, native.stderr);
  const imported = wasm.import_pptx_document(input, bytes);
  assert.equal(imported, native.stdout.trimEnd(), 'import native/WASM bytes differ');
  const response = JSON.parse(imported);
  assert.equal(response.status, 'imported');
  cases.push({ name: 'real-source-import', input, response });
  const snapshot = response.snapshot;
  assert.equal(snapshot.document.title, JSON.parse(exportRequest).document.title);
  assert.equal(snapshot.document.sourceBindings.profile, 'presentationml-retained-fields-v2-draft');
  const object = Object.values(snapshot.document.objects).find(o => o.parent.kind === 'slide' && o.content.paragraphs.length && o.content.paragraphs[0].runs.length && o.transform);
  const p = object.content.paragraphs[0];
  const splice = { kind: 'spliceText', object: object.id, paragraph: p.id, run: p.runs[0].id, start: 0, delete: 0, insert: '中文🚀' };
  const transform = structuredClone(object.transform); transform.origin.x = '101';
  const inputEdit = transaction([splice, { kind: 'setTransform', object: object.id, transform }], snapshot);
  const edited = compare('retained-mixed-edit', inputEdit, 'prepared');
  assert.deepEqual(compare('retained-deterministic-replay', inputEdit, 'prepared'), edited);
  const stale = structuredClone(inputEdit); stale.snapshot = edited.snapshot;
  compare('retained-stale-revision', stale, 'error', 'REVISION_CONFLICT');
  compare('retained-protected-structural-edit', transaction([splice, { kind: 'deleteObject', object: object.id, policy: 'rejectDependencies' }], snapshot), 'error', 'INPUT_INVALID');
  const title = compare('retained-title-edit', transaction([{ kind: 'setTitle', title: '原生 <&> title' }], snapshot), 'prepared');
  assert.equal(title.snapshot.document.title, '原生 <&> title');
  const legacyDocument = structuredClone(snapshot.document);
  legacyDocument.sourceBindings.profile = 'presentationml-retained-fields-v1-draft';
  legacyDocument.title = '';
  const legacy = compare('initialize-retained-v1', { operation: 'initialize', document: legacyDocument }, 'initialized').snapshot;
  compare('retained-v1-title-stays-unprojected', transaction([{ kind: 'setTitle', title: 'blocked' }], legacy), 'error', 'INPUT_INVALID');
} finally { rmSync(temporary, { recursive: true, force: true }); }

function sha256(path) { return createHash('sha256').update(readFileSync(path)).digest('hex'); }
console.log(JSON.stringify({
  format: 'musteroffice.foundation-parity/1',
  scope: 'Author/retained Document computation and PPTX import binding. No rendering, Office/WPS, performance or product integration claims.',
  platform: process.platform, architecture: process.arch, node: process.version,
  nativeSha256: sha256(cli), wasmSha256: sha256(modulePath.replace(/\.js$/, '_bg.wasm')),
  passed: cases.length, cases, schemaPairs,
}, null, 2));
