/** Real public runtimes: preserve V1 geometry, explicitly select V2 anchor focus. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
const root='.codex-work/radial-observation',out=root+'/runtime';fs.mkdirSync(out,{recursive:true});
const wasm=createRequire(import.meta.url)('../../.codex-work/radial-observation/wasm-node/mo_wasm.js');
const entry=path=>{const b=fs.readFileSync(path);return {path,byteLength:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const put=(path,b)=>{fs.writeFileSync(path,b);return entry(path);};
const cases=[];let unchanged=0,extendedDiagnostic=0;
const profile='drawingml-circle-anchor-focus-q96-v2-draft';
function run(name,source,input,status,expected,prior){
 const request=put(out+'/'+name+'.request.json',input);
 const n=spawnSync('target/debug/mo-cli',['layout-pptx-radial',request.path,source],{env:{},timeout:60000,encoding:'utf8',maxBuffer:32*1024*1024});assert.equal(n.status,0,n.stderr);
 const native=n.stdout.trimEnd(),actual=wasm.layout_pptx_radial(input,fs.readFileSync(source));assert.equal(actual,native,name);
 const result=JSON.parse(actual);assert.equal(result.status,status,name+actual);
 if(expected)assert.deepEqual(result.plans,expected);
 if(prior){
  const old=fs.readFileSync(prior.response.path,'utf8');
  if(name==='prior-invalid-profile'){
   assert.equal(result.error.code,'INPUT_INVALID');assert(result.error.message.includes(profile));assert(result.error.message.includes('drawingml-circle-path-bounds-q96-v1-draft'));extendedDiagnostic++;
  }else{assert.equal(actual,old,name);unchanged++;}
 }
 const record={name,source:entry(source),request,response:put(out+'/'+name+'.response.json',actual),status,plans:result.plans?.length??0};
 if(prior)record.priorResponse=prior.response;
 cases.push(record);
}
const previous='.codex-work/radial-layout/parity.json';
for(const c of JSON.parse(fs.readFileSync(previous)).cases)run('prior-'+c.name,c.source.path,fs.readFileSync(c.request.path,'utf8'),c.status,null,c);
const envelope=fills=>({profile,fills,options:{coordinateTolerance:'16777216'}});
for(const f of fs.readdirSync(root+'/native').filter(f=>f.endsWith('.pptx')).sort()){
 const name=f.slice(0,-5),base=root+'/native/'+name;
 run('anchor-'+name,base+'.pptx',JSON.stringify(envelope(JSON.parse(fs.readFileSync(base+'.query.json')))),'evaluated',JSON.parse(fs.readFileSync(base+'.plan.json')));
}
const sourceCases=JSON.parse(fs.readFileSync(root+'/sources.json')).cases;
const fillProfile=JSON.parse(fs.readFileSync(root+'/native/center-point.query.json')).profile;
for(const c of sourceCases){
 const fills={expectedSourceSha256:c.source.sha256,surface:'/ppt/slides/slide1.xml',targets:[{kind:'object',nativeId:42}],profile:fillProfile};
 run('observed-'+c.name,c.source.path,JSON.stringify(envelope(fills)),'evaluated');
}
assert.equal(cases.length,82);assert.equal(unchanged,41);assert.equal(extendedDiagnostic,1);
const report={format:'musteroffice.radial-anchor-parity/1',previous:entry(previous),pairedCalls:cases.length,priorUnchanged:unchanged,extendedProfileDiagnostics:extendedDiagnostic,cases,
 artifacts:['target/debug/mo-cli',root+'/wasm-node/mo_wasm.js',root+'/wasm-node/mo_wasm_bg.wasm'].map(entry)};
fs.writeFileSync(root+'/parity.json',JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify({paired:cases.length,priorUnchanged:unchanged,extendedDiagnostic,evaluated:cases.filter(c=>c.status==='evaluated').length,plans:cases.reduce((n,c)=>n+c.plans,0)}));
