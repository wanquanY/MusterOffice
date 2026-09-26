import fs from 'node:fs';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
const root='.codex-work/geometry-eval/application-probes',wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const sha=b=>createHash('sha256').update(b).digest('hex');
const cases=[];
for(const c of JSON.parse(fs.readFileSync(root+'/observations.json')).cases){
 const bytes=fs.readFileSync(c.sourcePath);assert.equal(sha(bytes),c.sourceSha256);
 const request={expectedSourceSha256:c.sourceSha256,surface:'/ppt/slides/slide1.xml',objects:[2],profile:'ecma376-2016-ms-presets-draft-v2'};
 const raw=JSON.stringify(request),requestPath=root+'/'+c.name+'.request.json';fs.writeFileSync(requestPath,raw);
 const n=spawnSync('target/release/mo-cli',['pptx-geometry',requestPath,c.sourcePath],{encoding:'utf8',timeout:30000,maxBuffer:16*1024*1024});assert.equal(n.status,0,n.stderr);
 const w=wasm.evaluate_pptx_geometry(raw,bytes);assert.equal(n.stdout.trimEnd(),w,c.name);
 const responsePath=root+'/'+c.name+'.response.json';fs.writeFileSync(responsePath,w);const r=JSON.parse(w);assert.equal(r.status,'evaluated');
 const o=r.geometry.objects[0].outcome;
 cases.push({name:c.name,sourcePath:c.sourcePath,sourceSha256:c.sourceSha256,requestPath,requestSha256:sha(raw),responsePath,responseSha256:sha(w),
   outcome:o.status,reason:o.reason??null,evaluatedWidthPt:o.status==='resolved'?o.geometry.paths[0].commands[1].to.x/12700:null});
}
const report={format:'musteroffice.geometry-application-parity/1',nativeCliSha256:sha(fs.readFileSync('target/release/mo-cli')),rustWasmSha256:sha(fs.readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),cases,exactResponseBytes:true};
fs.writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(cases.map(({name,outcome,reason,evaluatedWidthPt})=>({name,outcome,reason,evaluatedWidthPt}))));
