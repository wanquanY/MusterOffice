// Actual CLI and WASM computation, including native model equivalence and edits.
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { createRequire } from 'node:module';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';

assert.equal(process.argv.length, 5, 'usage: compose-parity.mjs <cli> <wasm-js> <new-output-directory>');
const [cli, wasmPath, root] = process.argv.slice(2).map(p => resolve(p));
const wasm = createRequire(import.meta.url)(wasmPath);
const fixture = JSON.parse(readFileSync(new URL('../../fixtures/presentations/compose/invocation.json', import.meta.url)));
mkdirSync(root);
const empty = join(root, 'inputs.json'), spool = join(root, 'temporary');
writeFileSync(empty, '[]'); mkdirSync(spool);
const cases = [];
function run(name, invocation, expectedFailure) {
  const raw = JSON.stringify(invocation), input = join(root, `${name}.json`), output = join(root, name);
  writeFileSync(input, raw);
  const native = spawnSync(cli, ['compute', input, empty, spool, output], {encoding:'utf8', maxBuffer:32*1024*1024});
  assert.ifError(native.error);
  let value, error;
  try { value = wasm.compute_document(raw); } catch (e) { error = String(e); }
  if (expectedFailure) {
    assert.notEqual(native.status, 0, name);
    assert.equal(native.stdout, '', name);
    const failure = JSON.parse(native.stderr).error;
    assert.equal(failure.code, expectedFailure, name);
    assert.deepEqual(JSON.parse(error), failure, name);
    cases.push({name, failure:failure.code}); return;
  }
  assert.equal(native.status, 0, native.stderr);
  assert.equal(native.stderr, ''); assert.equal(error, undefined);
  assert.equal(readFileSync(join(output, 'result.json'), 'utf8').trimEnd(), value, name);
  const receipt = JSON.parse(value);
  cases.push({name, semanticDigest:receipt.result.snapshot.semanticDigest});
  return receipt.result.snapshot;
}
const created = run('three-slides', fixture);
assert.equal(created.document.slideOrder.length, 3);
const direct = structuredClone(fixture);
direct.request.action = {kind:'create', document:created.document};
const recreated = run('equivalent-native-document', direct);
assert.equal(created.revision, recreated.revision);
assert.equal(created.semanticDigest, recreated.semanticDigest);
for (const [name, mutate, failure] of [
  ['empty-deck', p => {p.slides=[];}, 'INPUT_INVALID'],
  ['duplicate-slide', p => {p.slides.push(structuredClone(p.slides[0]));}, 'INPUT_INVALID'],
  ['duplicate-object', p => {p.slides[1].elements[0].id=p.slides[0].elements[0].id;}, 'INPUT_INVALID'],
  ['negative-size', p => {p.slides[0].elements[0].frame.width='-1';}, 'INPUT_INVALID'],
  ['unknown-author-field', p => {p.slides[0].elements[0].html='<b>invalid</b>';}, 'INPUT_INVALID'],
  ['tabs-and-paragraphs', p => {p.slides[0].elements[0].text.text='A\tB\r\n\rC\n';}, undefined],
  ['empty-editable-text', p => {p.slides[0].elements[0].text.text='';}, undefined],
  ['expanded-text-budget', p => {p.slides[0].elements[0].text.text='\t'.repeat(100000);}, 'LIMIT_EXCEEDED'],
]) {
  const input = structuredClone(fixture); mutate(input.request.action.presentation); run(name, input, failure);
}
const object=created.document.objects['object:title'], paragraph=object.content.text.paragraphs[0], text=paragraph.runs[0];
const edit={request:{...fixture.request, requestId:'request:edit', action:{kind:'apply', documentId:created.document.id,
  baseRevision:created.revision, operations:[{operationId:'operation:title',operation:{kind:'spliceText',
    object:object.id, paragraph:paragraph.id, run:text.id, start:0, delete:Array.from(text.content.text).length, insert:'Edited native text'}}]}},snapshot:created};
const edited=run('edit-composed-document',edit);
assert.equal(edited.document.objects[object.id].content.text.paragraphs[0].runs[0].content.text,'Edited native text');
run('stale-edit-rejected',{...edit,snapshot:edited},'REVISION_CONFLICT');
run('create-with-base-rejected',{...fixture,snapshot:created},'INPUT_INVALID');
const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex');
const report={format:'musteroffice.compose-parity/1',status:'passed',cliSha256:sha(cli),wasmSha256:sha(wasmPath.replace(/\.js$/,'_bg.wasm')),cases};
writeFileSync(join(root,'report.json'),JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({status:'passed',cases:cases.length,report:join(root,'report.json')}));
