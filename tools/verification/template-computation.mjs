// Run the same owned corpus through native CLI, WASM, and the Rust test oracle.
import assert from 'node:assert/strict';
import { readFile, readdir, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { spawnSync } from 'node:child_process';
import { resolve, join } from 'node:path';

const [corpusArg, cliArg, wasmArg, reportArg] = process.argv.slice(2);
assert.ok(corpusArg && cliArg && wasmArg && reportArg, 'usage: template-computation.mjs <corpus> <cli> <wasm-js> <new-report>');
const corpus = resolve(corpusArg), cli = resolve(cliArg), wasmPath = resolve(wasmArg);
const wasm = createRequire(import.meta.url)(wasmPath);
const sha = value => createHash('sha256').update(value).digest('hex');
const cases = [];
for (const file of (await readdir(corpus)).filter(name=>name.endsWith('.request.json')).sort()) {
  const request = await readFile(join(corpus,file),'utf8');
  const expected = JSON.parse(await readFile(join(corpus,file.replace('.request.','.response.')),'utf8'));
  const native = spawnSync(cli,['template',join(corpus,file)], {encoding:'utf8', maxBuffer: 40*1024*1024});
  assert.ifError(native.error);
  assert.equal(native.status,0,native.stderr);
  const wasmOutput = wasm.compute_template(request);
  assert.equal(native.stdout.trimEnd(),wasmOutput,`exact native/WASM wire bytes: ${file}`);
  assert.deepEqual(JSON.parse(wasmOutput),expected,`independent Rust test oracle: ${file}`);
  cases.push({name:file,requestSha256:sha(request),responseSha256:sha(wasmOutput),status:expected.status});
}
assert.ok(cases.length >= 15,'include native, authored, discovery and rejected requests');
const report = {nativeSha256:sha(await readFile(cli)), wasmSha256:sha(await readFile(wasmPath.replace(/\.js$/, '_bg.wasm'))), cases};
await writeFile(resolve(reportArg),JSON.stringify(report,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({cases:cases.length,exactWireEquality:true,report:resolve(reportArg)}));
