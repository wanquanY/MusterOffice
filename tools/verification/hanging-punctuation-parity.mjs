import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {pathToFileURL} from 'node:url';
import {spawnSync} from 'node:child_process';
import factory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';

const root = path.resolve(process.argv[2] ?? '.codex-work/hanging-punctuation-20260930');
const wasm = await import(pathToFileURL(path.join(root, 'wasm-web/mo_wasm.js')));
wasm.initSync({module: fs.readFileSync(path.join(root, 'wasm-web/mo_wasm_bg.wasm'))});
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const bundle = fs.readFileSync('fixtures/fonts/owned-hanging.ttf');
const fonts = JSON.parse(fs.readFileSync('fixtures/fonts/manifest-hanging.json')).manifest.fonts;
assert.equal(sha(bundle), fonts[0].expectedSha256);
const component = await ShapingComponent.create(factory,
  new WebAssembly.Module(fs.readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm')));
const cases = [];
const input = [];
for (const text of ['AA!', '中中。', 'אב!', 'AAA!', '中。中。', '中中。\u2028中', 'AA!\u0301']) {
  for (const width of ['100', '274320', '284320', '411480']) {
    for (const hangingPunctuation of ['none', 'end']) {
      for (const overflow of ['keepUnbreakable', 'emergencyGrapheme']) {
        const request = {paragraph: {text, direction: text.startsWith('א') ? 'rightToLeft' : 'leftToRight',
          spans: [{end: [...text].length, style: 0}], fonts,
          styles: [{language: 'und', features: [], candidates: [{font: 0, variations: []}],
            suppressDottedCircle: false, maxGlyphs: 100}]},
          styles: [{fontSize: '228600', baselineShift: '0'}], strutStyle: 0,
          spacing: {kind: 'natural'}, width, overflow, hangingPunctuation};
        const json = JSON.stringify(request);
        const header = Buffer.alloc(8);
        header.writeUInt32LE(Buffer.byteLength(json)); header.writeUInt32LE(bundle.length, 4);
        input.push(header, Buffer.from(json), bundle);
        cases.push({request});
      }
    }
  }
}
const native = spawnSync('target/release/mo-text-worker', ['--layout'],
  {input: Buffer.concat(input), maxBuffer: 64 * 1024 * 1024, timeout: 120000});
assert.equal(native.status, 0, native.stderr?.toString());
let cursor = 0, hangingLines = 0, glyphs = 0;
for (const row of cases) {
  const count = native.stdout.readUInt32LE(cursor); cursor += 4;
  const result = native.stdout.subarray(cursor, cursor + count).toString(); cursor += count;
  assert.equal(wasm.layout_paragraph(JSON.stringify(row.request), bundle, component), result);
  const response = JSON.parse(result);
  assert.equal(response.status, 'evaluated');
  assert.deepEqual(response.result.issues, []);
  hangingLines += response.result.decisions.filter(d => d.hanging).length;
  glyphs += response.result.geometry.layout.lines.reduce((n, l) => n + l.glyphs.length, 0);
  row.response = response;
}
assert.equal(cursor, native.stdout.length);
assert.ok(hangingLines > 0);
const report = {format: 'musteroffice.hanging-punctuation-parity/1',
  workerSha256: sha(fs.readFileSync('target/release/mo-text-worker')),
  rustWasmSha256: sha(fs.readFileSync(path.join(root, 'wasm-web/mo_wasm_bg.wasm'))),
  componentSha256: sha(fs.readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm')),
  fontSha256: sha(bundle), exactResponses: cases.length, hangingLines, glyphs, cases};
fs.writeFileSync(path.join(root, 'parity.json'), JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify({exactResponses: cases.length, hangingLines, glyphs}));
