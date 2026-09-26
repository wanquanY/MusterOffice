import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
const root='.codex-work/source-text',wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const sha=b=>createHash('sha256').update(b).digest('hex'),cases=[];
function inspect(path) {
 const b=fs.readFileSync(path),n=spawnSync('target/release/mo-cli',['pptx-inspect',path],{encoding:'utf8',timeout:30000,maxBuffer:64*1024*1024});
 assert.equal(n.status,0,n.stderr);const raw=wasm.inspect_pptx(b);assert.equal(n.stdout.trimEnd(),raw,path);return {raw,value:JSON.parse(raw)};
}
function declarations(index) {
 const i=structuredClone(index);delete i.sourceSha256;delete i.byteLength;
 for(const s of Object.values(i.surfaces)){delete s.sha256;for(const o of s.objects)o.paragraphs=[];}
 return i;
}
for(const c of JSON.parse(fs.readFileSync(root+'/manifest.json')).cases) {
 const b=fs.readFileSync(c.path);assert.equal(sha(b),c.sha256);const {raw,value:r}=inspect(c.path);
 assert.equal(r.status,c.error?'error':'inspected',c.name+raw.slice(0,500));
 const responsePath=root+'/'+c.name+'.response.json';fs.writeFileSync(responsePath,raw);
 cases.push({name:'inspect-'+c.name,status:r.status==='error'?r.error.code:r.status,sourcePath:c.path,sourceSha256:c.sha256,responsePath,responseSha256:sha(raw)});
 if(!c.editable)continue;
 const object=r.index.surfaces['/ppt/slides/slide1.xml'].objects[0];assert.equal(object.paragraphs[0][0].editable,true,c.name);
 const q={expectedSourceSha256:c.sha256,edits:[{target:{part:'/ppt/slides/slide1.xml',objectId:object.nativeId,paragraph:0,run:0},expectedText:object.paragraphs[0][0].text,replacement:'Native styles preserved 中ع & < >'}]};
 const json=JSON.stringify(q),requestPath=root+'/'+c.name+'.edit.json',pptxPath=root+'/'+c.name+'.edited.pptx';fs.writeFileSync(requestPath,json);fs.rmSync(pptxPath,{force:true});
 const n=spawnSync('target/release/mo-cli',['pptx-edit-text',requestPath,c.path,pptxPath],{encoding:'utf8',timeout:30000,maxBuffer:64*1024*1024});assert.equal(n.status,0,n.stderr);
 const output=fs.readFileSync(pptxPath);assert.deepEqual(output,Buffer.from(wasm.edit_pptx_text(json,b)));assert.equal(JSON.parse(n.stdout).report.sha256,sha(output));
 const {raw:afterRaw,value:after}=inspect(pptxPath);assert.equal(after.status,'inspected');assert.deepEqual(declarations(after.index),declarations(r.index),c.name);
 const afterPath=root+'/'+c.name+'.edited.response.json';fs.writeFileSync(afterPath,afterRaw);
 cases.push({name:'edit-'+c.name,status:'edited',sourcePath:c.path,sourceSha256:c.sha256,requestPath,requestSha256:sha(json),pptxPath,pptxSha256:sha(output),responsePath:afterPath,responseSha256:sha(afterRaw),textDeclarationsPreserved:true});
}
fs.writeFileSync(root+'/parity.json',JSON.stringify({format:'musteroffice.source-text-parity/1',nativeCliSha256:sha(fs.readFileSync('target/release/mo-cli')),rustWasmSha256:sha(fs.readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),exactIndexAndCandidateBytes:true,cases},null,2)+'\n');
console.log(JSON.stringify({batches:cases.length,edited:cases.filter(c=>c.status==='edited').length}));
