/** Actual public Native/WASM editing, strict requests and previous API regression. */
import fs from 'node:fs';import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';import {createRequire} from 'node:module';import {createHash} from 'node:crypto';
const root='.codex-work/transform-edit',out=root+'/product';fs.mkdirSync(out,{recursive:true});
const require=createRequire(import.meta.url),wasm=require('../../.codex-work/transform-edit/wasm-node/mo_wasm.js');
const sha=b=>createHash('sha256').update(b).digest('hex');
const entry=p=>{const b=fs.readFileSync(p);return {path:p,byteLength:b.length,sha256:sha(b)};};
const put=(p,b)=>{fs.writeFileSync(p,b);return entry(p);};
const load=r=>{assert.deepEqual(entry(r.path),r);return fs.readFileSync(r.path);};
const command=args=>spawnSync('target/debug/mo-cli',args,{env:{},encoding:'utf8',timeout:60000,maxBuffer:80*1024*1024});
const fixturePath=root+'/fixtures.json',fixtures=JSON.parse(fs.readFileSync(fixturePath)),cases=[];
const fields=['origin','size','childOrigin','childSize','rotation','flipHorizontal','flipVertical'];
const values=t=>Object.fromEntries(fields.map(k=>[k,t?.[k]??null]));
function targets(f){const r=JSON.parse(wasm.inspect_pptx(load(f.source)));assert.equal(r.status,'inspected');const result=[];
 for(const [part,s] of Object.entries(r.index.surfaces)){
  if(s.rootGroupTransform)result.push({name:'root/'+part,target:{part,nativeId:s.rootObjectId},transform:s.rootGroupTransform});
  for(const o of s.objects)result.push({name:o.name,target:{part,nativeId:o.nativeId},transform:o.transform,kind:o.kind});
 }return result;
}
const fixture=name=>{const f=fixtures.cases.find(c=>c.name===name);assert(f);return f;};
function request(f,names){const all=targets(f);return {expectedSourceSha256:f.source.sha256,edits:names.map(name=>{
 const t=all.find(t=>t.name===name);assert(t,name);const expected=values(t.transform);
 return {target:t.target,expected,replacement:structuredClone(expected)};
})};}
function combined(e){
 e.replacement.rotation=-8100000;e.replacement.flipHorizontal=true;e.replacement.flipVertical=false;
 if(e.replacement.origin)e.replacement.origin={x:'-12700',y:'25400'};
 if(e.replacement.size)e.replacement.size={width:'3810000',height:'1270000'};
 if(e.replacement.childOrigin)e.replacement.childOrigin={x:'127',y:'-254'};
 if(e.replacement.childSize)e.replacement.childSize={width:'635000',height:'254000'};
}
function execute(name,f,q,expected='edited'){
 const json=typeof q==='string'?q:JSON.stringify(q),source=load(f.source),prefix=out+'/'+name;
 const requestFile=put(prefix+'.request.json',json),path=prefix+'.pptx';assert(!fs.existsSync(path));
 let bytes,error;try{bytes=Buffer.from(wasm.edit_pptx_transforms(json,source));}catch(e){error=String(e);}
 const n=command(['pptx-edit-transforms',requestFile.path,f.source.path,path]);
 const record={name,source:f.source,request:requestFile,rawXsd:f.rawXsd};
 if(bytes){assert.equal(expected,'edited',name);assert.equal(n.status,0,n.stderr);assert.deepEqual(fs.readFileSync(path),bytes,name);
  assert.equal(JSON.parse(n.stdout).report.sha256,sha(bytes));record.status='edited';record.output=entry(path);
  const w=wasm.inspect_pptx(bytes),i=command(['pptx-inspect',path]);assert.equal(i.status,0);assert.equal(i.stdout.trimEnd(),w);
  record.inspection=put(prefix+'.inspection.json',w);
 }else{assert.notEqual(expected,'edited',name+': '+error);assert.notEqual(n.status,0);assert.equal(n.stderr.trim(),'Error: '+JSON.stringify(error));
  assert(error.startsWith(expected+':'),name+': '+error);assert(!fs.existsSync(path));record.status='error';record.error=error;
 }
 cases.push(record);return record;
}
const base=fixture('base');
for(const t of targets(base).filter(t=>t.transform)){
 for(const mode of ['noop','all','clear']){
  const q=request(base,[t.name]);if(mode==='all')combined(q.edits[0]);
  if(mode==='clear')for(const k of ['rotation','flipHorizontal','flipVertical'])q.edits[0].replacement[k]=null;
  const r=execute(t.name.replaceAll(':','-')+'-'+mode,base,q);if(mode==='noop')assert.equal(r.output.sha256,r.source.sha256);
 }
}
for(const f of fixtures.cases){
 if(['base','missing-transform','missing-origin','unknown-attribute','ignored-attribute','ignored-child','alternate-object'].includes(f.name))continue;
 let names=['title:1'];if(f.name==='root')names=['root//ppt/slides/slide1.xml','group:2'];
 if(f.name==='inherited')names=['rule:layout'];if(f.name==='frame-connector')names=['owned-table','owned-connector'];
 const q=request(f,names);q.edits.forEach(combined);execute(f.name,f,q);
}
for(const name of ['unknown-attribute','ignored-attribute','ignored-child','alternate-object']){
 const f=fixture(name),q=request(f,['title:1']);const n=execute(name+'-noop',f,q);assert.equal(n.output.sha256,n.source.sha256);
 combined(q.edits[0]);execute(name,f,q,'MAPPING_NOT_IMPLEMENTED');
 if(name==='alternate-object'){const other=request(f,['unicode:1']);combined(other.edits[0]);execute(name+'-outside',f,other);}
}
for(const name of ['missing-transform','missing-origin']){
 const f=fixture(name),q=request(f,['title:1']);combined(q.edits[0]);
 if(name==='missing-origin')q.edits[0].replacement.origin={x:'1',y:'2'};
 execute(name,f,q,'MAPPING_NOT_IMPLEMENTED');
}
const q=request(base,['title:1','title:2','group:2','child:2a','rule:layout']);q.edits.forEach(combined);
const multi=execute('multipart',base,q);q.edits.reverse();const reversed=execute('multipart-reversed',base,q);assert.equal(multi.output.sha256,reversed.output.sha256);
for(const [name,mutate,code] of [
 ['stale-source',q=>q.expectedSourceSha256='0'.repeat(64),'SOURCE_CONFLICT'],
 ['stale-value',q=>q.edits[0].expected.rotation=700,'SOURCE_CONFLICT'],
 ['duplicate',q=>q.edits.push(structuredClone(q.edits[0])),'SOURCE_CONFLICT'],
 ['missing-object',q=>q.edits[0].target.nativeId=4294967295,'SOURCE_CONFLICT'],
 ['missing-part',q=>q.edits[0].target.part='/ppt/slides/absent.xml','SOURCE_CONFLICT'],
 ['negative-size',q=>q.edits[0].replacement.size.width='-1','SOURCE_CONFLICT'],
 ['high-coordinate',q=>q.edits[0].replacement.origin.x='27273042316901','SOURCE_CONFLICT'],
 ['low-coordinate',q=>q.edits[0].replacement.origin.x='-27273042329601','SOURCE_CONFLICT'],
 ['missing-leaf',q=>q.edits[0].replacement.origin=null,'MAPPING_NOT_IMPLEMENTED'],
 ['new-child',q=>q.edits[0].replacement.childOrigin={x:'0',y:'0'},'MAPPING_NOT_IMPLEMENTED'],
 ['number-emu',q=>q.edits[0].replacement.origin.x=12700,'INPUT_INVALID'],
 ['noncanonical-emu',q=>q.edits[0].replacement.origin.x='+1','INPUT_INVALID'],
 ['float-angle',q=>q.edits[0].replacement.rotation=0.5,'INPUT_INVALID'],
 ['angle-overflow',q=>q.edits[0].replacement.rotation=2147483648,'INPUT_INVALID'],
 ['physical-ordinal',q=>q.edits[0].target.ordinal=1,'INPUT_INVALID'],
 ['unknown-field',q=>q.script='ignored','INPUT_INVALID'],
 ['string-bool',q=>q.edits[0].replacement.flipHorizontal='true','INPUT_INVALID'],
 ['atomic-second',q=>{const e=request(base,['title:2']).edits[0];e.expected.rotation=99;q.edits.push(e);},'SOURCE_CONFLICT']
]){const q=request(base,['title:1']);combined(q.edits[0]);mutate(q);execute(name,base,q,code);}
for(const [name,rotation,x,size] of [['low-limit',-2147483648,'-27273042329600','0'],['high-limit',2147483647,'27273042316900','27273042316900']]){
 const q=request(base,['title:1']);q.edits[0].replacement.rotation=rotation;q.edits[0].replacement.origin.x=x;q.edits[0].replacement.size.width=size;execute(name,base,q);
}
const empty=execute('empty',base,{expectedSourceSha256:base.source.sha256,edits:[]});assert.equal(empty.output.sha256,base.source.sha256);
execute('duplicate-json',base,'{"expectedSourceSha256":"'+base.source.sha256+'","edits":[],"edits":[]}','INPUT_INVALID');
const overwritten=command(['pptx-edit-transforms',multi.request.path,multi.source.path,multi.output.path]);assert.notEqual(overwritten.status,0);assert.deepEqual(entry(multi.output.path),multi.output);
assert(!fs.readdirSync(out).some(n=>n.endsWith('.tmp')));

