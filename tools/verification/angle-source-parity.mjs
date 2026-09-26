// Source-preserving text edits must not canonicalize untouched transform tokens.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
const root='.codex-work/angle-export',read=p=>JSON.parse(fs.readFileSync(p));
const sha=b=>createHash('sha256').update(b).digest('hex');
const wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const cases=[];
for(const fixture of read(root+'/source-inputs.json')){
 const source=fs.readFileSync(fixture.sourcePath);assert.equal(sha(source),fixture.sourceSha256);
 const before=JSON.parse(wasm.inspect_pptx(source)).index;
 const part='/ppt/slides/slide2.xml',object=before.surfaces[part].objects.find(o=>o.name==='child:2a');
 const request={expectedSourceSha256:before.sourceSha256,edits:[{target:{part,objectId:object.nativeId,paragraph:0,run:0},expectedText:object.paragraphs[0][0].text,replacement:'Angle preservation '+fixture.angle}]};
 const json=JSON.stringify(request),requestPath=`${root}/${fixture.name}.edit.json`,pptxPath=`${root}/${fixture.name}.edited.pptx`;
 fs.writeFileSync(requestPath,json);fs.rmSync(pptxPath,{force:true});
 const n=spawnSync('target/release/mo-cli',['pptx-edit-text',requestPath,fixture.sourcePath,pptxPath],{encoding:'utf8',maxBuffer:16*1024*1024,timeout:30000});assert.equal(n.status,0,n.stderr);
 const bytes=fs.readFileSync(pptxPath);assert.deepEqual(bytes,Buffer.from(wasm.edit_pptx_text(json,source)));
 const after=JSON.parse(wasm.inspect_pptx(bytes)).index;
 for(const index of [before,after])assert.equal(index.surfaces[part].objects.find(o=>o.name==='group:2').transform.rotation,fixture.angle);
 const responsePath=`${root}/${fixture.name}.index.json`;fs.writeFileSync(responsePath,JSON.stringify(after));
 assert.equal(sha(fs.readFileSync(fixture.sourcePath)),fixture.sourceSha256);
 cases.push({...fixture,requestPath,requestSha256:sha(json),pptxPath,pptxSha256:sha(bytes),responsePath,responseSha256:sha(JSON.stringify(after))});
}
fs.writeFileSync(root+'/source-parity.json',JSON.stringify({format:'musteroffice.angle-source-parity/1',nativeCliSha256:sha(fs.readFileSync('target/release/mo-cli')),rustWasmSha256:sha(fs.readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),cases},null,2)+'\n');console.log(JSON.stringify({sourcePreservationBatches:cases.length}));
