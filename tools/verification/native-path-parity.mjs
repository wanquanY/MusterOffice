import fs from 'node:fs';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
const root='.codex-work/native-paths',wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const sha=b=>createHash('sha256').update(b).digest('hex'),cases=[];
const manifest=JSON.parse(fs.readFileSync(root+'/manifest.json'));
function run(c,request,name=c.name,validRequest=true){
 const bytes=fs.readFileSync(c.path);assert.equal(sha(bytes),c.sha256);
 const raw=JSON.stringify(request),requestPath=root+'/'+name+'.request.json';fs.writeFileSync(requestPath,raw);
 const n=spawnSync('target/release/mo-cli',['pptx-paths',requestPath,c.path],{encoding:'utf8',maxBuffer:128*1024*1024,timeout:60000});assert.equal(n.status,0,n.stderr);
 const w=wasm.compile_pptx_paths(raw,bytes);assert.equal(n.stdout.trimEnd(),w,name);
 const responsePath=root+'/'+name+'.response.json';fs.writeFileSync(responsePath,w);
 const record={name,requestPath,requestSha256:sha(raw),responsePath,responseSha256:sha(w),sourcePath:c.path,sourceSha256:c.sha256,validRequest};
 cases.push(record);return [JSON.parse(w),record];
}
function request(c){return {geometry:{expectedSourceSha256:c.sha256,surface:c.part,objects:[c.objectId],profile:'ecma376-2016-ms-presets-draft-v2'},options:{profile:'drawingml-polar-arcs-q96-hermite-v1-draft',coordinateTolerance:c.tolerance??'4294967296'}};}
let compiled=0,unresolved=0;
for(const c of manifest.cases){
 const q=request(c),[r,record]=run(c,q);assert.equal(r.status,'compiled',c.name);const o=r.paths.objects[0].outcome;
 if(c.pathExpected) assert.equal(o.status==='compiled'?'compiled':o.issue,c.pathExpected,c.name);
 if(o.status==='compiled')compiled++;else unresolved++;
 const raw=JSON.stringify(q.geometry),path=root+'/'+c.name+'.evaluated.json';
 const evalRequestPath=root+'/'+c.name+'.evaluate-request.json';fs.writeFileSync(evalRequestPath,raw);
 const native=spawnSync('target/release/mo-cli',['pptx-geometry',evalRequestPath,c.path],{encoding:'utf8',maxBuffer:64*1024*1024,timeout:30000});assert.equal(native.status,0,native.stderr);
 const evaluated=wasm.evaluate_pptx_geometry(raw,fs.readFileSync(c.path));assert.equal(native.stdout.trimEnd(),evaluated);fs.writeFileSync(path,evaluated);
 Object.assign(record,{evaluationPath:path,evaluationSha256:sha(evaluated),evaluationRequestPath:evalRequestPath,evaluationRequestSha256:sha(raw)});
}
const first=manifest.cases[0];
for(const [name,change,code,valid] of [
 ['digest',q=>q.geometry.expectedSourceSha256='0'.repeat(64),'SOURCE_CONFLICT',true],
 ['unknown-object',q=>q.geometry.objects=[4294967295],'INPUT_INVALID',true],
 ['count',q=>q.geometry.objects=Array(257).fill(first.objectId),'LIMIT_EXCEEDED',true],
 ['zero-tolerance',q=>q.options.coordinateTolerance='0','INPUT_INVALID',true],
 ['profile',q=>q.options.profile='unknown','INPUT_INVALID',false],
 ['unknown',q=>q.options.hiddenRepair=true,'INPUT_INVALID',false],
]){const q=request(first);change(q);const [r]=run(first,q,'reject-'+name,valid);assert.equal(r.error.code,code,name);}
const report={format:'musteroffice.native-path-parity/1',nativeCliSha256:sha(fs.readFileSync('target/release/mo-cli')),rustWasmSha256:sha(fs.readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),cases,exactResponseBytes:true,compiled,unresolved};
fs.writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({batches:cases.length,evaluationBatches:manifest.cases.length,compiled,unresolved}));
const owned=new Set(manifest.cases.filter(c=>c.owned).map(c=>c.name));
const guideCases=cases.filter(c=>owned.has(c.name)).map(c=>({name:c.name,sourcePath:c.sourcePath,sourceSha256:c.sourceSha256,requestPath:c.evaluationRequestPath,requestSha256:c.evaluationRequestSha256,responsePath:c.evaluationPath,responseSha256:c.evaluationSha256}));
fs.writeFileSync(root+'/guide-parity.json',JSON.stringify({format:'musteroffice.native-path-guides-parity/1',nativeCliSha256:report.nativeCliSha256,rustWasmSha256:report.rustWasmSha256,cases:guideCases,exactResponseBytes:true},null,2)+'\n');
