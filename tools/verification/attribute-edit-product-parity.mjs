/** Current Native/WASM and previous WASM source inspection/text edit regression. */
import fs from 'node:fs';import assert from 'node:assert/strict';
import {createRequire} from 'node:module';import {spawnSync} from 'node:child_process';import {createHash} from 'node:crypto';
const root='.codex-work/attribute-edit',out=root+'/product';fs.mkdirSync(out,{recursive:true});
const require=createRequire(import.meta.url),before=require('../../.codex-work/elliptic-source/wasm-node/mo_wasm.js'),after=require('../../.codex-work/attribute-edit/wasm-node/mo_wasm.js');
const sha=b=>createHash('sha256').update(b).digest('hex');
const entry=p=>{const b=fs.readFileSync(p);return {path:p,byteLength:b.length,sha256:sha(b)};};
const put=(p,b)=>{fs.writeFileSync(p,b);return entry(p);};
const manifestPath='.codex-work/pptx-color-map-fixtures/manifest.json',manifest=JSON.parse(fs.readFileSync(manifestPath));
const cases=[];let first=null;
function compute(m,json,b){try{return {bytes:Buffer.from(m.edit_pptx_text(json,b))};}catch(e){return {error:String(e)};}}
function edit(name,json,source){
 const bytes=fs.readFileSync(source),old=compute(before,json,bytes),current=compute(after,json,bytes);assert.deepEqual(current,old,name);
 const prefix=out+'/'+name,q=put(prefix+'.request.json',json),output=prefix+'.pptx';assert(!fs.existsSync(output));
 const n=spawnSync('target/debug/mo-cli',['pptx-edit-text',q.path,source,output],{encoding:'utf8',env:{},timeout:60000,maxBuffer:80*1024*1024});
 const record={name,source:entry(source),request:q};
 if(current.bytes){assert.equal(n.status,0,n.stderr);assert.deepEqual(fs.readFileSync(output),current.bytes);assert.equal(JSON.parse(n.stdout).report.sha256,sha(current.bytes));record.output=entry(output);record.status='edited';}
 else{assert.notEqual(n.status,0);assert.equal(n.stderr.trim(),'Error: '+JSON.stringify(current.error));assert(!fs.existsSync(output));record.status='error';record.error=current.error;}
 cases.push(record);return record;
}
for(const c of manifest.cases){
 const b=fs.readFileSync(c.path);assert.equal(sha(b),c.sha256);
 const previous=before.inspect_pptx(b),current=after.inspect_pptx(b);assert.equal(current,previous,c.name);
 const n=spawnSync('target/debug/mo-cli',['pptx-inspect',c.path],{encoding:'utf8',env:{},timeout:60000,maxBuffer:80*1024*1024});
 assert.equal(n.status,0,n.stderr);assert.equal(n.stdout.trimEnd(),current,c.name);
 cases.push({name:'inspect-'+c.name,source:entry(c.path),response:put(out+'/inspect-'+c.name+'.json',current)});
 const r=JSON.parse(current);if(r.status!=='inspected')continue;
 let selected;
 for(const [part,surface] of Object.entries(r.index.surfaces))for(const object of surface.objects){
  if(!selected&&object.paragraphs[0]?.[0])selected={part,objectId:object.nativeId,paragraph:0,run:0,old:object.paragraphs[0][0].text};
 }
 if(!selected)continue;
 const {old,...target}=selected;
 const q={expectedSourceSha256:r.index.sourceSha256,edits:[{target,expectedText:old,replacement:'保留来源属性 🚀 & < >\r\n é'}]};
 const edited=edit('edit-'+c.name,JSON.stringify(q),c.path);
 if(!first&&edited.status==='edited')first={source:c.path,q,edited};
}
assert(first);
for(const [name,mutate] of [
 ['no-op',q=>{q.edits[0].replacement=q.edits[0].expectedText;}],['empty',q=>{q.edits=[];}],
 ['stale-source',q=>{q.expectedSourceSha256='0'.repeat(64);} ],
 ['stale-text',q=>{q.edits[0].expectedText='wrong';}],['duplicate',q=>{q.edits.push(q.edits[0]);}],
 ['invalid-character',q=>{q.edits[0].replacement='\0';}],['unknown-field',q=>{q.script='ignored';}],
 ['missing-target',q=>{q.edits[0].target.objectId=4294967295;}]
]){const q=structuredClone(first.q);mutate(q);const r=edit(name,JSON.stringify(q),first.source);if(name==='no-op'||name==='empty')assert.equal(r.output.sha256,r.source.sha256);}
const original=first.edited.output,n=spawnSync('target/debug/mo-cli',['pptx-edit-text',first.edited.request.path,first.source,original.path],{encoding:'utf8',env:{},maxBuffer:80*1024*1024});
assert.notEqual(n.status,0);assert.deepEqual(entry(original.path),original);
assert(!fs.readdirSync(out).some(n=>n.endsWith('.tmp')));
const report={format:'musteroffice.attribute-edit-product-parity/1',cases,comparedCalls:cases.length,
 inspectionCalls:cases.filter(c=>c.response).length,editCalls:cases.filter(c=>c.request).length,
 edited:cases.filter(c=>c.status==='edited').length,rejected:cases.filter(c=>c.status==='error').length,
 sourceManifest:entry(manifestPath),overwriteRefused:true,noTemporaryFiles:true,
 artifacts:[entry('target/debug/mo-cli'),entry('.codex-work/elliptic-source/wasm-node/mo_wasm_bg.wasm'),entry(root+'/wasm-node/mo_wasm_bg.wasm')]};
fs.writeFileSync(root+'/product.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({calls:report.comparedCalls,edited:report.edited,rejected:report.rejected}));
