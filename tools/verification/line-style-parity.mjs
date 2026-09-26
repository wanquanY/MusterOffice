// Invoke the real Native/WASM line query, then edit native source and query again.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
const root='.codex-work/line-style',wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const sha=b=>createHash('sha256').update(b).digest('hex');
const cases=[];
function native(args){const n=spawnSync('target/release/mo-cli',args,{encoding:'utf8',timeout:30000,maxBuffer:64*1024*1024});assert.equal(n.status,0,n.stderr);return n.stdout.trimEnd();}
function run(name,path,request,expected){
 const bytes=fs.readFileSync(path),json=typeof request==='string'?request:JSON.stringify(request),requestPath=`${root}/${name}.request.json`;
 fs.writeFileSync(requestPath,json);const a=native(['pptx-lines',requestPath,path]),b=wasm.resolve_pptx_lines(json,bytes);assert.equal(a,b,name);
 const r=JSON.parse(a);assert.equal(r.status==='error'?r.error.code:r.status,expected,name);
 const responsePath=`${root}/${name}.response.json`;fs.writeFileSync(responsePath,a);
 const item={name,status:expected,sourcePath:path,sourceSha256:sha(bytes),requestPath,requestSha256:sha(json),responsePath,responseSha256:sha(a)};
 cases.push(item);return {result:r,item};
}
const manifest=JSON.parse(fs.readFileSync(root+'/manifest.json'));
for(const [group,items] of [['style',manifest.cases],['declaration',JSON.parse(fs.readFileSync('.codex-work/source-lines/manifest.json')).cases]]){
 for(const c of items){
  const bytes=fs.readFileSync(c.path);assert.equal(sha(bytes),c.sha256);const inspected=JSON.parse(wasm.inspect_pptx(bytes));
  const id=inspected.status==='inspected'?inspected.index.surfaces['/ppt/slides/slide1.xml'].objects[0].nativeId:1;
  const request={expectedSourceSha256:c.sha256,surface:'/ppt/slides/slide1.xml',objects:[id],profile:'ms-oi29500-lines-2024-draft-v1'};
  const {result}=run(group+'-'+c.name,c.path,request,c.error??'evaluated');
  if(group!=='style')continue;
  const object=inspected.index.surfaces[request.surface].objects[0];
  const edit={expectedSourceSha256:c.sha256,edits:[{target:{part:request.surface,objectId:id,paragraph:0,run:0},expectedText:object.paragraphs[0][0].text,replacement:'Native line style retained 中文'}]};
  const editJson=JSON.stringify(edit),editPath=`${root}/${c.name}.edit.json`,pptxPath=`${root}/${c.name}.edited.pptx`;
  fs.writeFileSync(editPath,editJson);fs.rmSync(pptxPath,{force:true});native(['pptx-edit-text',editPath,c.path,pptxPath]);
  const candidate=fs.readFileSync(pptxPath);assert.deepEqual(candidate,Buffer.from(wasm.edit_pptx_text(editJson,bytes)));
  const {result:after,item}=run('edited-'+c.name,pptxPath,{...request,expectedSourceSha256:sha(candidate)},'evaluated');
  assert.deepEqual(after.styles.objects,result.styles.objects);
  item.editRequestPath=editPath;item.editRequestSha256=sha(editJson);item.originalSourcePath=c.path;item.originalSourceSha256=c.sha256;item.lineSemanticsPreserved=true;
 }
}
const c=manifest.cases[0],bytes=fs.readFileSync(c.path),idx=JSON.parse(wasm.inspect_pptx(bytes)).index;
const q={expectedSourceSha256:c.sha256,surface:'/ppt/slides/slide1.xml',objects:[c.object],profile:'ms-oi29500-lines-2024-draft-v1'};
for(const [name,mutate,status,validRequest] of [
 ['duplicate-object',q=>({...q,objects:[c.object,c.object]}),'evaluated',true],
 ['empty-query',q=>({...q,objects:[]}),'evaluated',true],
 ['all-object-kinds',q=>({...q,surface:'/ppt/slides/slide2.xml',objects:idx.surfaces['/ppt/slides/slide2.xml'].objects.map(o=>o.nativeId)}),'evaluated',true],
 ['wrong-source',q=>({...q,expectedSourceSha256:'0'.repeat(64)}),'SOURCE_CONFLICT',true],
 ['wrong-surface',q=>({...q,surface:'/missing.xml'}),'INPUT_INVALID',true],
 ['unknown-object',q=>({...q,objects:[c.object,4294967295]}),'INPUT_INVALID',true],
 ['query-budget',q=>({...q,objects:Array(257).fill(c.object)}),'LIMIT_EXCEEDED',true],
 ['unknown-profile',q=>({...q,profile:'unverified'}),'INPUT_INVALID',false],
 ['unknown-field',q=>({...q,execute:'unrecognized'}),'INPUT_INVALID',false],
 ['negative-object',q=>({...q,objects:[-1]}),'INPUT_INVALID',false],
 ['duplicate-key',q=>JSON.stringify(q).replace('"objects":','"objects":[],"objects":'),'INPUT_INVALID',false],
]){run('contract-'+name,c.path,mutate(q),status).item.validRequest=validRequest;}
const report={format:'musteroffice.line-style-parity/1',nativeCliSha256:sha(fs.readFileSync('target/release/mo-cli')),rustWasmSha256:sha(fs.readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),exactResponseAndEditCandidateBytes:true,cases};
fs.writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({batches:cases.length,editQueries:cases.filter(c=>c.lineSemanticsPreserved).length}));
