// Actual PPTX bytes -> shared Rust fill inheritance and working-precision colors.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
const root='.codex-work/fill-colors',wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const sha=b=>createHash('sha256').update(b).digest('hex'),read=p=>JSON.parse(fs.readFileSync(p));
const cases=[];
function native(args){const r=spawnSync('target/release/mo-cli',args,{encoding:'utf8',timeout:30000,maxBuffer:64*1024*1024});assert.equal(r.status,0,r.stderr);return r.stdout.trimEnd();}
function run(name,path,request,expected='evaluated'){
 const source=fs.readFileSync(path),json=typeof request==='string'?request:JSON.stringify(request),requestPath=`${root}/${name}.request.json`;fs.writeFileSync(requestPath,json);
 const raw=native(['pptx-fill-colors',requestPath,path]);assert.equal(raw,wasm.resolve_pptx_fill_colors(json,source),name);const r=JSON.parse(raw);
 assert.equal(r.status==='error'?r.error.code:r.status,expected,name);
 const responsePath=`${root}/${name}.response.json`;fs.writeFileSync(responsePath,raw);
 const item={name,sourcePath:path,sourceSha256:sha(source),requestPath,requestSha256:sha(json),responsePath,responseSha256:sha(raw),status:expected};cases.push(item);return {result:r,item};
}
const manifests=[['color',read(root+'/manifest.json').cases],['style',read('.codex-work/fill-resolution/manifest.json').cases],['effect',read('.codex-work/source-effects/manifest.json').cases]];
for(const [group,inputs] of manifests)for(const c of inputs){
 const source=fs.readFileSync(c.path);assert.equal(sha(source),c.sha256);const idx=JSON.parse(wasm.inspect_pptx(source)).index;
 const surface='/ppt/slides/slide1.xml';const object=idx?.surfaces[surface]?.objects.find(o=>o.kind==='shape');
 const targets=group==='color'?c.targets??c.objects.map(nativeId=>({kind:'object',nativeId})):group==='style'?[{kind:'object',nativeId:c.nativeId},{kind:'line',nativeId:c.nativeId},{kind:'rootGroup'},{kind:'background'}]:[{kind:'object',nativeId:object?.nativeId??1},{kind:'background'}];
 const request={expectedSourceSha256:c.sha256,surface,targets,fillProfile:'ms-oi29500-fills-2024-draft-v1',colorProfile:'ecma376-2016-draft-v1',context:c.context??{systemColors:{},placeholder:null}};
 const {result,item:initial}=run(`${group}-${c.name}`,c.path,request,c.expected??c.error??'evaluated');if(result.status==='error')continue;
 const old=JSON.parse(wasm.resolve_pptx_fills(JSON.stringify({expectedSourceSha256:request.expectedSourceSha256,surface,targets,profile:request.fillProfile}),source));
 assert.deepEqual(result.colors.targets.map(c=>({target:c.target,outcome:c.style})),old.styles.targets);
 const stylesPath=`${root}/${group}-${c.name}.styles.json`,stylesRaw=JSON.stringify(old);fs.writeFileSync(stylesPath,stylesRaw);initial.stylesPath=stylesPath;initial.stylesSha256=sha(stylesRaw);
 const obj=idx.surfaces[surface].objects.find(o=>o.nativeId===targets[0].nativeId)??object;
 const edit={expectedSourceSha256:c.sha256,edits:[{target:{part:surface,objectId:obj.nativeId,paragraph:0,run:0},expectedText:obj.paragraphs[0][0].text,replacement:'Native fill colors retained 中文'}]};
 const editRaw=JSON.stringify(edit),editPath=`${root}/${group}-${c.name}.edit.json`,pptxPath=`${root}/${group}-${c.name}.edited.pptx`;fs.writeFileSync(editPath,editRaw);fs.rmSync(pptxPath,{force:true});native(['pptx-edit-text',editPath,c.path,pptxPath]);
 const candidate=fs.readFileSync(pptxPath);assert.deepEqual(candidate,Buffer.from(wasm.edit_pptx_text(editRaw,source)));
 const {result:after,item}=run(`edited-${group}-${c.name}`,pptxPath,{...request,expectedSourceSha256:sha(candidate)});assert.deepEqual(after.colors.targets,result.colors.targets);
 item.editRequestPath=editPath;item.editRequestSha256=sha(editRaw);item.originalSourcePath=c.path;item.originalSourceSha256=c.sha256;item.colorSemanticsPreserved=true;
}
const c=manifests[0][1][0],q={expectedSourceSha256:c.sha256,surface:'/ppt/slides/slide1.xml',targets:c.objects.map(nativeId=>({kind:'object',nativeId})),fillProfile:'ms-oi29500-fills-2024-draft-v1',colorProfile:'ecma376-2016-draft-v1',context:{systemColors:{},placeholder:null}};
for(const [name,mutate,status,validRequest] of [
 ['duplicate-object',q=>({...q,targets:[{kind:'object',nativeId:100},{kind:'object',nativeId:100}]}),'evaluated',true],
 ['empty-query',q=>({...q,targets:[]}),'evaluated',true],
 ['all-object-kinds',q=>({...q,surface:'/ppt/slides/slide2.xml',targets:JSON.parse(wasm.inspect_pptx(fs.readFileSync(c.path))).index.surfaces['/ppt/slides/slide2.xml'].objects.map(o=>({kind:'object',nativeId:o.nativeId}))}),'evaluated',true],
 ['wrong-source',q=>({...q,expectedSourceSha256:'0'.repeat(64)}),'SOURCE_CONFLICT',true],
 ['wrong-surface',q=>({...q,surface:'/missing.xml'}),'INPUT_INVALID',true],
 ['unknown-object',q=>({...q,targets:[{kind:'object',nativeId:100},{kind:'object',nativeId:4294967295}]}),'INPUT_INVALID',true],
 ['query-budget',q=>({...q,targets:Array.from({length:257},()=>({kind:'object',nativeId:100}))}),'LIMIT_EXCEEDED',true],
 ['unknown-fill-profile',q=>({...q,fillProfile:'unverified'}),'INPUT_INVALID',false],
 ['unknown-color-profile',q=>({...q,colorProfile:'unverified'}),'INPUT_INVALID',false],
 ['unknown-field',q=>({...q,arbitrary:'unrecognized'}),'INPUT_INVALID',false],
 ['negative-object',q=>({...q,targets:[{kind:'object',nativeId:-1}]}),'INPUT_INVALID',false],
 ['duplicate-key',q=>JSON.stringify(q).replace('"targets":','"targets":[],"targets":'),'INPUT_INVALID',false],
 ['invalid-context',q=>({...q,context:{systemColors:{},placeholder:[256,0,0,255]}}),'INPUT_INVALID',false],
])run('contract-'+name,c.path,mutate(q),status).item.validRequest=validRequest;
const report={format:'musteroffice.fill-color-parity/1',nativeCliSha256:sha(fs.readFileSync('target/release/mo-cli')),rustWasmSha256:sha(fs.readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),exactResponseAndEditCandidateBytes:true,cases};
fs.writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({batches:cases.length,editQueries:cases.filter(c=>c.colorSemanticsPreserved).length}));
