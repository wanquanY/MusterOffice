import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync, existsSync, mkdirSync, mkdtempSync, readdirSync } from 'node:fs';
import { createRequire } from 'node:module';
import { resolve, relative, join } from 'node:path';
import { spawnSync } from 'node:child_process';

const [cliArg, wasmArg, manifestArg, externalArg] = process.argv.slice(2);
if (!manifestArg) throw new Error('usage: pptx-source-parity.mjs <cli> <wasm-js> <manifest> [external-owned-fixture.pptx]');
const cli = resolve(cliArg), wasmPath = resolve(wasmArg);
const wasm = createRequire(import.meta.url)(wasmPath);
const manifest = JSON.parse(readFileSync(manifestArg));
mkdirSync('.codex-work', { recursive: true });
const directory = mkdtempSync(resolve('.codex-work/pptx-source-parity-'));
const hash = b => createHash('sha256').update(b).digest('hex');
const cases = [], outputs = [], requests = [];
let authoredIndex, authoredPath;

function inspect(name, path, expected) {
  const bytes = readFileSync(path);
  const native = spawnSync(cli, ['pptx-inspect', path], { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  assert.equal(native.status, 0, native.stderr);
  const actual = wasm.inspect_pptx(bytes);
  assert.equal(native.stdout.trimEnd(), actual, `${name} source index differs`);
  const response = JSON.parse(actual);
  if (expected.error) {
    assert.equal(response.status, 'error');
    assert.equal(response.error.code, expected.error);
  } else {
    assert.equal(response.status, 'inspected', actual);
    assert.equal(response.index.sourceSha256, hash(bytes));
    assert.equal(response.index.slides.length, 2);
    assert.equal(Object.values(response.index.surfaces).reduce((n, s) => n + s.objects.length, 0), expected.objects);
    if (expected.signature) assert.equal(response.index.containsSignatures, true);
  }
  cases.push({ name: `inspect-${name}`, source: relative(process.cwd(), path), sourceSha256: hash(bytes), response });
  return response.index;
}

function select(index, replacement, objectName = 'title:1') {
  const [part, surface] = Object.entries(index.surfaces).find(([, s]) => s.objects.some(o => o.name === objectName));
  const object = surface.objects.find(o => o.name === objectName);
  return {
    expectedSourceSha256: index.sourceSha256,
    edits: [{ target: { part, objectId: object.nativeId, paragraph: 0, run: 0 }, expectedText: object.paragraphs[0][0].text, replacement }],
  };
}

function edit(name, path, request, code, noop = false) {
  const input = JSON.stringify(request), source = readFileSync(path);
  const requestPath = join(directory, `${name}.json`), output = join(directory, `${name}.pptx`);
  writeFileSync(requestPath, input);
  requests.push({ name, request, expectedCode: code ?? null });
  const native = spawnSync(cli, ['pptx-edit-text', requestPath, path, output], { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  if (code) {
    assert.notEqual(native.status, 0, `${name} unexpectedly succeeded`);
    assert.ok(native.stderr.includes(`${code}:`), native.stderr);
    assert.throws(() => wasm.edit_pptx_text(input, source), e => String(e).startsWith(`${code}:`));
    assert.equal(existsSync(output), false);
    cases.push({ name, status: 'rejected', code });
  } else {
    assert.equal(native.status, 0, native.stderr);
    const bytes = readFileSync(output);
    assert.deepEqual(bytes, Buffer.from(wasm.edit_pptx_text(input, source)), `${name} candidate bytes differ`);
    assert.equal(JSON.parse(native.stdout).report.sha256, hash(bytes));
    if (noop) assert.deepEqual(bytes, source);
    outputs.push({ name, source: relative(process.cwd(), path), output: relative(process.cwd(), output), request: relative(process.cwd(), requestPath), sha256: hash(bytes), noop });
    cases.push({ name, status: 'edited', sha256: hash(bytes), byteLength: bytes.length, noop });
  }
  assert.equal(readdirSync(directory).some(n => n.endsWith('.tmp')), false);
}

for (const sample of manifest.cases) {
  const path = resolve(sample.path);
  assert.equal(hash(readFileSync(path)), sample.sha256);
  const index = inspect(sample.name, path, sample.expect);
  if (!index) continue;
  const request = select(index, '原生编辑 😀 é & < >\r\n第二行');
  if ('colorMapping' in sample.expect) {
    const surface = index.surfaces[request.edits[0].target.part];
    const binding = surface.resolvedColorMapping;
    if (sample.expect.colorMapping === null) {
      assert.equal(binding, null);
    } else {
      assert.equal(binding.part, sample.expect.colorMapping.part);
      const declaration = index.surfaces[binding.part].colorMapping;
      assert.equal(declaration.kind, 'explicit');
      assert.equal(declaration.sourceOrdinal, binding.sourceOrdinal);
      assert.equal(declaration.mapping.accent1, sample.expect.colorMapping.accent1);
    }
  }
  if (sample.expect.themeSelection) {
    const selection = index.surfaces[request.edits[0].target.part].themeSelection;
    for (const [family, part] of Object.entries(sample.expect.themeSelection)) {
      assert.equal(selection[family].part, part);
    }
  }
  if (sample.expect.inheritance) {
    const surface = index.surfaces[request.edits[0].target.part];
    const object = surface.objects.find(o => o.nativeId === request.edits[0].target.objectId);
    assert.equal(object.resolution.placeholderMatch.status, sample.expect.inheritance);
    if (sample.expect.inheritance === 'matched') {
      if (sample.expect.ambiguousMaster) {
        assert.equal(object.resolution.origin, null);
      } else {
        assert.equal(object.resolution.origin.value.x, sample.expect.zero ? '0' : sample.expect.originX ?? '111');
        assert.equal(object.resolution.origin.declaredBy.part, sample.expect.zero ? '/ppt/slides/slide1.xml' : '/ppt/slideMasters/slideMaster2.xml');
      }
      assert.equal(object.resolution.size.value.width, sample.expect.zero ? '0' : sample.expect.width ?? '555');
      assert.equal(object.resolution.size.declaredBy.part, sample.expect.zero ? '/ppt/slides/slide1.xml' : '/ppt/slideLayouts/slideLayout2.xml');
    } else {
      assert.equal(object.resolution.origin, null);
      assert.equal(object.resolution.size, null);
    }
    assert.equal(surface.effectiveTheme.overrides.length, sample.expect.themeOverrides ?? 0);
  }
  if (sample.expect.editable !== undefined) {
    const surface = index.surfaces[request.edits[0].target.part];
    const run = surface.objects.find(o => o.nativeId === request.edits[0].target.objectId).paragraphs[0][0];
    assert.equal(run.editable, sample.expect.editable);
    if (sample.expect.constraint !== undefined) assert.equal(run.editConstraint, sample.expect.constraint);
  }
  edit(`edit-${sample.name}`, path, request, sample.expect.edit === 'success' ? undefined : sample.expect.edit);
  edit(`noop-${sample.name}`, path, { expectedSourceSha256: index.sourceSha256, edits: [] }, undefined, true);
  if (sample.expect.outsideEdit) {
    edit(`outside-${sample.name}`, path, select(index, '分支外原生编辑 😀', 'unicode:1'));
    request.edits[0].replacement = request.edits[0].expectedText;
    edit(`same-value-${sample.name}`, path, request, undefined, true);
  }
  if (sample.name === 'authored') { authoredIndex = index; authoredPath = path; }
}

for (const [name, mutate, code] of [
  ['stale-source', r => { r.expectedSourceSha256 = '0'.repeat(64); }, 'SOURCE_CONFLICT'],
  ['stale-text', r => { r.edits[0].expectedText = 'stale'; }, 'SOURCE_CONFLICT'],
  ['missing-object', r => { r.edits[0].target.objectId = 999999; }, 'SOURCE_CONFLICT'],
  ['duplicate-target', r => { r.edits.push(structuredClone(r.edits[0])); }, 'SOURCE_CONFLICT'],
  ['forged-part', r => { r.edits[0].target.part = '/docProps/core.xml'; }, 'SOURCE_CONFLICT'],
  ['invalid-xml-character', r => { r.edits[0].replacement = '\u0000'; }, 'SOURCE_CONFLICT'],
]) {
  const request = select(authoredIndex, 'new'); mutate(request);
  edit(name, authoredPath, request, code);
}
const sameValue = select(authoredIndex, '');
sameValue.edits[0].replacement = sameValue.edits[0].expectedText;
edit('same-value', authoredPath, sameValue, undefined, true);

if (externalArg) {
  const path = resolve(externalArg);
  // Independent lxml inventory confirms LibreOffice added two native placeholders
  // and moved the layout rule to a master. This is observed drift, not acceptance.
  const index = inspect('external-owned-roundtrip', path, { objects: 17 });
  const request = select(index, '外部文件原生修改 😀');
  const surface = index.surfaces[request.edits[0].target.part];
  assert.equal(surface.textEditBarriers.length, 0);
  assert.ok(surface.compatibility.selections.length > 0);
  assert.ok(surface.compatibility.selections.every(s => s.branches.some(b => b.fallback && b.selected)));
  edit('external-owned-roundtrip', path, request);
  edit('noop-external-owned-roundtrip', path, { expectedSourceSha256: index.sourceSha256, edits: [] }, undefined, true);
}

const original = outputs[0];
const overwrite = spawnSync(cli, ['pptx-edit-text', original.request, original.source, original.output], { encoding: 'utf8' });
assert.notEqual(overwrite.status, 0);
assert.equal(hash(readFileSync(original.output)), original.sha256);
console.log(JSON.stringify({
  format: 'musteroffice.pptx-source-parity/1',
  scope: 'Partial source projection and ordinary text leaf preservation; not resolved import, layout/playback or Office/WPS acceptance.',
  nativeSha256: hash(readFileSync(cli)), wasmSha256: hash(readFileSync(wasmPath.replace(/\.js$/, '_bg.wasm'))),
  artifactDirectory: relative(process.cwd(), directory), passed: cases.length, cases, requests, outputs,
  hostChecks: { noPartialOutputOnRejection: true, existingOutputPreserved: true, stagingCleanup: true },
}, null, 2));
