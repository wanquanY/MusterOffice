// Run existing real public entrypoint suites into fresh outputs, then compare
// every successful authored/source candidate with the frozen parent WASM.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
const root='.codex-work/sealed-export';
const sha=b=>createHash('sha256').update(b).digest('hex');
const entry=p=>{const b=fs.readFileSync(p);return {path:p,byteLength:b.length,sha256:sha(b)};};
const parent=JSON.parse(fs.readFileSync('docs/reviews/evidence/2026-09-26-resource-host-verification.json'));
for(const key of ['rustWasm','rustWasmGlue']) assert.deepEqual(entry(parent.unchangedKernelArtifacts[key].path),parent.unchangedKernelArtifacts[key]);
const old=createRequire(import.meta.url)('../../'+parent.unchangedKernelArtifacts.rustWasmGlue.path);
const historicalManifest=JSON.parse(fs.readFileSync('.codex-work/pptx-source-fixtures/manifest.json'));
const updated=structuredClone(historicalManifest);
const historical=updated.cases.find(c=>c.name==='main-must-understand');
assert.equal(historical.expect.error,'MAPPING_NOT_IMPLEMENTED');
const historicalResponse=JSON.parse(old.inspect_pptx(fs.readFileSync(historical.path)));
assert.deepEqual(historicalResponse.error,{code:'INPUT_INVALID',message:'invalid XML: MCE: unbound compatibility namespace prefix'});
historical.expect.error='INPUT_INVALID';
const malformed=updated.cases.find(c=>c.name==='alternate-content');
assert.deepEqual(malformed.expect,{edit:'MAPPING_NOT_IMPLEMENTED',objects:15,editable:false});
assert.deepEqual(JSON.parse(old.inspect_pptx(fs.readFileSync(malformed.path))).error,{code:'INPUT_INVALID',message:'invalid XML: MCE: Choice/Fallback order or cardinality'});
malformed.expect={error:'INPUT_INVALID'};
if(!fs.existsSync(root+'/source-manifest.json')) fs.writeFileSync(root+'/source-manifest.json',JSON.stringify(updated,null,2)+'\n',{flag:'wx'});
const sourceManifest=JSON.parse(fs.readFileSync(root+'/source-manifest.json'));
assert.deepEqual(sourceManifest,updated);
const reports={};
for(const [name,script,args] of [
 ['authored','tools/verification/pptx-parity.mjs',[]],
 ['source','tools/verification/pptx-source-parity.mjs',[root+'/source-manifest.json']],
 ['editor','tools/verification/native-wasm-parity.mjs',[]],
]) {
 const record=parent.sourceFiles.find(r=>r.path===script);assert.deepEqual(entry(script),record);
 const run=spawnSync(process.execPath,[script,'target/debug/mo-cli',root+'/wasm-node/mo_wasm.js',...args],{encoding:'utf8',timeout:120000,maxBuffer:64*1024*1024});
 fs.writeFileSync(`${root}/${name}-parity.stderr`,run.stderr,{flag:'wx'});
 assert.equal(run.status,0,run.stderr);assert.equal(run.stderr,'');
 fs.writeFileSync(`${root}/${name}-parity.json`,run.stdout,{flag:'wx'});
 reports[name]=JSON.parse(run.stdout);
 if(reports[name].artifactDirectory) assert(!fs.readdirSync(reports[name].artifactDirectory).some(n=>n.startsWith('mo-spool-')));
 console.log(JSON.stringify({suite:name,cases:reports[name].passed??reports[name].cases.length}));
}
let authored=0,rewrites=0,inspections=0,editor=0;
for(const c of reports.authored.cases) {
 const dir=reports.authored.artifactDirectory;
 const request=fs.readFileSync(path.join(dir,c.name+'.json'),'utf8');
 const input=fs.readFileSync(path.join(dir,c.name+'.bin'));
 if(c.status==='exported') {
   const b=Buffer.from(old.export_pptx(request,input));assert.equal(sha(b),c.sha256);
   assert.deepEqual(b,fs.readFileSync(path.join(dir,c.name+'.pptx')));authored++;
 } else assert.throws(()=>old.export_pptx(request,input),e=>String(e).startsWith(c.code+':'));
}
for(const c of reports.source.outputs) {
 const b=Buffer.from(old.edit_pptx_text(fs.readFileSync(c.request,'utf8'),fs.readFileSync(c.source)));
 assert.equal(sha(b),c.sha256);assert.deepEqual(b,fs.readFileSync(c.output));rewrites++;
}
for(const c of reports.source.cases) if(c.response) {
 assert.deepEqual(JSON.parse(old.inspect_pptx(fs.readFileSync(c.source))),c.response);inspections++;
}
for(const c of reports.editor.cases) {
 assert.deepEqual(JSON.parse(old.dispatch_json(c.input)),c.response);editor++;
}
const report={format:'musteroffice.sealed-export-parity/1',native:entry('target/debug/mo-cli'),wasm:entry(root+'/wasm-node/mo_wasm_bg.wasm'),parentWasm:parent.unchangedKernelArtifacts.rustWasm,authoredCases:reports.authored.passed,sourceCases:reports.source.passed??reports.source.cases.length,editorCases:reports.editor.passed,previousAuthoredByteIdentical:authored,previousEditedByteIdentical:rewrites,previousSourceResponsesIdentical:inspections,previousEditorResponsesIdentical:editor,reports:Object.fromEntries(Object.keys(reports).map(k=>[k,entry(`${root}/${k}-parity.json`)])),limitations:['Current authored/source/editor public entrypoints; no new rendering, full PPTX feature or Office/WPS acceptance.']};
fs.writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify({authored,rewrites,inspections,editor}));
