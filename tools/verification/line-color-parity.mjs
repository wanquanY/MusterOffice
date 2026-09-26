// Actual PPTX bytes -> shared Rust line inheritance and working-precision colors.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
const root='.codex-work/line-colors',wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const sha=b=>createHash('sha256').update(b).digest('hex'),read=p=>JSON.parse(fs.readFileSync(p));
const cases=[];
function native(args){const r=spawnSync('target/release/mo-cli',args,{encoding:'utf8',timeout:30000,maxBuffer:64*1024*1024});assert.equal(r.status,0,r.stderr);return r.stdout.trimEnd();}
function run(name,path,request,expected='evaluated'){
 const source=fs.readFileSync(path),json=typeof request==='string'?request:JSON.stringify(request),requestPath=`${root}/${name}.request.json`;fs.writeFileSync(requestPath,json);
 const raw=native(['pptx-line-colors',requestPath,path]);assert.equal(raw,wasm.resolve_pptx_line_colors(json,source),name);const r=JSON.parse(raw);
 assert.equal(r.status==='error'?r.error.code:r.status,expected,name);
 const responsePath=`${root}/${name}.response.json`;fs.writeFileSync(responsePath,raw);
 const item={name,sourcePath:path,sourceSha256:sha(source),requestPath,requestSha256:sha(json),responsePath,responseSha256:sha(raw),status:expected};cases.push(item);return {result:r,item};
}
const manifests=[['color',read(root+'/manifest.json').cases],['style',read('.codex-work/line-style/manifest.json').cases]];
for(const [group,inputs] of manifests)for(const c of inputs){
 const source=fs.readFileSync(c.path);assert.equal(sha(source),c.sha256);const idx=JSON.parse(wasm.inspect_pptx(source)).index;
 const ids=c.objects??[c.object],request={expectedSourceSha256:c.sha256,surface:'/ppt/slides/slide1.xml',objects:ids,lineProfile:'ms-oi29500-lines-2024-draft-v1',colorProfile:'ecma376-2016-draft-v1',context:c.context??{systemColors:{},placeholder:null}};
 const {result}=run(`${group}-${c.name}`,c.path,request,c.expected??'evaluated');if(result.status==='error')continue;
 const old=JSON.parse(wasm.resolve_pptx_lines(JSON.stringify({expectedSourceSha256:request.expectedSourceSha256,surface:request.surface,objects:ids,profile:request.lineProfile}),source));
 assert.deepEqual(result.colors.objects.map(c=>({nativeId:c.nativeId,outcome:c.style})),old.styles.objects);
 const obj=idx.surfaces[request.surface].objects.find(o=>o.nativeId===ids[0]);
 const edit={expectedSourceSha256:c.sha256,edits:[{target:{part:request.surface,objectId:ids[0],paragraph:0,run:0},expectedText:obj.paragraphs[0][0].text,replacement:'Native color expressions retained 中文'}]};
 const editRaw=JSON.stringify(edit),editPath=`${root}/${group}-${c.name}.edit.json`,pptxPath=`${root}/${group}-${c.name}.edited.pptx`;fs.writeFileSync(editPath,editRaw);fs.rmSync(pptxPath,{force:true});native(['pptx-edit-text',editPath,c.path,pptxPath]);
 const candidate=fs.readFileSync(pptxPath);assert.deepEqual(candidate,Buffer.from(wasm.edit_pptx_text(editRaw,source)));
 const {result:after,item}=run(`edited-${group}-${c.name}`,pptxPath,{...request,expectedSourceSha256:sha(candidate)});assert.deepEqual(after.colors.objects,result.colors.objects);
 item.editRequestPath=editPath;item.editRequestSha256=sha(editRaw);item.originalSourcePath=c.path;item.originalSourceSha256=c.sha256;item.colorSemanticsPreserved=true;
}
const c=manifests[0][1][0],q={expectedSourceSha256:c.sha256,surface:'/ppt/slides/slide1.xml',objects:c.objects,lineProfile:'ms-oi29500-lines-2024-draft-v1',colorProfile:'ecma376-2016-draft-v1',context:{systemColors:{},placeholder:null}};
for(const [name,mutate,status,validRequest] of [
 ['duplicate-object',q=>({...q,objects:[100,100]}),'evaluated',true],
 ['empty-query',q=>({...q,objects:[]}),'evaluated',true],
 ['all-object-kinds',q=>({...q,surface:'/ppt/slides/slide2.xml',objects:JSON.parse(wasm.inspect_pptx(fs.readFileSync(c.path))).index.surfaces['/ppt/slides/slide2.xml'].objects.map(o=>o.nativeId)}),'evaluated',true],
 ['wrong-source',q=>({...q,expectedSourceSha256:'0'.repeat(64)}),'SOURCE_CONFLICT',true],
 ['wrong-surface',q=>({...q,surface:'/missing.xml'}),'INPUT_INVALID',true],
 ['unknown-object',q=>({...q,objects:[100,4294967295]}),'INPUT_INVALID',true],
 ['query-budget',q=>({...q,objects:Array(257).fill(100)}),'LIMIT_EXCEEDED',true],
 ['unknown-line-profile',q=>({...q,lineProfile:'unverified'}),'INPUT_INVALID',false],
 ['unknown-color-profile',q=>({...q,colorProfile:'unverified'}),'INPUT_INVALID',false],
 ['unknown-field',q=>({...q,arbitrary:'unrecognized'}),'INPUT_INVALID',false],
 ['negative-object',q=>({...q,objects:[-1]}),'INPUT_INVALID',false],
 ['duplicate-key',q=>JSON.stringify(q).replace('"objects":','"objects":[],"objects":'),'INPUT_INVALID',false],
 ['invalid-context',q=>({...q,context:{systemColors:{},placeholder:[256,0,0,255]}}),'INPUT_INVALID',false],
])run('contract-'+name,c.path,mutate(q),status).item.validRequest=validRequest;
const report={format:'musteroffice.line-color-parity/1',nativeCliSha256:sha(fs.readFileSync('target/release/mo-cli')),rustWasmSha256:sha(fs.readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),exactResponseAndEditCandidateBytes:true,cases};
fs.writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({batches:cases.length,editQueries:cases.filter(c=>c.colorSemanticsPreserved).length}));
