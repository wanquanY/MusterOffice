// The service adds cooperative edit cancellation. Existing native/WASM edit
// requests must retain their exact responses with cancellation disabled.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
const root='.codex-work/operation-host';
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const script='tools/verification/native-wasm-parity.mjs';
const parent=JSON.parse(fs.readFileSync('docs/reviews/evidence/2026-09-26-retained-timing-verification.json'));
assert.deepEqual(entry(script),parent.sourceFiles.find(r=>r.path===script));
const result=spawnSync(process.execPath,[script,'target/debug/mo-cli',root+'/wasm-node/mo_wasm.js'],{encoding:'utf8',timeout:60000,maxBuffer:32*1024*1024});
assert.equal(result.status,0,result.stderr);assert.equal(result.stderr,'');
fs.writeFileSync(root+'/core-parity.json',result.stdout);
const report=JSON.parse(result.stdout);
const old=createRequire(import.meta.url)('../../.codex-work/retained-timing/wasm-node/mo_wasm.js');
const current=createRequire(import.meta.url)('../../'+root+'/wasm-node/mo_wasm.js');
assert.deepEqual(entry(parent.currentArtifacts.rustWasm.path),parent.currentArtifacts.rustWasm);
assert.deepEqual(entry(parent.currentArtifacts.rustWasmGlue.path),parent.currentArtifacts.rustWasmGlue);
for(const c of report.cases){
  const before=old.dispatch_json(c.input),after=current.dispatch_json(c.input);
  assert.equal(after,before,c.name);assert.equal(after,JSON.stringify(c.response),c.name);
}
fs.writeFileSync(root+'/edit-regression.json',JSON.stringify({format:'musteroffice.operation-host-edit-regression/1',scope:'Existing document computation requests, not PPTX, rendering or Office/WPS acceptance.',requests:report.passed,previousResponsesByteIdentical:true,coreParity:entry(root+'/core-parity.json'),program:entry(script),parentWasm:entry(parent.currentArtifacts.rustWasm.path),currentWasm:entry(root+'/wasm-node/mo_wasm_bg.wasm')},null,2)+'\n');
console.log(JSON.stringify({requests:report.passed,previousResponsesByteIdentical:true}));
