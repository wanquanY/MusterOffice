/** Real native/WASM no-wrap flow; explicit line ends are derived from source. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {pathToFileURL} from 'node:url';
import {spawnSync} from 'node:child_process';
import factory from '../../.codex-work/harfbuzz/release/mo-hb.mjs';
import {ShapingComponent} from '../../.codex-work/text-component/index.js';
const root = path.resolve(process.argv[2] ?? '.codex-work/nowrap-20260930');
const wasm = await import(pathToFileURL(path.join(root, 'wasm-web/mo_wasm.js')));
wasm.initSync({module: fs.readFileSync(path.join(root, 'wasm-web/mo_wasm_bg.wasm'))});
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const fonts = JSON.parse(fs.readFileSync('fixtures/fonts/manifest-hanging.json')).manifest.fonts;
const bundle = fs.readFileSync('fixtures/fonts/owned-hanging.ttf');
assert.equal(sha(bundle), fonts[0].expectedSha256);
const component = await ShapingComponent.create(factory,
  new WebAssembly.Module(fs.readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm')));
const cases = [], input = [];
for (const text of ['', 'A A A', '中中中', 'אב אב', 'AA\u2028A A\u2028', '\u2028\u2028', 'AA!\u0301', ' A A ']) {
  for (const width of ['1', '137160', '284320', '2000000']) {
    for (const overflow of ['keepUnbreakable', 'emergencyGrapheme']) {
      for (const wrapping of ['wrap', 'noWrap']) {
        const request = {paragraph: {text, direction: text.startsWith('א') ? 'rightToLeft' : 'leftToRight',
          spans: text ? [{end: [...text].length, style: 0}] : [], fonts,
          styles: [{language: 'und', features: [], candidates: [{font: 0, variations: []}],
            suppressDottedCircle: false, maxGlyphs: 100}]},
          styles: [{fontSize: '228600', baselineShift: '0'}], strutStyle: 0,
          spacing: {kind: 'natural'}, width, overflow, wrapping};
        const json = Buffer.from(JSON.stringify(request)), header = Buffer.alloc(8);
        header.writeUInt32LE(json.length); header.writeUInt32LE(bundle.length, 4);
        input.push(header, json, bundle); cases.push({request});
      }
    }
  }
}
try {
  const native = spawnSync('target/release/mo-text-worker', ['--layout'],
    {input: Buffer.concat(input), maxBuffer: 64 << 20, timeout: 120000});
  assert.ifError(native.error); assert.equal(native.status, 0, native.stderr?.toString());
  let cursor = 0, overflowingNoWrapLines = 0;
  for (const row of cases) {
    const size = native.stdout.readUInt32LE(cursor); cursor += 4;
    const result = native.stdout.subarray(cursor, cursor + size).toString(); cursor += size;
    assert.equal(wasm.layout_paragraph(JSON.stringify(row.request), bundle, component), result);
    const response = JSON.parse(result); assert.equal(response.status, 'evaluated');
    assert.deepEqual(response.result.issues, []);
    if (row.request.wrapping === 'noWrap') {
      const text = [...row.request.paragraph.text], ends = [];
      text.forEach((ch, i) => { if (ch === '\u2028') ends.push(i + 1); });
      ends.push(text.length);
      assert.deepEqual(response.result.decisions.map(d => d.end.scalarOffset), ends);
      assert.ok(response.result.decisions.every(d => !d.emergency));
      assert.equal(response.result.work.evaluatedCandidates, ends.length);
      overflowingNoWrapLines += response.result.decisions.filter(d => d.overflows).length;
    }
    row.responseSha256 = sha(Buffer.from(result));
  }
  assert.equal(cursor, native.stdout.length); assert.ok(overflowingNoWrapLines > 0);
  const report = {format: 'musteroffice.no-wrap-parity/1', exactResponses: cases.length,
    overflowingNoWrapLines, workerSha256: sha(fs.readFileSync('target/release/mo-text-worker')),
    wasmSha256: sha(fs.readFileSync(path.join(root, 'wasm-web/mo_wasm_bg.wasm'))),
    componentSha256: sha(fs.readFileSync('.codex-work/harfbuzz/release/mo-hb.wasm')),
    fontSha256: sha(bundle), cases};
  fs.writeFileSync(path.join(root, 'flow-parity.json'), JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify({exactResponses: cases.length, overflowingNoWrapLines}));
} finally { component.invalidate(); }
