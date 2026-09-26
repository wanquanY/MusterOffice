import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {createRequire} from 'node:module';
import {spawnSync} from 'node:child_process';
const root='.codex-work/fill-resolution', slide='/ppt/slides/slide1.xml';
const wasm=createRequire(import.meta.url)('../../.codex-work/wasm-node/mo_wasm.js');
const sha=b=>createHash('sha256').update(b).digest('hex'), read=p=>JSON.parse(fs.readFileSync(p));
const cases=[];
function native(args){const r=spawnSync('target/release/mo-cli',args,{encoding:'utf8',timeout:30000,maxBuffer:64*1024*1024});assert.equal(r.status,0,r.stderr);return r.stdout.trimEnd();}
function run(name,path,query,expected='evaluated',validRequest=true){
 const bytes=fs.readFileSync(path), raw=typeof query==='string'?query:JSON.stringify(query);
 const requestPath=`${root}/${name}.request.json`;fs.writeFileSync(requestPath,raw);
 const result=native(['pptx-fills',requestPath,path]); assert.equal(result,wasm.resolve_pptx_fills(raw,bytes),name);
 const response=JSON.parse(result); assert.equal(response.status==='error'?response.error.code:response.status,expected,name);
 const responsePath=`${root}/${name}.response.json`;fs.writeFileSync(responsePath,result);
 const record={name,sourcePath:path,sourceSha256:sha(bytes),requestPath,requestSha256:sha(raw),responsePath,responseSha256:sha(result),status:expected,validRequest};cases.push(record);
 return {response,record};
}
const owned=read(root+'/manifest.json').cases;
const legacy=read('.codex-work/source-fills/manifest.json').cases;
let sample;
for(const [group,inputs] of [['owned',owned],['declaration',legacy]]){
 for(const c of inputs){
  const bytes=fs.readFileSync(c.path);assert.equal(sha(bytes),c.sha256);
  const raw=wasm.inspect_pptx(bytes);assert.equal(raw,native(['pptx-inspect',c.path]));
  const inspection=JSON.parse(raw), objects=inspection.index?.surfaces[slide]?.objects;
  const shape=objects?.find(o=>o.nativeId===c.nativeId)??objects?.find(o=>o.kind==='shape');
  const pic=objects?.find(o=>o.kind==='picture');
  const targets=[{kind:'object',nativeId:shape?.nativeId??1},{kind:'line',nativeId:shape?.nativeId??1},{kind:'picture',nativeId:pic?.nativeId??1},{kind:'rootGroup'},{kind:'background'}];
  const query={expectedSourceSha256:c.sha256,surface:slide,targets,profile:'ms-oi29500-fills-2024-draft-v1'};
  const {response}=run(group+'-'+c.name,c.path,query,c.error??'evaluated');
  if(group!=='owned')continue;
  for(const [pointer,value] of Object.entries(c.expectations)){const result=pointer.slice(1).split('/').reduce((v,k)=>v?.[k],response);assert.deepEqual(result,value,c.name+pointer);}
  const inspectionPath=`${root}/${c.name}.inspection.json`;fs.writeFileSync(inspectionPath,raw);
  const record=cases.at(-1);record.inspectionPath=inspectionPath;record.inspectionSha256=sha(raw);
  const edit={expectedSourceSha256:c.sha256,edits:[{target:{part:slide,objectId:shape.nativeId,paragraph:0,run:0},expectedText:shape.paragraphs[0][0].text,replacement:'fill inheritance preserved 中文 & < >'}]};
  const editRaw=JSON.stringify(edit),editRequestPath=`${root}/${c.name}.edit.json`,pptxPath=`${root}/${c.name}.edited.pptx`;
  fs.writeFileSync(editRequestPath,editRaw);fs.rmSync(pptxPath,{force:true});
  const metadata=JSON.parse(native(['pptx-edit-text',editRequestPath,c.path,pptxPath]));
  const candidate=fs.readFileSync(pptxPath);assert.deepEqual(candidate,Buffer.from(wasm.edit_pptx_text(editRaw,bytes)));assert.equal(metadata.report.sha256,sha(candidate));
  const {response:after,record:edited}=run('edited-'+c.name,pptxPath,{...query,expectedSourceSha256:sha(candidate)});
  assert.deepEqual(after.styles.targets,response.styles.targets,c.name);
  Object.assign(edited,{editRequestPath,editRequestSha256:sha(editRaw),pptxPath,pptxSha256:sha(candidate),fillStylesPreserved:true});
  sample??={c,query,inspection};
 }
}
const {c,query,inspection}=sample;
for(const [name,mutate,expected,valid] of [
 ['empty',q=>({...q,targets:[]}),'evaluated',true],
 ['duplicates',q=>({...q,targets:[q.targets[0],q.targets[0]]}),'evaluated',true],
 ['all-object-targets',q=>({...q,targets:inspection.index.surfaces[slide].objects.flatMap(o=>['object','line','picture'].map(kind=>({kind,nativeId:o.nativeId})))}),'evaluated',true],
 ['wrong-digest',q=>({...q,expectedSourceSha256:'0'.repeat(64)}),'SOURCE_CONFLICT',true],
 ['missing-surface',q=>({...q,surface:'/missing.xml'}),'INPUT_INVALID',true],
 ['missing-object',q=>({...q,targets:[{kind:'object',nativeId:4294967295}]}),'INPUT_INVALID',true],
 ['too-many',q=>({...q,targets:Array.from({length:257},()=>q.targets[0])}),'LIMIT_EXCEEDED',true],
 ['unknown-profile',q=>({...q,profile:'default'}),'INPUT_INVALID',false],
 ['unknown-target',q=>({...q,targets:[{kind:'script'}]}),'INPUT_INVALID',false],
 ['extra-key',q=>({...q,arbitraryScript:'x'}),'INPUT_INVALID',false],
 ['duplicate-key',q=>JSON.stringify(q).replace('"surface":','"surface":"/other.xml","surface":') ,'INPUT_INVALID',false],
 ['invalid-json',()=>'{','INPUT_INVALID',false],
])run('contract-'+name,c.path,mutate(query),expected,valid);
const report={format:'musteroffice.fill-style-parity/1',nativeCliSha256:sha(fs.readFileSync('target/release/mo-cli')),rustWasmSha256:sha(fs.readFileSync('.codex-work/wasm-node/mo_wasm_bg.wasm')),cases,exactResponseAndCandidateBytes:true};
fs.writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({batches:cases.length,edited:cases.filter(c=>c.fillStylesPreserved).length}));
