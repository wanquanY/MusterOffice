// Verify the current debug CLI against the frozen, unchanged Rust WASM.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
const root='.codex-work/job-results';
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const parent=JSON.parse(fs.readFileSync('docs/reviews/evidence/2026-09-26-sealed-export-verification.json'));
for(const k of ['rustWasm','rustWasmGlue']) assert.deepEqual(entry(parent.currentArtifacts[k].path),parent.currentArtifacts[k]);
const reports={};
for(const [name,script,args] of [
 ['authored','tools/verification/pptx-parity.mjs',[]],
 ['source','tools/verification/pptx-source-parity.mjs',['.codex-work/sealed-export/source-manifest.json']],
 ['editor','tools/verification/native-wasm-parity.mjs',[]],
]) {
 assert.deepEqual(entry(script),parent.sourceFiles.find(r=>r.path===script));
 const run=spawnSync(process.execPath,[script,'target/debug/mo-cli',parent.currentArtifacts.rustWasmGlue.path,...args],{encoding:'utf8',timeout:120000,maxBuffer:64*1024*1024});
 fs.writeFileSync(`${root}/${name}-parity.stderr`,run.stderr,{flag:'wx'});
 fs.writeFileSync(`${root}/${name}-parity.json`,run.stdout,{flag:'wx'});
 assert.equal(run.status,0,run.stderr);assert.equal(run.stderr,'');
 reports[name]=JSON.parse(run.stdout);
 console.log(JSON.stringify({suite:name,cases:reports[name].passed??reports[name].cases.length}));
}
const report={format:'musteroffice.job-results-parity/1',native:entry('target/debug/mo-cli'),wasm:parent.currentArtifacts.rustWasm,authoredCases:reports.authored.passed,sourceCases:reports.source.passed??reports.source.cases.length,editorCases:reports.editor.passed,reports:Object.fromEntries(Object.keys(reports).map(k=>[k,entry(`${root}/${k}-parity.json`)])),limitations:['Actual public regression against frozen WASM; no new renderer or complete feature/target-application acceptance.']};
fs.writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n',{flag:'wx'});
