import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
const root='.codex-work/source-lines',wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const sha=b=>createHash('sha256').update(b).digest('hex');
const cases=[],outputs=[];
function inspect(path){
 const bytes=fs.readFileSync(path),n=spawnSync('target/release/mo-cli',['pptx-inspect',path],{encoding:'utf8',timeout:30000,maxBuffer:64*1024*1024});
 assert.equal(n.status,0,n.stderr);const w=wasm.inspect_pptx(bytes);assert.equal(n.stdout.trimEnd(),w,path);
 return {value:JSON.parse(w),raw:w};
}
for(const c of JSON.parse(fs.readFileSync(root+'/manifest.json')).cases){
 const bytes=fs.readFileSync(c.path);assert.equal(sha(bytes),c.sha256);const {value:r,raw}=inspect(c.path);
 assert.equal(r.status==='error'?r.error.code:r.status,c.error??'inspected',c.name);
 const responsePath=root+'/'+c.name+'.response.json';fs.writeFileSync(responsePath,raw);
 cases.push({name:'inspect-'+c.name,status:r.status==='error'?r.error.code:r.status,sourcePath:c.path,sourceSha256:c.sha256,responsePath,responseSha256:sha(raw)});
 if(r.status==='error')continue;
 const object=r.index.surfaces['/ppt/slides/slide1.xml'].objects[0];
 const request={expectedSourceSha256:c.sha256,edits:[{target:{part:'/ppt/slides/slide1.xml',objectId:object.nativeId,paragraph:0,run:0},expectedText:object.paragraphs[0][0].text,replacement:'line source preserved 中文 & < >'}]};
 const json=JSON.stringify(request),requestPath=root+'/'+c.name+'.edit.json',outputPath=root+'/'+c.name+'.edited.pptx';fs.writeFileSync(requestPath,json);fs.rmSync(outputPath,{force:true});
 const n=spawnSync('target/release/mo-cli',['pptx-edit-text',requestPath,c.path,outputPath],{encoding:'utf8',timeout:30000,maxBuffer:64*1024*1024});assert.equal(n.status,0,n.stderr);
 const output=fs.readFileSync(outputPath);assert.deepEqual(output,Buffer.from(wasm.edit_pptx_text(json,bytes)));assert.equal(JSON.parse(n.stdout).report.sha256,sha(output));
 const {value:after,raw:afterRaw}=inspect(outputPath);assert.equal(after.status,'inspected');
 assert.deepEqual(after.index.surfaces['/ppt/slides/slide1.xml'].objects[0].line,object.line);
 assert.deepEqual(after.index.surfaces['/ppt/slides/slide1.xml'].objects[0].lineReference,object.lineReference);
 const candidateResponsePath=root+'/'+c.name+'.edited.response.json';fs.writeFileSync(candidateResponsePath,afterRaw);
 const item={name:'edit-'+c.name,status:'edited',requestPath,requestSha256:sha(json),sourcePath:c.path,sourceSha256:c.sha256,pptxPath:outputPath,pptxSha256:sha(output),responsePath:candidateResponsePath,responseSha256:sha(afterRaw)};
 cases.push(item);outputs.push(item);
}
const report={format:'musteroffice.source-line-parity/1',nativeCliSha256:sha(fs.readFileSync('target/release/mo-cli')),rustWasmSha256:sha(fs.readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),cases,outputs,exactIndexAndCandidateBytes:true};
fs.writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({batches:cases.length,edited:outputs.length}));
