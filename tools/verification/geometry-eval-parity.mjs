import fs from 'node:fs';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
const root='.codex-work/geometry-eval',wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const sha=b=>createHash('sha256').update(b).digest('hex');
const cases=[];
function evaluate(c,request,bytes,name,validRequest=true){
 const raw=JSON.stringify(request),requestPath=root+'/'+name+'.request.json';fs.writeFileSync(requestPath,raw);
 const n=spawnSync('target/release/mo-cli',['pptx-geometry',requestPath,c.path],{encoding:'utf8',timeout:30000,maxBuffer:64*1024*1024});
 assert.equal(n.status,0,n.stderr);const w=wasm.evaluate_pptx_geometry(raw,bytes);assert.equal(n.stdout.trimEnd(),w,name);
 const responsePath=root+'/'+name+'.response.json';fs.writeFileSync(responsePath,w);const result=JSON.parse(w);
 const record={name,sourcePath:c.path,sourceSha256:sha(bytes),requestPath,requestSha256:sha(raw),responsePath,responseSha256:sha(w),validRequest,status:result.status};
 cases.push(record);return result;
}
for(const c of JSON.parse(fs.readFileSync(root+'/manifest.json')).cases){
 const bytes=fs.readFileSync(c.path);assert.equal(sha(bytes),c.sha256);
 const request={expectedSourceSha256:c.sha256,surface:c.part,objects:[c.objectId],profile:'ecma376-2016-ms-presets-draft-v2'};
 const r=evaluate(c,request,bytes,c.name);assert.equal(r.status,'evaluated',c.name);
 const o=r.geometry.objects[0].outcome;
 if(c.expectedOutcome){const actual=o.status==='resolved'?'resolved':o.reason.issue??o.reason.kind;assert.equal(actual,c.expectedOutcome,c.name);}
 // Edit a real leaf text in the same source object. The geometry query must
 // remain equivalent, with only the source digest changing.
 if(o.status==='resolved'){
  const inspection=JSON.parse(wasm.inspect_pptx(bytes));const obj=inspection.index.surfaces[c.part].objects.find(o=>o.nativeId===c.objectId);
  if(obj.paragraphs[0]?.[0]?.editable){
   const edit={expectedSourceSha256:c.sha256,edits:[{target:{part:c.part,objectId:c.objectId,paragraph:0,run:0},expectedText:obj.paragraphs[0][0].text,replacement:'geometry evaluated 中文'}]};
   const editRequestPath=root+'/'+c.name+'.edit.json',outputPath=root+'/'+c.name+'.edited.pptx';const text=JSON.stringify(edit);fs.writeFileSync(editRequestPath,text);fs.rmSync(outputPath,{force:true});
   const n=spawnSync('target/release/mo-cli',['pptx-edit-text',editRequestPath,c.path,outputPath],{encoding:'utf8',timeout:30000});assert.equal(n.status,0,n.stderr);
   const output=fs.readFileSync(outputPath);assert.deepEqual(output,Buffer.from(wasm.edit_pptx_text(text,bytes)));
   const after=evaluate({...c,path:outputPath},{...request,expectedSourceSha256:sha(output)},output,'edited-'+c.name);
   assert.deepEqual(after.geometry.objects,r.geometry.objects);
   Object.assign(cases.at(-1),{originalSourcePath:c.path,originalSourceSha256:c.sha256,editRequestPath,editRequestSha256:sha(text),geometryValuesPreserved:true});
  }
 }
}
const first=JSON.parse(fs.readFileSync(root+'/manifest.json')).cases[0],bytes=fs.readFileSync(first.path),request={expectedSourceSha256:first.sha256,surface:first.part,objects:[first.objectId],profile:'ecma376-2016-ms-presets-draft-v2'};
for(const [name,change,code,valid] of [
 ['wrong-source',{expectedSourceSha256:'0'.repeat(64)},'SOURCE_CONFLICT',true],['missing-object',{objects:[4294967295]},'INPUT_INVALID',true],
 ['missing-surface',{surface:'/missing.xml'},'INPUT_INVALID',true],['too-many',{objects:Array(257).fill(first.objectId)},'LIMIT_EXCEEDED',true],
 ['profile',{profile:'invented'},'INPUT_INVALID',false],['unknown-field',{script:'run()'},'INPUT_INVALID',false]
]){const r=evaluate(first,{...request,...change},bytes,'reject-'+name,valid);assert.equal(r.error.code,code,name);}
const report={format:'musteroffice.geometry-evaluation-parity/1',nativeCliSha256:sha(fs.readFileSync('target/release/mo-cli')),rustWasmSha256:sha(fs.readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),cases,exactResponseAndEditCandidateBytes:true};
fs.writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({batches:cases.length,edited:cases.filter(c=>c.geometryValuesPreserved).length}));
