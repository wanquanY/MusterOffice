import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
const root='.codex-work/text-style',wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const sha=b=>createHash('sha256').update(b).digest('hex'),cases=[];
function native(args){const r=spawnSync('target/release/mo-cli',args,{encoding:'utf8',timeout:30000,maxBuffer:64*1024*1024});assert.equal(r.status,0,r.stderr);return r.stdout.trimEnd();}
function save(name,raw){const responsePath=root+'/'+name+'.response.json';fs.writeFileSync(responsePath,raw);return {responsePath,responseSha256:sha(raw)};}
for(const c of JSON.parse(fs.readFileSync(root+'/manifest.json')).cases){
 const b=fs.readFileSync(c.sourcePath);assert.equal(sha(b),c.sourceSha256);
 const inspected=wasm.inspect_pptx(b);assert.equal(native(['pptx-inspect',c.sourcePath]),inspected,c.name);
 const index=JSON.parse(inspected);assert.equal(index.status,c.status==='error'?'error':'inspected',c.name);
 cases.push({name:c.name,kind:'source',sourcePath:c.sourcePath,sourceSha256:c.sourceSha256,...save(c.name+'.source',inspected)});
 const q={expectedSourceSha256:c.sourceSha256,surface:c.slide,objects:[c.object,c.object],profile:'drawingml-body-inheritance-draft-v1'};
 function query(path,bytes,q,name){const request=JSON.stringify(q),requestPath=root+'/'+name+'.request.json';fs.writeFileSync(requestPath,request);const raw=wasm.resolve_pptx_text_bodies(request,bytes);assert.equal(native(['pptx-text-bodies',requestPath,path]),raw,name);const r=JSON.parse(raw);cases.push({name,kind:'query',sourcePath:path,sourceSha256:sha(bytes),requestPath,requestSha256:sha(request),...save(name,raw)});return r;}
 const result=query(c.sourcePath,b,q,c.name);
 if(c.status==='error'){assert.equal(result.status,'error');continue;}
 assert.equal(result.status,'evaluated',c.name);assert.equal(result.styles.objects[0].outcome.status,c.status,c.name);assert.deepEqual(result.styles.objects[0],result.styles.objects[1]);
 const object=index.index.surfaces[c.slide].objects.find(o=>o.nativeId===c.object);
 const edit={expectedSourceSha256:c.sourceSha256,edits:[{target:{part:c.slide,objectId:c.object,paragraph:0,run:0},expectedText:object.paragraphs[0][0].text,replacement:'Native body preserved 中ع'}]};
 const json=JSON.stringify(edit),requestPath=root+'/'+c.name+'.edit.json',path=root+'/'+c.name+'.edited.pptx';fs.writeFileSync(requestPath,json);fs.rmSync(path,{force:true});const report=JSON.parse(native(['pptx-edit-text',requestPath,c.sourcePath,path]));const output=fs.readFileSync(path);assert.deepEqual(output,Buffer.from(wasm.edit_pptx_text(json,b)));assert.equal(report.report.sha256,sha(output));
 const after=query(path,output,{...q,expectedSourceSha256:sha(output)},c.name+'.edited');delete after.styles.sourceSha256;delete result.styles.sourceSha256;assert.deepEqual(after,result,c.name);
 const last=cases.at(-1);Object.assign(last,{editRequestPath:requestPath,editRequestSha256:sha(json),textBodyPreserved:true});
}
fs.writeFileSync(root+'/parity.json',JSON.stringify({format:'musteroffice.text-body-parity/1',nativeCliSha256:sha(fs.readFileSync('target/release/mo-cli')),rustWasmSha256:sha(fs.readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),exactNativeWasmResponsesAndEdits:true,cases},null,2)+'\n');
console.log(JSON.stringify({batches:cases.length,edited:cases.filter(c=>c.textBodyPreserved).length}));
