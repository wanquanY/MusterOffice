import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync, existsSync, mkdirSync, mkdtempSync, readdirSync } from 'node:fs';
import { createRequire } from 'node:module';
import { resolve, relative, join } from 'node:path';
import { spawnSync } from 'node:child_process';

const [cliPath, wasmPath] = process.argv.slice(2);
if (!wasmPath) throw new Error('usage: node pptx-parity.mjs <cli> <wasm-node-js>');
const cli = resolve(cliPath);
const modulePath = resolve(wasmPath);
const { export_pptx } = createRequire(import.meta.url)(modulePath);
const fixture = JSON.parse(readFileSync(new URL('../../fixtures/presentations/native-export/request.json', import.meta.url), 'utf8'));
const binary = readFileSync(new URL('../../fixtures/presentations/native-export/resources.bin', import.meta.url));
mkdirSync('.codex-work', { recursive: true });
const directory = mkdtempSync(resolve('.codex-work/pptx-parity-'));
const cases = [];
let initialBytes;
const hash = bytes => createHash('sha256').update(bytes).digest('hex');

function compare(name, mutate = () => {}, expectedCode) {
  const request = structuredClone(fixture);
  mutate(request);
  const requestBytes = JSON.stringify(request);
  const jsonPath = join(directory, `${name}.json`);
  const resourcePath = join(directory, `${name}.bin`);
  const outputPath = join(directory, `${name}.pptx`);
  writeFileSync(jsonPath, requestBytes);
  writeFileSync(resourcePath, binary);
  const native = spawnSync(cli, ['pptx-export', jsonPath, resourcePath, outputPath], { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  if (expectedCode) {
    assert.notEqual(native.status, 0, `${name}: native unexpectedly succeeded`);
    assert.ok(native.stderr.includes(expectedCode), `${name}: ${native.stderr}`);
    assert.throws(() => export_pptx(requestBytes, binary), error => String(error).startsWith(`${expectedCode}:`), name);
    assert.equal(existsSync(outputPath), false, `${name}: failure exposed output`);
    cases.push({ name, status: 'rejected', code: expectedCode });
  } else {
    assert.equal(native.status, 0, native.stderr);
    const wasmBytes = Buffer.from(export_pptx(requestBytes, binary));
    const nativeBytes = readFileSync(outputPath);
    assert.deepEqual(wasmBytes, nativeBytes, `${name}: PPTX bytes differ`);
    if (name === 'native-objects') initialBytes = nativeBytes;
    if (name === 'deterministic-replay') assert.deepEqual(nativeBytes, initialBytes);
    const inspection = JSON.parse(native.stdout);
    assert.equal(inspection.status, 'inspected');
    assert.equal(inspection.report.sha256, hash(nativeBytes));
    cases.push({ name, status: 'exported', sha256: hash(nativeBytes), byteLength: nativeBytes.length, parts: inspection.report.parts.length });
  }
  assert.equal(readdirSync(directory).some(name => name.endsWith('.tmp')), false, 'staging files leaked');
}

compare('native-objects');
compare('deterministic-replay');
compare('rgba8-alpha', request => { request.document.objects['round:1'].appearance.fill.value.color.rgba.alpha = 128; });
compare('hidden-slide', request => { request.document.slides['slide:2'].hidden = true; });
compare('rtl-paragraph', request => { request.document.objects['unicode:1'].content.text.paragraphs[0].style.direction = { kind: 'value', value: 'rightToLeft' }; });
compare('vertical-ltr-body', request => { request.document.objects['title:1'].content.text.paragraphs[0].style.direction = { kind: 'value', value: 'verticalLeftToRight' }; });
compare('vertical-rtl-body', request => { request.document.objects['title:1'].content.text.paragraphs[0].style.direction = { kind: 'value', value: 'verticalRightToLeft' }; });
compare('missing-image', request => { request.resourceBindings = []; }, 'RESOURCE_REQUIRED');
compare('resource-digest-mismatch', request => { request.document.resources['resource:checker'].sha256 = '0'.repeat(64); }, 'PRESERVATION_CONFLICT');
compare('range-outside-bundle', request => { request.resourceBindings[0].byteLength = '100000'; }, 'INPUT_INVALID');
compare('duplicate-resource-binding', request => { request.resourceBindings.push(request.resourceBindings[0]); }, 'INPUT_INVALID');
compare('noncanonical-resource-length', request => { request.resourceBindings[0].byteLength = '01'; }, 'INPUT_INVALID');
compare('missing-theme-color', request => { delete request.defaults.themeColors.accent6; }, 'INPUT_INVALID');
compare('nonrepresentable-font-size', request => { request.defaults.textSize = '12701'; }, 'INPUT_INVALID');
compare('unsupported-layout-style', request => { request.document.layouts['layout:brand'].defaultText.bold = { kind: 'value', value: true }; }, 'MAPPING_NOT_IMPLEMENTED');
compare('invalid-xml-character', request => { request.document.objects['title:1'].content.text.paragraphs[0].runs[0].content.text = 'bad\u0000text'; }, 'INPUT_INVALID');
compare('unsupported-decorative', request => { request.document.objects['title:1'].accessibility.decorative = true; }, 'MAPPING_NOT_IMPLEMENTED');

const originalPath = join(directory, 'native-objects.pptx');
const overwrite = spawnSync(cli, ['pptx-export', join(directory, 'native-objects.json'), join(directory, 'native-objects.bin'), originalPath], { encoding: 'utf8' });
assert.notEqual(overwrite.status, 0, 'existing output was overwritten');
assert.deepEqual(readFileSync(originalPath), initialBytes, 'existing output changed');
assert.equal(readdirSync(directory).some(name => name.endsWith('.tmp')), false);
console.log(JSON.stringify({
  format: 'musteroffice.pptx-export-parity/1',
  scope: 'Current authored PresentationML subset and binary bridge. No full import, layout/rendering/playback, font embedding, Office/WPS acceptance or replacement claim.',
  platform: process.platform, architecture: process.arch, node: process.version,
  nativeSha256: hash(readFileSync(cli)), wasmSha256: hash(readFileSync(modulePath.replace(/\.js$/, '_bg.wasm'))),
  artifactDirectory: relative(process.cwd(), directory), passed: cases.length, cases,
  hostChecks: { actualStagedFileReopened: true, existingOutputPreserved: true, noPartialOutputOnRejection: true, stagingCleanup: true },
}, null, 2));
