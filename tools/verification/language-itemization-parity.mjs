/** Compare real native/WASM paragraph shaping and line layout with authored
 * language metadata. Owned synthetic font only; not Office/WPS certification.
 * Usage: node language-itemization-parity.mjs worker wasm.js text.js hbDir output
 */
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { createRequire } from 'node:module';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';

const args = process.argv.slice(2);
assert.equal(args.length, 5, 'worker wasm.js text.js hbDir output');
const [worker, modulePath, textPath, hbRoot, output] = args.map(p => resolve(p));
const read = p => readFileSync(p);
const hash = b => createHash('sha256').update(b).digest('hex');
const wasm = createRequire(import.meta.url)(modulePath);
const { ShapingComponent } = await import(pathToFileURL(textPath));
const { default: factory } = await import(pathToFileURL(join(hbRoot, 'mo-hb.mjs')));
const component = await ShapingComponent.create(factory, new WebAssembly.Module(read(join(hbRoot, 'mo-hb.wasm'))));
const bundle = read('fixtures/fonts/owned-hanging.ttf');
const fonts = JSON.parse(read('fixtures/fonts/manifest-hanging.json')).manifest.fonts;
assert.equal(hash(bundle), fonts[0].expectedSha256);
const cases = [];
for (const text of ['AA。', '中中。', 'AA!']) {
  for (const language of ['zh-CN', 'ja', 'ko-KR', 'und-Hani', 'en-x-Hani', 'und']) {
    const paragraph = { text, direction: 'leftToRight', spans: [{ end: [...text].length, style: 0 }], fonts,
      styles: [{ language, features: [], candidates: [{ font: 0, variations: [] }], suppressDottedCircle: false, maxGlyphs: 100 }] };
    cases.push({ mode: '--paragraph', entry: 'shape_paragraph', language, text, request: paragraph });
    for (const hangingPunctuation of ['none', 'end']) {
      cases.push({ mode: '--layout', entry: 'layout_paragraph', language, text,
        request: { paragraph, styles: [{ fontSize: '228600', baselineShift: '0' }], strutStyle: 0,
          spacing: { kind: 'natural' }, width: '284320', overflow: 'emergencyGrapheme', hangingPunctuation } });
    }
  }
}
try {
  for (const mode of ['--paragraph', '--layout']) {
    const rows = cases.filter(row => row.mode === mode);
    const input = rows.flatMap(row => {
      const json = Buffer.from(JSON.stringify(row.request));
      const header = Buffer.alloc(8);
      header.writeUInt32LE(json.length); header.writeUInt32LE(bundle.length, 4);
      return [header, json, bundle];
    });
    const native = spawnSync(worker, [mode], { input: Buffer.concat(input), timeout: 60000, maxBuffer: 32 << 20 });
    assert.ifError(native.error); assert.equal(native.status, 0, native.stderr.toString());
    let cursor = 0;
    for (const row of rows) {
      const n = native.stdout.readUInt32LE(cursor); cursor += 4;
      const raw = native.stdout.subarray(cursor, cursor + n).toString(); cursor += n;
      assert.equal(wasm[row.entry](JSON.stringify(row.request), bundle, component), raw);
      row.response = JSON.parse(raw);
      assert.equal(row.response.status, 'evaluated', raw);
      if (mode === '--paragraph') {
        const items = row.response.result.itemization;
        const ambiguous = row.text === 'AA。' && ['en-x-Hani', 'und'].includes(row.language);
        assert.equal(items.notices.length, Number(ambiguous));
        if (!ambiguous && row.text === 'AA。') {
          assert.equal(items.items[0].script, 'Latn');
          assert.equal(items.items.at(-1).script, row.language === 'ja' ? 'Kana' : row.language === 'ko-KR' ? 'Hang' : 'Hani');
        }
      }
    }
    assert.equal(cursor, native.stdout.length);
  }
} finally { component.invalidate(); }
mkdirSync(output, { recursive: true });
writeFileSync(join(output, 'report.json'), JSON.stringify({
  profile: 'authored-language-native-wasm-parity/1', exactResponses: cases.length,
  nativeSha256: hash(read(worker)), wasmSha256: hash(read(modulePath.replace(/\.js$/, '_bg.wasm'))),
  componentSha256: hash(read(join(hbRoot, 'mo-hb.wasm'))), fontSha256: hash(bundle),
  officeWpsProven: false, cases,
}, null, 2) + '\n', { flag: 'wx' });
console.log(JSON.stringify({ exactResponses: cases.length, officeWpsProven: false }));