// Frozen earlier public inspection and text-edit results must remain identical.
const oldPath='.codex-work/attribute-edit/product.json',old=JSON.parse(fs.readFileSync(oldPath));const regression=[];
for(const c of old.cases){
 const b=load(c.source);let response;
 if(c.response){response=wasm.inspect_pptx(b);assert.equal(response,load(c.response).toString());const n=command(['pptx-inspect',c.source.path]);assert.equal(n.status,0);assert.equal(n.stdout.trimEnd(),response);}
 else{
  const json=load(c.request).toString();let bytes,error;try{bytes=Buffer.from(wasm.edit_pptx_text(json,b));}catch(e){error=String(e);}
  const path=out+'/old-'+c.name+'.pptx',n=command(['pptx-edit-text',c.request.path,c.source.path,path]);assert(!fs.existsSync(path)||c.output);
  if(c.output){assert(bytes);assert.deepEqual(bytes,load(c.output));assert.equal(n.status,0,n.stderr);assert.deepEqual(fs.readFileSync(path),bytes);response=entry(path);}
  else{assert.equal(error,c.error);assert.notEqual(n.status,0);assert.equal(n.stderr.trim(),'Error: '+JSON.stringify(error));assert(!fs.existsSync(path));}
 }
 regression.push({name:c.name,source:c.source,...(c.response?{response:c.response}:c.output?{output:response}:{error:c.error})});
}
const report={format:'musteroffice.transform-edit-parity/1',cases,edited:cases.filter(c=>c.status==='edited').length,
 rejected:cases.filter(c=>c.status==='error').length,regressionCalls:regression.length,regression,fixtures:entry(fixturePath),previous:entry(oldPath),
 overwriteRefused:true,noTemporaryFiles:true,currentArtifacts:[entry('target/debug/mo-cli'),entry(root+'/wasm-node/mo_wasm_bg.wasm')]};
fs.writeFileSync(root+'/product.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({cases:cases.length,edited:report.edited,rejected:report.rejected,regression:regression.length}));
