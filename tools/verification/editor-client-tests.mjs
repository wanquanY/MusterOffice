import assert from 'node:assert/strict';
import { readFileSync, mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { PresentationEditor, EditorComputationError } from '../../.codex-work/editor-client/build/editor-client/src/index.js';

// Explicit real native/WASM programs; missing build inputs fail, never skip.
const require = createRequire(import.meta.url);
const wasm = require(resolve(process.env.MUSTEROFFICE_EDITOR_WASM ?? '.codex-work/native-editor/wasm-node/mo_wasm.js'));
const native = resolve(process.env.MUSTEROFFICE_EDITOR_NATIVE ?? 'target/debug/mo-cli');
const editor = new PresentationEditor(wasm);
const example = JSON.parse(readFileSync(new URL('../../contracts/generated/document.schema.json', import.meta.url))).examples[0];

function nativeRequest(request) {
  const result = spawnSync(native, [], { input: JSON.stringify(request), encoding: 'utf8', maxBuffer: 32 * 1024 * 1024 });
  assert.equal(result.error, undefined);
  assert.equal(result.status, 0, result.stderr);
  return JSON.parse(result.stdout);
}
function transaction(snapshot, requestId, operations) {
  return { documentId: snapshot.document.id, baseRevision: snapshot.revision, requestId,
    operations: operations.map((operation, index) => ({operationId: `${requestId}:${index}`, operation})) };
}
function history(snapshot, originalSnapshot, originalTransaction, direction) {
  return {documentId: snapshot.document.id, baseRevision: snapshot.revision, requestId: `${direction}:1`,
    direction, originalSnapshot, originalTransaction};
}

test('native and WASM return identical edit, undo and redo candidates and receipts', () => {
  const original = editor.initialize(example);
  assert.deepEqual(original, nativeRequest({operation: 'initialize', document: example}).snapshot);
  const slideId = original.document.slideOrder[0];
  const change = transaction(original, 'edit:1', [{kind: 'setSlideName', slide: slideId, name: '中文 🚀'}]);
  const applied = editor.prepare(original, change);
  assert.deepEqual(applied, nativeRequest({operation: 'prepare', snapshot: original, transaction: change}));
  const later = editor.prepare(applied.snapshot, transaction(applied.snapshot, 'title:1', [{kind: 'setTitle', title: 'Later independent change'}]));
  const undoRequest = history(later.snapshot, original, change, 'undo');
  const undo = editor.prepareHistory(later.snapshot, undoRequest);
  assert.deepEqual(undo, nativeRequest({operation: 'prepareHistory', snapshot: later.snapshot, transaction: undoRequest}));
  assert.equal(undo.snapshot.document.title, 'Later independent change');
  assert.equal(undo.snapshot.document.slides[slideId].name, original.document.slides[slideId].name);
  assert.notEqual(undo.snapshot.revision, original.revision);
  const redoRequest = history(undo.snapshot, original, change, 'redo');
  const redo = editor.prepareHistory(undo.snapshot, redoRequest);
  assert.deepEqual(redo, nativeRequest({operation: 'prepareHistory', snapshot: undo.snapshot, transaction: redoRequest}));
  assert.deepEqual(redo.snapshot.document, later.snapshot.document);
});

test('a conflicting or tampered original is rejected by both engines without changing inputs', () => {
  const original = editor.initialize(example);
  const change = transaction(original, 'edit:1', [{kind: 'setTitle', title: 'one'}]);
  const applied = editor.prepare(original, change).snapshot;
  const current = editor.prepare(applied, transaction(applied, 'edit:2', [{kind: 'setTitle', title: 'two'}])).snapshot;
  const request = history(current, original, change, 'undo');
  const before = JSON.stringify(current);
  const expected = nativeRequest({operation: 'prepareHistory', snapshot: current, transaction: request});
  assert.equal(expected.status, 'error');
  assert.equal(expected.error.code, 'REFERENCE_CONFLICT');
  assert.throws(() => editor.prepareHistory(current, request), error => {
    assert.ok(error instanceof EditorComputationError);
    assert.deepEqual(error.diagnostic, expected.error);
    return true;
  });
  const tampered = structuredClone(request);
  tampered.originalSnapshot.document.title = 'forged';
  const nativeFailure = nativeRequest({operation: 'prepareHistory', snapshot: current, transaction: tampered});
  assert.equal(nativeFailure.status, 'error');
  assert.throws(() => editor.prepareHistory(current, tampered), error => {
    assert.deepEqual(error.diagnostic, nativeFailure.error); return true;
  });
  assert.equal(JSON.stringify(current), before);
});

test('segmentation and page placements come from the actual kernel', () => {
  const result = editor.segment('A😀e\u0301👩🏽‍💻');
  assert.deepEqual(result.boundaries.map(b => [b.scalarOffset, b.utf16Offset]), [[0,0],[1,1],[2,3],[4,5],[8,12]]);
  const snapshot = editor.initialize(example);
  const placement = editor.placements({document: snapshot.document, slide: snapshot.document.slideOrder[0]});
  assert.equal(placement.documentSha256, snapshot.semanticDigest);
  assert.ok(placement.surfaces.some(surface => surface.objects.length > 0));
});

test('cross-run text and local styling produce identical native/WASM candidates', () => {
  const original = editor.initialize(example);
  const [objectId, object] = Object.entries(original.document.objects).find(([, object]) => object.content.kind === 'shape' && object.content.text);
  const paragraphId = object.content.text.paragraphs[0].id;
  const caret = scalarOffset => ({paragraph: paragraphId, scalarOffset, affinity: 'after'});
  const command = {documentId: original.document.id, baseRevision: original.revision,
    requestId: 'text-range:1', operationId: 'text-range-operation:1', object: objectId,
    action: {kind: 'replace', selection: {anchor: caret(0), focus: caret(0)}, text: '中文😀\ne\u0301'}};
  const result = editor.prepareText(original, command);
  assert.deepEqual(result, nativeRequest({operation: 'prepareText', snapshot: original, command}).result);
  assert.equal(result.rangeChange.afterParagraphs.length, 2);
  assert.equal(result.selection.focus.scalarOffset, 2);
  const selected = {anchor: caret(0), focus: caret(2)};
  const style = {...command, baseRevision: result.snapshot.revision, requestId: 'text-style:1',
    action: {kind: 'setCharacterStyle', selection: selected, patch: {bold: {kind: 'value', value: true}}}};
  const styled = editor.prepareText(result.snapshot, style);
  assert.deepEqual(styled, nativeRequest({operation: 'prepareText', snapshot: result.snapshot, command: style}).result);
  assert.equal(styled.snapshot.document.objects[objectId].content.text.paragraphs[0].runs[0].style.bold.value, true);
});

test('slide duplication remaps its owned graph identically in native and WASM', () => {
  const original = editor.initialize(example);
  const source = original.document.slideOrder[0];
  const operation = {kind: 'duplicateSlide', source, slide: 'copied:slide', index: 1};
  const request = transaction(original, 'copy:1', [operation]);
  const result = editor.prepare(original, request);
  assert.deepEqual(result, nativeRequest({operation: 'prepare', snapshot: original, transaction: request}));
  assert.equal(result.snapshot.document.slideOrder[1], 'copied:slide');
  assert.ok(result.receipt.changes.createdObjects.length > 0);
  for (const id of result.receipt.changes.createdObjects) {
    assert.equal(original.document.objects[id], undefined);
  }
  const undo = editor.prepareHistory(result.snapshot, history(result.snapshot, original, request, 'undo'));
  assert.deepEqual(undo.snapshot.document, original.document);
});


test('text capability queries and structured rejections are identical in native/WASM/TS', () => {
  const snapshot = editor.initialize(example);
  const [object, shape] = Object.entries(snapshot.document.objects).find(([,o]) => o.content.kind === 'shape' && o.content.text);
  const caret = {paragraph: shape.content.text.paragraphs[0].id, scalarOffset: 0, affinity: 'after'};
  const query = {object, selection: {anchor: caret, focus: caret}};
  const caps = editor.textCapabilities(snapshot, query);
  assert.deepEqual(caps, nativeRequest({operation: 'textCapabilities', snapshot, query}).capabilities);
  assert.equal(caps.revision, snapshot.revision);
  assert.equal(caps.replace.kind, 'available');
  assert.equal(caps.characterStyle.reason.kind, 'nonemptySelectionRequired');
  const command = {documentId: snapshot.document.id, baseRevision: snapshot.revision, requestId: 'cap:style', operationId: 'cap:style',
    object, action: {kind: 'setCharacterStyle', selection: query.selection, patch: {}}};
  const failure = nativeRequest({operation: 'prepareText', snapshot, command});
  assert.throws(() => editor.prepareText(snapshot, command), error => {
    assert.ok(error instanceof EditorComputationError);
    assert.deepEqual(error.diagnostic, failure.error);
    assert.deepEqual(error.diagnostic.textRestriction, caps.characterStyle.reason);
    return true;
  });
  const tampered = structuredClone(snapshot); tampered.document.title = 'changed without a revision';
  assert.throws(() => editor.textCapabilities(tampered, query), EditorComputationError);
});


test('actual retained PPTX range export, reimport and capability diagnostics match native/WASM', () => {
  const sourceRequest = JSON.parse(readFileSync('fixtures/presentations/native-export/request.json'));
  const sourceResources = readFileSync('fixtures/presentations/native-export/resources.bin');
  const shape = Object.values(sourceRequest.document.objects).find(o => o.content.kind === 'shape' && o.content.text);
  const p = shape.content.text.paragraphs[0], style = p.runs[0].style;
  p.runs = ['A😀', 'e', '\u0301中'].map((text,i) => ({id: `native-parity:${i}`, style, content: {kind: 'text', text}}));
  const material = Buffer.from(wasm.export_pptx(JSON.stringify(sourceRequest), sourceResources));
  const importRequest = bytes => ({documentId: 'retained:parity', resourceId: 'source:parity', expectedSourceSha256: createHash('sha256').update(bytes).digest('hex')});
  const original = JSON.parse(wasm.import_pptx_document(JSON.stringify(importRequest(material)), material)).snapshot;
  assert.ok(original);
  assert.deepEqual(original, nativeRequest({operation: 'initialize', document: original.document}).snapshot);
  const [object, content] = Object.entries(original.document.objects).map(([id,o]) => [id,o.content]).find(([,c]) =>
    c.kind === 'retainedSource' && c.paragraphs.some(p => p.runs.map(r => r.text).join('') === 'A😀e\u0301中'));
  const paragraph = content.paragraphs.find(p => p.runs.map(r => r.text).join('') === 'A😀e\u0301中');
  const caret = scalarOffset => ({paragraph: paragraph.id, scalarOffset, affinity: 'after'});
  const selected = {anchor: caret(4), focus: caret(1)};
  const query = {object, selection: selected};
  const caps = editor.textCapabilities(original, query);
  assert.deepEqual(caps, nativeRequest({operation: 'textCapabilities', snapshot: original, query}).capabilities);
  assert.equal(caps.replacementPolicy, 'retainedTextLeaves');
  assert.equal(caps.replace.kind, 'available');
  const command = {documentId: original.document.id, baseRevision: original.revision, requestId: 'native:range', operationId: 'native:range',
    object, action: {kind: 'replace', selection: selected, text: 'α<&\t🚀'}};
  const result = editor.prepareText(original, command);
  assert.deepEqual(result, nativeRequest({operation: 'prepareText', snapshot: original, command}).result);
  assert.deepEqual(result.snapshot.document.sourceBindings, original.document.sourceBindings);
  const exportRequest = {document: result.snapshot.document, defaults: sourceRequest.defaults,
    resourceBindings: [{resourceId: original.document.sourceBindings.resource, byteOffset: '0', byteLength: String(material.length)}]};
  const exported = Buffer.from(wasm.export_pptx(JSON.stringify(exportRequest), material));
  const dir = mkdtempSync(resolve(tmpdir(), 'mo-retained-parity-'));
  try {
    const requestPath = resolve(dir, 'request.json'), sourcePath = resolve(dir, 'source.pptx'), outputPath = resolve(dir, 'edited.pptx');
    writeFileSync(requestPath, JSON.stringify(exportRequest)); writeFileSync(sourcePath, material);
    const out = spawnSync(native, ['pptx-export', requestPath, sourcePath, outputPath], {encoding: 'utf8', timeout: 60000});
    assert.equal(out.error, undefined); assert.equal(out.status, 0, out.stderr);
    assert.deepEqual(readFileSync(outputPath), exported);
    writeFileSync(requestPath, JSON.stringify(importRequest(exported)));
    const reimport = spawnSync(native, ['pptx-import', requestPath, outputPath], {encoding: 'utf8', timeout: 60000});
    assert.equal(reimport.error, undefined); assert.equal(reimport.status, 0, reimport.stderr);
    const restored = JSON.parse(wasm.import_pptx_document(JSON.stringify(importRequest(exported)), exported));
    assert.deepEqual(JSON.parse(reimport.stdout), restored);
    assert.equal(restored.status, 'imported');
    assert.ok(Object.values(restored.snapshot.document.objects).some(o => o.content.kind === 'retainedSource' &&
      o.content.paragraphs.some(p => p.runs.map(r => r.text).join('') === 'Aα<&\t🚀中')));
  } finally { rmSync(dir, {recursive: true, force: true}); }
  const guarded = structuredClone(original.document);
  guarded.sourceBindings.objects[object].runs[paragraph.runs[1].id].constraint = 'dynamicField';
  const protectedSnapshot = editor.initialize(guarded);
  const protectedCaps = editor.textCapabilities(protectedSnapshot, query);
  assert.deepEqual(protectedCaps, nativeRequest({operation: 'textCapabilities', snapshot: protectedSnapshot, query}).capabilities);
  assert.deepEqual(protectedCaps.replace.reason, {kind: 'nativeRun', run: paragraph.runs[1].id, constraint: 'dynamicField'});
  const rejected = {...command, baseRevision: protectedSnapshot.revision};
  const failure = nativeRequest({operation: 'prepareText', snapshot: protectedSnapshot, command: rejected});
  assert.throws(() => editor.prepareText(protectedSnapshot, rejected), error => {
    assert.deepEqual(error.diagnostic, failure.error);
    assert.deepEqual(error.diagnostic.textRestriction, protectedCaps.replace.reason); return true;
  });
});
