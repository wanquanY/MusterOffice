// Exercise the public transports with the edited table corpus. The core owns
// computation; this development harness owns files and verification outputs.
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { createRequire } from 'node:module';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
const [wasmPath, cliPath, corpusPath, outputPath] = process.argv.slice(2).map(value => resolve(value));
assert(outputPath, 'usage: <wasm-node.js> <cli> <native-table-parity-directory> <new-output-directory>');
const wasm = createRequire(import.meta.url)(wasmPath);
mkdirSync(outputPath, { recursive: false });
const hash = value => createHash('sha256').update(value).digest('hex');
function call(args, input) {
  const result = spawnSync(cliPath, args, { input, maxBuffer: 32 * 1024 * 1024 });
  assert.equal(result.status, 0, result.stderr?.toString());
  assert.equal(result.stderr.length, 0);
  return JSON.parse(result.stdout);
}
function save(name, value) {
  const path = join(outputPath, name + '.json');
  writeFileSync(path, JSON.stringify(value, null, 2) + '\n');
  return path;
}
const cases = [];
for (const entry of JSON.parse(readFileSync(join(corpusPath, 'verification.json'))).exports) {
  const name = entry.name;
  const path = join(corpusPath, name + '.pptx');
  const bytes = readFileSync(path);
  assert.equal(hash(bytes), entry.sha256);
  const inspected = call(['pptx-inspect', path]);
  assert.deepEqual(JSON.parse(wasm.inspect_pptx(bytes)), inspected, name);
  assert.equal(inspected.status, 'inspected');
  save(name + '.before', inspected);
  const importRequest = { documentId: 'table:' + name, resourceId: 'source:' + name, expectedSourceSha256: entry.sha256 };
  const imported = call(['pptx-import', save(name + '.import-request', importRequest), path]);
  assert.deepEqual(JSON.parse(wasm.import_pptx_document(JSON.stringify(importRequest), bytes)), imported);
  assert.equal(imported.status, 'imported');
  assert.equal(imported.snapshot.document.sourceBindings.profile, 'presentationml-retained-fields-v3-draft');
  save(name + '.imported', imported);
  const edits = [];
  for (const [part, surface] of Object.entries(inspected.index.surfaces)) {
    for (const object of surface.objects.filter(object => object.table)) {
      assert.equal(object.textBodyOrdinal, undefined);
      for (const [row, value] of object.table.rows.entries()) {
        for (const [column, cell] of value.cells.entries()) {
          assert(surface.text.roots.some(root => root.owner === object.nativeId && root.sourceOrdinal === cell.textBodyOrdinal && root.cell.row === row && root.cell.column === column));
          for (let paragraph = cell.paragraphStart; paragraph < cell.paragraphStart + cell.paragraphCount; paragraph++) {
            for (const [run, value] of object.paragraphs[paragraph].entries()) {
              assert(value.editable);
              edits.push({ target: { part, objectId: object.nativeId, paragraph, run }, expectedText: value.text, replacement: `更改 ${row}:${column} <&> 🚀 é ${name}` });
            }
          }
        }
      }
    }
  }
  assert(edits.length > 0);
  const request = { expectedSourceSha256: entry.sha256, edits };
  const requestPath = save(name + '.edit-request', request);
  const output = join(outputPath, name + '.pptx');
  call(['pptx-edit-text', requestPath, path, output]);
  const actual = readFileSync(output);
  assert.deepEqual(Buffer.from(wasm.edit_pptx_text(JSON.stringify(request), bytes)), actual);
  const after = call(['pptx-inspect', output]);
  assert.deepEqual(JSON.parse(wasm.inspect_pptx(actual)), after);
  assert.equal(after.status, 'inspected');
  save(name + '.after', after);
  for (const edit of edits) {
    const object = after.index.surfaces[edit.target.part].objects.find(object => object.nativeId === edit.target.objectId);
    assert.equal(object.paragraphs[edit.target.paragraph][edit.target.run].text, edit.replacement);
  }
  const rejectedPath = join(outputPath, name + '.stale.pptx');
  const rejection = spawnSync(cliPath, ['pptx-edit-text', requestPath, output, rejectedPath]);
  assert.equal(rejection.status, 1);
  assert.match(rejection.stderr.toString(), /SOURCE_CONFLICT/);
  assert(!existsSync(rejectedPath));
  assert.throws(() => wasm.edit_pptx_text(JSON.stringify(request), actual), /SOURCE_CONFLICT/);
  cases.push({ name, sourceSha256: entry.sha256, outputSha256: hash(actual), bytes: actual.length, editedLeaves: edits.length, comparisons: 5 });
}
const report = { scope: 'native/WASM table inspection, import, preserved native text edits and stale rejection; no rendering or external application acceptance', node: process.version,
  cliSha256: hash(readFileSync(cliPath)), wasmJsSha256: hash(readFileSync(wasmPath)), wasmSha256: hash(readFileSync(wasmPath.replace(/\.js$/, '_bg.wasm'))),
  cases, presentations: cases.length, comparisons: cases.reduce((n, c) => n + c.comparisons, 0), editedLeaves: cases.reduce((n, c) => n + c.editedLeaves, 0) };
save('verification', report);
console.log(JSON.stringify({ presentations: report.presentations, comparisons: report.comparisons, editedLeaves: report.editedLeaves }));
